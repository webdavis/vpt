use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use vpt_adapters::SystemClock;
use vpt_application::ports::Clock;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

static ENABLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

pub(crate) fn step<T>(
    id: &RecordingId,
    step: &str,
    audio_secs: Option<f64>,
    work: impl FnOnce() -> Result<T, Box<ErrorDocument>>,
) -> Result<T, Box<ErrorDocument>> {
    report(id, step, "started", audio_secs);
    let result = work();
    let committed = match &result {
        Ok(_) => true,
        Err(error) => error.completed.contains(&id.to_string()),
    };
    report(
        id,
        step,
        if committed { "done" } else { "failed" },
        audio_secs,
    );
    result
}

pub(crate) fn report(id: &RecordingId, step: &str, state: &str, audio_secs: Option<f64>) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let line = serde_json::json!({
        "schema": "vpt.progress/1",
        "recording": id.as_str(),
        "step": step,
        "state": state,
        "audio_secs": audio_secs,
        "at": SystemClock.now().rfc3339(),
    });
    // A closed or full standard error must not fail the work it describes.
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}
