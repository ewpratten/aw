use super::Backend;

pub struct OpenCode {
    pub binary: String,
}

impl Backend for OpenCode {
    fn is_installed(&self) -> bool {
        super::responds_to_version("opencode")
    }

    fn extend_env(&self, session_env: &mut Vec<String>) {
        // Teach OpenCode how to talk to cmux
        if session_env.iter().any(|var| var.starts_with("CMUX_")) {
            session_env.push("OPENCODE_CMUX_TRANSPORT=cli".to_string());
        }
    }

    fn command(&self, _session: &str, prompt: Option<&str>) -> String {
        let binary = super::shell_quote(&self.binary);
        match prompt {
            Some(prompt) => format!("{binary} --auto --prompt {}", super::shell_quote(prompt)),
            None => format!("{binary} -c --auto || {binary} --auto"),
        }
    }
}
