use vpt_adapters::{ClonefileArchive, HelperClient, SystemClock, VoiceMemosStore};
use vpt_application::ports::{EventSink, RecordingLedger, RunHistory};
use vpt_application::{Ingest, IngestError, IngestReport, Mode, SourceSettings};

pub(crate) fn execute<L: RecordingLedger + RunHistory, E: EventSink>(
    recorder: &VoiceMemosStore,
    archive: &ClonefileArchive,
    ledger: &L,
    trash: &HelperClient,
    sink: &mut E,
    settings: &SourceSettings,
    mode: &Mode,
) -> Result<IngestReport, Box<IngestError>> {
    Ingest {
        recorder,
        archive,
        ledger,
        clock: &SystemClock,
        trash,
        sink,
        settings,
    }
    .run(mode)
}
