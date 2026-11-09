use crate::{
    cli::{
        args::{Invocation, Verb},
        output::{Outcome, emit},
    },
    commands, compose, doctor,
};

pub(crate) fn run(invocation: &Invocation) -> i32 {
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
    };
    emit(outcome, invocation.json)
}
