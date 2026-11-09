use super::*;
pub(super) fn run<L, S, E>(
    ledger: &L,
    stores: &S,
    settings: &vpt_application::Settings,
    sink: &mut E,
    warnings: &mut BTreeSet<SinkWarning>,
    report_briefs: bool,
) -> Result<Vec<String>, Box<ErrorDocument>>
where
    L: PublicationJournal
        + RunHistory
        + EngineOutputLedger
        + NoteLedger
        + BriefLedger
        + VocabularyLedger,
    S: Stores,
    E: EventSink,
{
    let mut completed = Vec::new();
    let result = (|| {
        let entries = ledger
            .pending_publications()
            .map_err(|error| Box::new(ledger_error(error)))?;
        if entries.is_empty() {
            return Ok(());
        }
        let run = commit(
            ledger.begin_run(RunKind::Publish, SystemClock.now()),
            sink,
            warnings,
        )?
        .run;
        let result = (|| {
            let mut publisher = entry::Publisher {
                report_briefs,
                ledger,
                stores,
                settings,
                sink,
                warnings,
            };
            for entry in entries {
                if let Some(id) = publisher.publish(run, entry)? {
                    let id = id.to_string();
                    if !completed.contains(&id) {
                        completed.push(id);
                    }
                }
            }
            Ok::<_, Box<ErrorDocument>>(())
        })();
        if result.as_ref().is_err_and(|error| {
            error.kind == vpt_protocol::error::ErrorKind::Ledger
                || error.rule.as_deref() == Some("unsafe_state")
        }) {
            return result;
        }
        let finish = commit(
            ledger.commit_step(
                run,
                &WorkDelta::default(),
                &[(
                    SystemClock.now(),
                    HistoryFact::RunFinished(if result.is_ok() {
                        RunOutcome::Completed
                    } else {
                        RunOutcome::Failed
                    }),
                )],
            ),
            sink,
            warnings,
        );
        match result {
            Err(mut error) => {
                if let Err(finish) = finish {
                    merge_completed(&mut error, finish.completed);
                }
                Err(error)
            }
            Ok(()) => finish.map(|_| ()),
        }
    })();
    match result {
        Ok(()) => Ok(completed),
        Err(mut error) => {
            merge_completed(&mut error, completed);
            Err(error)
        }
    }
}
