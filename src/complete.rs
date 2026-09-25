use crate::config::Config;
use crate::projects;
use clap_complete::CompletionCandidate;
use std::process::Command;

/// Load the config the same way the main program does
fn load_config() -> (Config, directories::ProjectDirs) {
    let project_dirs = directories::ProjectDirs::from("", "", "aw").expect("could not determine home directory");
    let config = Config::load(&project_dirs.config_dir().join("config.toml"));
    (config, project_dirs)
}

/// Candidates for the `project` argument
pub fn project_candidates() -> Vec<CompletionCandidate> {
    let (config, _) = load_config();
    projects::completion_names(&config).into_iter().map(CompletionCandidate::new).collect()
}

/// Candidates for the `context_id` argument: existing workspaces and running sessions
pub fn context_candidates() -> Vec<CompletionCandidate> {
    // The shell invokes us as `aw -- aw <project> <context>`, so dig the project out of that
    let Some(input) = std::env::args()
        .skip_while(|arg| arg != "--")
        .skip(2)
        .find(|arg| !arg.starts_with('-'))
    else {
        return Vec::new();
    };
    let (config, project_dirs) = load_config();
    let Ok((rel, _)) = projects::resolve(&config, &input) else { return Vec::new() };
    let mut ids = Vec::new();

    // Contexts that already have a disposable jj workspace on disk
    if let Ok(entries) = std::fs::read_dir(project_dirs.cache_dir().join(rel.replace('/', "-"))) {
        for entry in entries.flatten().filter(|entry| entry.path().is_dir()) {
            ids.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    // Contexts backing a currently running tmux session
    let prefix = format!("{}-", projects::session_name(&rel, None));
    if let Ok(output) = Command::new("tmux").args(["list-sessions", "-F", "#{session_name}"]).output() {
        for session in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(id) = session.strip_prefix(&prefix) {
                ids.push(id.to_string());
            }
        }
    }

    ids.sort();
    ids.dedup();
    ids.into_iter().map(CompletionCandidate::new).collect()
}
