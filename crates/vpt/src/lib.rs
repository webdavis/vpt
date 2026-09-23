mod cli;
mod commands;
mod compose;

use cli::args::{Invocation, Verb, parse};
use cli::output::{Outcome, emit};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub fn run() -> ! {
    let args: Vec<_> = std::env::args_os().collect();
    let json = args.iter().skip(1).any(|word| word == "--json");
    let code = match parse(args) {
        Ok(invocation) => dispatch(&invocation),
        Err(error) => emit(
            Outcome::ParseFailure(ErrorDocument::new(ErrorKind::Usage, error.0)),
            json,
        ),
    };
    std::process::exit(code);
}

fn dispatch(invocation: &Invocation) -> i32 {
    let outcome = match &invocation.verb {
        Verb::Version => Outcome::Success {
            document: commands::version::document(None),
            human: commands::version::human(None),
        },
        Verb::Setup { force } => commands::setup::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            *force,
        ),
        _ => Outcome::Failure(ErrorDocument::new(
            ErrorKind::Usage,
            "verb not implemented yet",
        )),
    };
    emit(outcome, invocation.json)
}
