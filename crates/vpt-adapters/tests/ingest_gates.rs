mod support;

use support::gates::{CloneBehavior, GateRecorder};
use support::{CAPTURED, Fixture, Parts, StopAt};
use vpt_application::ports::{Archive, RecordingLedger, RunHistory, TrashError};
use vpt_application::{Ingest, IngestError, IngestFailure, IngestReport, Mode};
use vpt_domain::fixtures::{box_of, m4a, mvhd_v1};
use vpt_domain::history::{EventDetail, EventKind, EventPathKey, EventState};
use vpt_domain::run::{HistoryFact, WorkOutcome};
use vpt_domain::sweep::{DeferralReason, SF_DATALESS};

#[path = "ingest_gates/deferral.rs"]
mod deferral;
#[path = "ingest_gates/failure.rs"]
mod failure;
#[path = "ingest_gates/source.rs"]
mod source;
#[path = "ingest_gates/staged.rs"]
mod staged;

fn probed(parts: &mut Parts, probe: &GateRecorder) -> Result<IngestReport, Box<IngestError>> {
    Ingest {
        recorder: probe,
        archive: &parts.archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    }
    .run(&Mode::default())
}

fn set_mtime(path: &std::path::Path, secs: i64) {
    use std::time::{Duration, UNIX_EPOCH};
    let time = UNIX_EPOCH + Duration::from_secs(u64::try_from(secs).unwrap());
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(time))
        .unwrap();
}
