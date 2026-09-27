mod cli;
mod commands;
mod compose;
mod doctor;
mod documents;

use cli::args::{Invocation, Verb, parse};
use cli::output::{Outcome, emit};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub fn run() -> ! {
    vpt_adapters::install_interrupt_handlers();
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
        Verb::Doctor => doctor::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ),
        Verb::Version => commands::version::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ),
        Verb::Setup { force } => commands::setup::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            *force,
        ),
        Verb::SymlinkDeploy => commands::symlink::deploy(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ),
        Verb::SymlinkVerify => commands::symlink::verify(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ),
        Verb::Show { id } => match compose::ledger_from(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ) {
            Ok(ledger) => commands::show::run(&ledger, id),
            Err(error) => Outcome::Failure(*error),
        },
        Verb::List { stage } => match compose::ledger_from(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ) {
            Ok(ledger) => commands::list::run(&ledger, stage.as_deref()),
            Err(error) => Outcome::Failure(*error),
        },
        Verb::Storage => match compose::storage_from(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ) {
            Ok((paths, stores)) => commands::storage::run(&paths, &stores),
            Err(error) => Outcome::Failure(*error),
        },
        _ => Outcome::Failure(ErrorDocument::new(
            ErrorKind::Usage,
            "verb not implemented yet",
        )),
    };
    emit(outcome, invocation.json)
}
