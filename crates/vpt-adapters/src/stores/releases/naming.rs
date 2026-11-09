use super::{MAX_FILES, ReleaseFileError, ReleaseFiles, ReleaseTarget};
use crate::ContainedError;
use vpt_domain::{
    identity::{RecordingId, local_timestamp, parse_local_timestamp},
    note::NoteStage,
    time::{UtcInstant, UtcOffset},
};

pub(super) fn select(
    files: &ReleaseFiles,
    id: &RecordingId,
    stage: NoteStage,
    at: UtcInstant,
    offset: UtcOffset,
) -> Result<ReleaseTarget, ReleaseFileError> {
    if offset.secs.unsigned_abs() >= 86_400 {
        return Err(ReleaseFileError::Invalid);
    }
    let stamp = local_timestamp(at, offset);
    parse_local_timestamp(&stamp).map_err(|_| ReleaseFileError::Invalid)?;
    let stamp = stamp.replace('-', "");
    let base = format!(
        "{}-{}-{}",
        id.capture_date(),
        stage.as_str(),
        &id.hash12()[..8]
    );
    let probe = files.destination.join(format!("{base}.md"));
    let (root, _) = files.stores.owner(&probe)?;
    for index in 0..=MAX_FILES {
        let name = match index {
            0 => format!("{base}.md"),
            1 => format!("{base}-{stamp}.md"),
            n => format!("{base}-{stamp}-{n}.md"),
        };
        let path = files.destination.join(name);
        match root.stat(&path) {
            Ok(_) => continue,
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => {
                root.revalidate().map_err(|e| super::contained(&path, e))?;
                return Ok(ReleaseTarget {
                    path,
                    collision: index != 0,
                });
            }
            Err(error) => return Err(super::contained(&path, error)),
        }
    }
    Err(ReleaseFileError::Capacity)
}
