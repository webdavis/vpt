use super::{Verb, root::Cli};
use clap::CommandFactory;

#[derive(clap::Args)]
pub(super) struct Completion {
    shell: clap_complete::Shell,
}

impl Completion {
    pub(super) fn verb(self) -> Verb {
        let mut script = Vec::new();
        clap_complete::generate(self.shell, &mut Cli::command(), "vpt", &mut script);
        Verb::Print(String::from_utf8_lossy(&script).into_owned())
    }
}
