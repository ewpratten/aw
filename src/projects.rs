use crate::config::Config;
use std::collections::HashMap;
use std::path::PathBuf;

/// Every jj repo `aw` knows about, as `(rel, path)` pairs in search priority order.
/// `rel` is either the bare repo name or `org/repo` for repos one level deep
pub fn discover(config: &Config) -> Vec<(String, PathBuf)> {
    let mut repos = Vec::new();

    // Hard-coded repos, named by their directory
    for repo in &config.extra_repos {
        if let Some(name) = repo.file_name() {
            repos.push((name.to_string_lossy().to_string(), repo.clone()));
        }
    }

    // Repos directly in each project dir, then org/repo one directory deeper
    for dir in &config.project_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
        entries.sort();
        for top in &entries {
            if top.join(".jj").is_dir() {
                repos.push((top.file_name().unwrap().to_string_lossy().to_string(), top.clone()));
            }
        }
        for org in &entries {
            let Ok(children) = std::fs::read_dir(org) else { continue };
            let mut children: Vec<_> = children.flatten().map(|entry| entry.path()).filter(|path| path.join(".jj").is_dir()).collect();
            children.sort();
            for repo in children {
                let rel = format!("{}/{}", org.file_name().unwrap().to_string_lossy(), repo.file_name().unwrap().to_string_lossy());
                repos.push((rel, repo));
            }
        }
    }
    repos
}

/// Resolve a user-supplied project name into its `(rel, path)`
pub fn resolve(config: &Config, input: &str) -> Result<(String, PathBuf), String> {
    // Qualified names map straight onto a project dir
    if input.contains('/') {
        return config
            .project_dirs
            .iter()
            .map(|dir| dir.join(input))
            .find(|path| path.is_dir())
            .map(|path| (input.to_string(), path))
            .ok_or_else(|| format!("No such project: {}", input));
    }

    // Bare names match the last path component of anything we know about
    let mut matches: Vec<_> = discover(config)
        .into_iter()
        .filter(|(rel, _)| rel.rsplit('/').next() == Some(input))
        .collect();
    match matches.len() {
        0 => Err(format!(
            "No jj repo named '{}' found in extra repos or under {} (searched top-level and one level deep)",
            input,
            config.project_dirs.iter().map(|dir| dir.display().to_string()).collect::<Vec<_>>().join(", ")
        )),
        1 => Ok(matches.remove(0)),
        _ => {
            log::warn!("Multiple directories matched '{}'. Using the first match.", input);
            Ok(matches.remove(0))
        }
    }
}

/// Names to offer when tab-completing a project. Bare names are used where they are
/// unambiguous, falling back to the qualified `org/repo` form when contested
pub fn completion_names(config: &Config) -> Vec<String> {
    let repos = discover(config);

    // Count how many repos share each bare name
    let mut name_count: HashMap<&str, usize> = HashMap::new();
    for (rel, _) in &repos {
        *name_count.entry(rel.rsplit('/').next().unwrap()).or_default() += 1;
    }

    let mut names = Vec::new();
    for (rel, _) in &repos {
        let name = rel.rsplit('/').next().unwrap();
        let candidate = match name_count[name] > 1 && rel.contains('/') {
            true => rel.clone(),
            false => name.to_string(),
        };
        if !names.contains(&candidate) {
            names.push(candidate);
        }
    }
    names
}

/// The tmux session name for a project (and optional context)
pub fn session_name(rel: &str, context_id: Option<&str>) -> String {
    let session = format!("aw-{}", rel.replace('/', "-")).replace('.', "-");
    match context_id {
        Some(context_id) => format!("{}-{}", session, context_id),
        None => session,
    }
}
