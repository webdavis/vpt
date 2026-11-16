mod cli;
mod commands;
mod compose;
mod dispatch;
mod doctor;
mod documents;
mod report;
mod self_test;
mod style;
#[cfg(test)]
mod test_temp;

use cli::args::parse;
use cli::output::{Outcome, emit};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub fn run() -> ! {
    vpt_adapters::install_interrupt_handlers();
    let args: Vec<_> = std::env::args_os().collect();
    let format = cli::format::named(&args);
    let code = match parse(args) {
        Ok(invocation) => {
            if invocation.verb.reads_ledger_only() {
                compose::ledger_identity::warn(invocation.config.as_deref());
            }
            dispatch::run(&invocation)
        }
        Err(error) => emit(
            Outcome::ParseFailure(
                ErrorDocument::new(ErrorKind::Usage, error.message).diagnostics(error.tips),
            ),
            format,
        ),
    };
    std::process::exit(code);
}
