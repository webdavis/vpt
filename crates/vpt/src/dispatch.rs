use crate::{
    cli::{
        args::{Invocation, Verb},
        output::{Outcome, emit},
    },
    commands, compose, doctor,
};

pub(crate) fn run(invocation: &Invocation) -> i32 {
    if invocation.progress {
        crate::cli::progress::enable();
    }
    let outcome = match &invocation.verb {
        Verb::Print(text) => return emit(Outcome::printed(text.clone()), None),
        Verb::Redact(options) => commands::redact::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            options,
        ),
        Verb::Brief(options) => commands::brief::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            options,
        ),
        Verb::Occasions => observe(invocation, commands::occasions::run),
        Verb::OccasionPath { id, stage } => observe(invocation, |ledger| {
            commands::occasions::path(ledger, id, stage)
        }),
        Verb::Analyze {
            id,
            dry_run,
            resend_refused,
        } => commands::analyze::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
            *dry_run,
            *resend_refused,
        ),
        Verb::AnalyzeBatch { scope, dry_run } => commands::analyze::batch::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            scope,
            *dry_run,
        ),
        Verb::VerifyNote { path, recording } => commands::verify::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            path,
            recording,
        ),
        Verb::Run {
            dry_run,
            resend_refused,
        } => commands::pipeline::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            *dry_run,
            *resend_refused,
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
        Verb::TagRename { old, new, dry_run } => commands::tag::rename(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            &commands::tag::Request {
                old: old.as_str(),
                new: new.as_str(),
                dry_run: *dry_run,
            },
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
        Verb::Doctor { checks, fix } => doctor::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            checks,
            *fix,
        ),
        Verb::Test { parts } => crate::self_test::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            parts,
        ),
        Verb::Ledger(action) => commands::ledger::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            action,
        ),
        Verb::Version => commands::version::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ),
        Verb::Setup { overwrite, engine } => commands::setup::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            *overwrite,
            engine.as_deref(),
        ),
        Verb::SymlinkDeploy => commands::symlink::deploy(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ),
        Verb::SymlinkVerify => commands::symlink::verify(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
        ),
        Verb::Path { id, stage } => {
            observe(invocation, |ledger| commands::path::run(ledger, id, stage))
        }
        Verb::Show {
            id,
            stage: Some(stage),
            redact,
        } => commands::show::stage::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            id,
            *stage,
            *redact,
        ),
        Verb::Show {
            id, stage: None, ..
        } => observe(invocation, |ledger| commands::show::run(ledger, id)),
        Verb::Search(options) => commands::search::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            options,
        ),
        Verb::Review { id } => observe(invocation, |ledger| commands::review::run(ledger, id)),
        Verb::ActionsList { id } => observe(invocation, |ledger| {
            commands::actions::list(ledger, id.as_deref())
        }),
        Verb::List { stage } => observe(invocation, |ledger| {
            commands::list::run(ledger, stage.as_deref())
        }),
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
                import: None,
            },
        ),
        Verb::Import(options) => commands::import::run(
            &compose::Environment::from_process(),
            invocation.config.as_deref(),
            options,
        ),
    };
    let outcome = crate::cli::terminal::styled(outcome, &invocation.verb, invocation.format);
    emit(outcome, invocation.format)
}

fn observe(
    invocation: &Invocation,
    command: impl FnOnce(&compose::RuntimeLedger) -> Outcome,
) -> Outcome {
    match compose::ledger_from(
        &compose::Environment::from_process(),
        invocation.config.as_deref(),
    ) {
        Ok(ledger) => command(&ledger),
        Err(error) => Outcome::Failure(*error),
    }
}
