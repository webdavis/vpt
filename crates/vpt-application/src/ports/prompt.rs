#[derive(Debug, PartialEq, Eq)]
pub enum PromptError {
    NoTerminal,
    Closed,
}

pub trait Prompt {
    fn choose(
        &mut self,
        question: &str,
        options: &[&str],
        preselected: usize,
    ) -> Result<usize, PromptError>;
}
