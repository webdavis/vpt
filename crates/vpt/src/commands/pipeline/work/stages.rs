use super::*;
use crate::compose::errors::ledger_error;

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    stores: &FilesystemStores,
    run: RunId,
    report: &mut Report,
    notices: &mut ReviewQueue,
) -> Result<(), Box<ErrorDocument>> {
    let mentions = crate::compose::mentions::Context::read(context.settings, context.roots)?;
    let mut records = context
        .ledger
        .recordings()
        .map_err(|error| Box::new(ledger_error(error)))?;
    if records.len() > 65_536 {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Ledger,
            "pipeline recording capacity exceeded",
        )));
    }
    records.sort_by(|a, b| {
        (a.captured_at.secs, a.id.as_str()).cmp(&(b.captured_at.secs, b.id.as_str()))
    });
    for mut record in records {
        super::eligibility::refresh(context, stores, run, &mut record)?;
        let before = (report.notes.len(), report.analyzed.len());
        super::initial::run(context, stores, &mentions, &record, run, report, notices)?;
        super::verification::run(
            context,
            stores,
            &mentions,
            &record.id,
            NoteStage::Transcript,
            report,
        )?;
        super::notices::refresh(context, &record.id, before, report, notices)?;
        super::sharing::run(context, &record.id, NoteStage::Transcript, run, report)?;
        super::eligibility::refresh(context, stores, run, &mut record)?;
        super::analysis::run(context, &mentions, &record.id, report)?;
        super::notices::refresh(context, &record.id, before, report, notices)?;
        super::verification::run(
            context,
            stores,
            &mentions,
            &record.id,
            NoteStage::Analysis,
            report,
        )?;
        super::notices::refresh(context, &record.id, before, report, notices)?;
        super::sharing::run(context, &record.id, NoteStage::Analysis, run, report)?;
    }
    Ok(())
}
