use super::fixture::{Fixture, titled, unchanged};
use super::*;
use crate::database::{child_for_root, snapshot, test_root};
use std::process::Output;
use std::time::Instant;

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        output.stdout.escape_ascii(),
        output.stderr.escape_ascii()
    );
}

#[test]
fn separate_process_reader_blocks_refresh_until_it_closes() {
    let (temp, _root) = test_root();
    let spawn =
        |scenario| child_for_root(scenario, temp.path(), std::process::id(), Instant::now());
    let seeded = spawn("title_peer_seed").unwrap().finish().unwrap();
    success(&seeded);
    let mut reader = spawn("title_peer_reader").unwrap();
    let ready = reader.ready();
    if let Err(error) = ready {
        let cleanup = reader.finish();
        panic!("reader readiness: {error}; cleanup={cleanup:?}");
    }
    let attempt = spawn("title_peer_busy").and_then(|child| child.finish());
    let released = reader.send("continue");
    let reader_output = reader.finish();
    let attempt = attempt.unwrap();
    released.unwrap();
    let reader_output = reader_output.unwrap();
    success(&attempt);
    success(&reader_output);
    let refreshed = spawn("title_peer_refresh").unwrap().finish().unwrap();
    success(&refreshed);
}

pub(super) fn participant(scenario: &str, root: &RootDir, ready: fn()) {
    if scenario == "title_peer_seed" {
        let fixture = Fixture::simple(root);
        titled(&fixture.refresh(), "a.m4a", "A");
        return;
    }
    let state = root.open_subdirectory("private").unwrap();
    let container = root.path().join("source");
    match scenario {
        "title_peer_reader" => {
            let reader = TitleCopy::existing(state.path()).unwrap();
            titled(&reader, "a.m4a", "A");
            ready();
            titled(&reader, "a.m4a", "A");
        }
        "title_peer_busy" => {
            let before = snapshot(root.path());
            let started = Instant::now();
            let copy =
                TitleCopy::refresh_with_timeout(&container, &state, Duration::from_millis(20))
                    .unwrap();
            assert_eq!(copy.title("a.m4a"), TitleLookup::Unavailable);
            assert!(started.elapsed() >= Duration::from_millis(20));
            assert!(started.elapsed() < Duration::from_millis(250));
            unchanged(&before, &snapshot(root.path()));
        }
        "title_peer_refresh" => {
            titled(
                &TitleCopy::refresh(&container, state.path()).unwrap(),
                "a.m4a",
                "A",
            );
        }
        _ => panic!("unknown title peer scenario"),
    }
}
