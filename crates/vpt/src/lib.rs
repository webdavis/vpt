mod cli;
mod commands;
mod compose;
mod dispatch;
mod doctor;
mod documents;

use cli::args::parse;
use cli::output::{Outcome, emit};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub fn run() -> ! {
    vpt_adapters::install_interrupt_handlers();
    let args: Vec<_> = std::env::args_os().collect();
    let json = args.iter().skip(1).any(|word| word == "--json");
    let code = match parse(args) {
        Ok(invocation) => dispatch::run(&invocation),
        Err(error) => emit(
            Outcome::ParseFailure(ErrorDocument::new(ErrorKind::Usage, error.0)),
            json,
        ),
    };
    std::process::exit(code);
}
