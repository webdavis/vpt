use crate::{
    cli::{
        args::{Invocation, Verb},
        output::{Outcome, emit},
    },
    commands, compose, doctor,
};

pub(crate) fn run(invocation: &Invocation) -> i32 {
    let outcome = match &invocation.verb {
        Verb::Brief(options) => commands::brief::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            options,
        ),
        Verb::Synthesize { id, dry_run } => commands::synthesize::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
            *dry_run,
        ),
        Verb::VerifyNote { path, recording } => commands::verify::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            path,
            recording,
        ),
        Verb::ReviewResolve { id, flag, action } => commands::review::resolve(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
            flag,
            action,
        ),
        Verb::ConfirmTag { id, tag, action } => commands::confirm::run_tag(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
            tag.as_str(),
            *action,
        ),
        Verb::ConfirmTerm { term } => commands::confirm::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            term.as_str(),
        ),
        Verb::ConfirmRelation { id, relation } => commands::confirm::run_relation(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
            relation.as_str(),
        ),
        Verb::NoteWrite { id } => commands::note::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
        ),
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
        Verb::Review { id } => match compose::ledger_from(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ) {
            Ok(ledger) => commands::review::run(&ledger, id),
            Err(error) => Outcome::Failure(*error),
        },
        Verb::List { stage } => match compose::ledger_from(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ) {
            Ok(ledger) => commands::list::run(&ledger, stage.as_deref()),
            Err(error) => Outcome::Failure(*error),
        },
        Verb::RetentionRun { dry_run } => commands::retention::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            *dry_run,
        ),
        Verb::Transcribe {
            id,
            language,
            dry_run,
        } => commands::transcribe::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
            language.as_deref(),
            *dry_run,
        ),
        Verb::Storage => match compose::storage_from(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ) {
            Ok((paths, stores)) => commands::storage::run(&paths, &stores),
            Err(error) => Outcome::Failure(*error),
        },
        Verb::Ingest { dry_run, once } => commands::ingest::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            &vpt_application::Mode {
                dry_run: *dry_run,
                once: once.clone(),
            },
        ),
    };
    emit(outcome, invocation.json)
}
