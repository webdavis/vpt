use super::*;

#[test]
fn an_archive_reader_keeps_container_reads_and_digest_on_the_same_file_version() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).unwrap();
    let path = audio.join("existing.m4a");
    let bytes = vpt_domain::fixtures::m4a(1_787_604_456, 12, b"audio");
    std::fs::write(&path, &bytes).unwrap();
    let reader = archive.open(&path).unwrap();
    let container =
        vpt_domain::container::inspect(archive.size(&reader).unwrap(), |offset, output| {
            archive.read_at(&reader, offset, output)
        })
        .unwrap();
    assert_eq!(container.duration_secs, 12);
    assert_eq!(
        archive.digest_handle(&reader).unwrap(),
        archive.digest(&path).unwrap()
    );
}

#[test]
fn a_reader_refuses_edits_growth_truncation_and_same_content_replacement_after_open() {
    for change in ["edit", "grow", "truncate", "replace"] {
        let (_temp, audio, _, _) = fixture(b"source");
        let archive = ClonefileArchive::open(&audio).unwrap();
        let path = audio.join("existing.m4a");
        std::fs::write(&path, b"original").unwrap();
        let reader = archive.open(&path).unwrap();
        let replacement = match change {
            "edit" => b"modified".as_slice(),
            "grow" => b"original plus more".as_slice(),
            "truncate" => b"".as_slice(),
            _ => {
                std::fs::rename(&path, audio.join("moved.m4a")).unwrap();
                b"original".as_slice()
            }
        };
        std::fs::write(&path, replacement).unwrap();
        assert!(
            matches!(archive.digest_handle(&reader), Err(ArchiveError::Sync(_))),
            "{change}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), replacement);
    }
}
