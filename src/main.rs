pub mod config;

use clap::{Parser, Subcommand};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

#[derive(Parser)]
#[command(args_conflicts_with_subcommands = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Project name. Bare `repo`, or `org/repo` format accepted. Current directory if omitted
    project: Option<String>,

    /// Spin up a disposable jj workspace for this context
    context_id: Option<String>,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
}

#[derive(Subcommand)]
enum Command {
    /// Kill detached aw sessions that have been idle for a while
    Gc,
}

/// Log an error and bail out
fn fail(message: impl std::fmt::Display) -> ! {
    log::error!("{}", message);
    std::process::exit(1);
}

/// Run a command, returning whether it exited successfully
fn run(command: &mut Process) -> bool {
    command.status().map(|status| status.success()).unwrap_or(false)
}

/// Walk up from `dir` looking for a jj repo root
fn find_jj_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|dir| dir.join(".jj").is_dir())
        .map(Path::to_path_buf)
}

fn main() {
    let args = Args::parse();

    // Set up logging
    fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("{}: {}", record.level(), message)))
        .level(args.verbose.log_level_filter())
        .chain(std::io::stderr())
        .apply()
        .unwrap();

    // Load the config, and figure out where to keep disposable workspaces
    let project_dirs = directories::ProjectDirs::from("", "", "aw").expect("could not determine home directory");
    let config = config::Config::load(&project_dirs.config_dir().join("config.toml"));
    let cache_dir = project_dirs.cache_dir();
    log::debug!("Loaded config: {:?}", config);

    // Everything below needs tmux
    if !run(Process::new("tmux").arg("-V").stdout(std::process::Stdio::null())) {
        fail("tmux is not installed");
    }

    // Prune idle, detached sessions
    if matches!(args.command, Some(Command::Gc)) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let output = Process::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}\t#{session_attached}\t#{session_activity}"])
            .output()
            .unwrap();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let [session, attached, activity] = line.split('\t').collect::<Vec<_>>()[..] else { continue };
            if !session.starts_with("aw-") || attached != "0" {
                continue;
            }

            let idle = now.saturating_sub(activity.parse().unwrap_or(now));
            if idle > config.max_session_idle_time {
                log::info!("Pruning {} (idle {}h)", session, idle / 3600);
                run(Process::new("tmux").args(["kill-session", "-t", session]));
            }
        }
        return;
    }

    // With no args, infer the project from the current directory if we're
    // inside one of the extra repos, or a repo under one of the project dirs
    let input = args.project.clone().unwrap_or_else(|| {
        find_jj_root(&std::env::current_dir().unwrap())
            .and_then(|root| match config.extra_repos.contains(&root) {
                true => Some(root.file_name().unwrap().to_string_lossy().to_string()),
                false => config
                    .project_dirs
                    .iter()
                    .find_map(|dir| root.strip_prefix(dir).ok())
                    .map(|rel| rel.to_string_lossy().to_string()),
            })
            .unwrap_or_else(|| fail("No project name given, and the current directory is not inside a known project"))
    });

    // Resolve the project name into a directory
    let (rel, project_dir) = if input.contains('/') {
        config
            .project_dirs
            .iter()
            .map(|dir| dir.join(&input))
            .find(|path| path.is_dir())
            .map(|path| (input.clone(), path))
            .unwrap_or_else(|| fail(format!("No such project: {}", input)))
    } else {
        let mut matches: Vec<(String, PathBuf)> = Vec::new();

        // Hard-coded repos, matched by directory name
        for repo in &config.extra_repos {
            if repo.file_name().is_some_and(|name| name == input.as_str()) {
                matches.push((input.clone(), repo.clone()));
            }
        }

        // Repos directly in the project dir, then org/repo one directory deeper
        for dir in &config.project_dirs {
            if dir.join(&input).join(".jj").is_dir() {
                matches.push((input.clone(), dir.join(&input)));
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
                entries.sort();
                for org in entries {
                    if org.join(&input).join(".jj").is_dir() {
                        matches.push((format!("{}/{}", org.file_name().unwrap().to_string_lossy(), input), org.join(&input)));
                    }
                }
            }
        }

        match matches.len() {
            0 => fail(format!(
                "No jj repo named '{}' found in extra repos or under {} (searched top-level and one level deep)",
                input,
                config.project_dirs.iter().map(|dir| dir.display().to_string()).collect::<Vec<_>>().join(", ")
            )),
            _ => {
                log::warn!("Multiple directories matched '{}'. Using the first match.", input);
                matches.remove(0)
            },
        }
    };
    if !project_dir.join(".jj").is_dir() {
        fail(format!("Not a jj repo: {}", project_dir.display()));
    }

    // A context id gets its own disposable jj workspace, so multiple contexts
    // for the same project can run side by side without stepping on each other
    let rel_slug = rel.replace('/', "-");
    let mut session = format!("aw-{}", rel_slug).replace('.', "-");
    let mut work_dir = project_dir.clone();
    if let Some(context_id) = &args.context_id {
        let context_id = context_id.replace(['/', '.'], "-");
        session = format!("{}-{}", session, context_id);
        work_dir = cache_dir.join(&rel_slug).join(&context_id);

        if !work_dir.is_dir() {
            std::fs::create_dir_all(work_dir.parent().unwrap()).unwrap();
            let steps: [&[&str]; 3] = [
                &["-R", project_dir.to_str().unwrap(), "workspace", "add", work_dir.to_str().unwrap(), "--name", &context_id],
                &["-R", work_dir.to_str().unwrap(), "git", "fetch"],
                &["-R", work_dir.to_str().unwrap(), "rebase", "-d", "trunk()"],
            ];
            for step in steps {
                if !run(Process::new("jj").args(step)) {
                    fail(format!("jj {} failed", step.join(" ")));
                }
            }
        }
    }

    // Pre-accept Claude's trust-this-folder dialog for the work dir
    let claude_config_path = directories::BaseDirs::new().unwrap().home_dir().join(".claude.json");
    if let Some(mut claude_config) = std::fs::read_to_string(&claude_config_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    {
        claude_config["projects"][work_dir.to_string_lossy().as_ref()]["hasTrustDialogAccepted"] = true.into();
        let temp_path = claude_config_path.with_extension("json.aw-tmp");
        std::fs::write(&temp_path, serde_json::to_string_pretty(&claude_config).unwrap()).unwrap();
        std::fs::rename(&temp_path, &claude_config_path).unwrap();
    }

    // Prefer claude code, fall back to opencode if it isn't installed.
    let quiet = |name: &str, arg: &str| {
        run(Process::new(name).arg(arg).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()))
    };
    let agent_command = if quiet("claude", "--version") {
        "claude -c --permission-mode auto || claude --permission-mode auto"
    } else if quiet("opencode", "--version") {
        "opencode -c --auto || opencode --auto"
    } else {
        fail("Neither claude nor opencode is installed");
    };

    // Spawn the session if it doesn't already exist
    if !run(Process::new("tmux").args(["has-session", "-t", &session]).stderr(std::process::Stdio::null())) {
        // Pass through cmux env vars so that the harness inside of tmux can communicate with cmux
        let mut session_env: Vec<String> = std::env::vars()
            .filter(|(key, _)| key.starts_with("CMUX_"))
            .map(|(key, value)| format!("{}={}", key, value))
            .collect();

        // Teach OpenCode how to talk to cmux
        if !session_env.is_empty() {
            session_env.push("OPENCODE_CMUX_TRANSPORT=cli".to_string());
        }

        // Launch the tmux session
        let mut new_session = Process::new("tmux");
        new_session.args(["new-session", "-d", "-s", &session, "-c"]).arg(&work_dir);
        for var in &session_env {
            new_session.args(["-e", var]);
        }
        if !run(new_session.arg(agent_command)) {
            fail(format!("Failed to create tmux session {}", session));
        }

        // Focus the pane running the agent
        run(Process::new("tmux").args(["select-pane", "-t", &format!("{}:", session)]));
    }

    // Various QoL settings for tmux
    let options: [&[&str]; 4] = [
        &["status", "off"],
        &["-w", "window-size", "smallest"],
        &["set-titles", "on"],
        &["set-titles-string", "#{?#{==:#{pane_title},#{host}},#{session_name},#{pane_title}}"],
    ];
    for option in options {
        run(Process::new("tmux").args(["set-option", "-t", &session]).args(option));
    }

    // Attach (or switch, if already inside tmux)
    let error = match std::env::var_os("TMUX") {
        Some(_) => Process::new("tmux").args(["switch-client", "-t", &session]).exec(),
        None => Process::new("tmux").args(["attach-session", "-t", &session]).exec(),
    };
    fail(format!("Failed to attach to tmux: {}", error));
}
