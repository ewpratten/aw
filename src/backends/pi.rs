use super::Backend;

pub struct Pi {
    pub binary: String,
}

impl Backend for Pi {
    fn is_installed(&self) -> bool {
        super::responds_to_version("pi")
    }

    fn command(&self, session: &str, prompt: Option<&str>) -> String {
        // Name the pi session after the aw session, and trust project-local files like we do for Claude
        let flags = format!("--approve --name '{}'", session);
        let binary = super::shell_quote(&self.binary);
        match prompt {
            Some(prompt) => format!("{binary} {flags} -- {}", super::shell_quote(prompt)),
            None => format!("{binary} -c {flags} || {binary} {flags}"),
        }
    }
}
