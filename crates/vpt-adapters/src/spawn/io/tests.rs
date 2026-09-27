use super::*;
use std::collections::VecDeque;
use std::io::{Cursor, ErrorKind};

struct ScriptedReader {
    outcomes: VecDeque<io::Result<Vec<u8>>>,
}

impl Read for ScriptedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let bytes = self.outcomes.pop_front().expect("scripted read")?;
        buffer[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

#[test]
fn capture_keeps_only_the_limit_and_drains_the_rest_to_eof() {
    for limit in [0, 1, 1000] {
        let input = vec![b'x'; 200_000];
        let mut reader = Cursor::new(input);
        let mut capture = Capture::new(limit);
        for _ in 0..100 {
            capture.drain(&mut reader).expect("drain");
            if capture.finished {
                break;
            }
        }
        assert!(capture.finished, "finite stream never reached EOF");
        assert_eq!(
            reader.position(),
            200_000,
            "unretained output was not drained"
        );
        assert!(
            capture.bytes == vec![b'x'; limit],
            "retained bytes differ: expected {limit}, received {}",
            capture.bytes.len()
        );
    }
}

#[test]
fn capture_retries_interrupted_reads_and_preserves_data() {
    let mut reader = ScriptedReader {
        outcomes: [
            Err(ErrorKind::Interrupted.into()),
            Ok(b"owned".to_vec()),
            Ok(Vec::new()),
        ]
        .into(),
    };
    let mut capture = Capture::new(100);
    capture
        .drain(&mut reader)
        .expect("interrupted read retried");
    assert!(capture.finished);
    assert_eq!(capture.bytes, b"owned");
}

#[test]
fn genuine_read_failure_is_not_eof_or_partial_success() {
    let mut reader = ScriptedReader {
        outcomes: [
            Ok(b"private-capture".to_vec()),
            Err(ErrorKind::PermissionDenied.into()),
        ]
        .into(),
    };
    let mut capture = Capture::new(100);
    let failure = capture
        .drain(&mut reader)
        .expect_err("genuine read failure");
    assert_eq!(failure.kind(), ErrorKind::PermissionDenied);
    assert!(!capture.finished);
}

#[test]
fn would_block_keeps_the_capture_open() {
    let mut reader = ScriptedReader {
        outcomes: [
            Err(ErrorKind::WouldBlock.into()),
            Ok(b"later".to_vec()),
            Ok(Vec::new()),
        ]
        .into(),
    };
    let mut capture = Capture::new(100);
    assert!(!capture.drain(&mut reader).expect("pending read"));
    assert!(!capture.finished, "temporary blocking was mistaken for EOF");
    capture.drain(&mut reader).expect("read after readiness");
    assert_eq!(capture.bytes, b"later");
    assert!(capture.finished);
}

struct ScriptedWriter {
    outcomes: VecDeque<io::Result<usize>>,
    bytes: Vec<u8>,
}

impl Write for ScriptedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self
            .outcomes
            .pop_front()
            .expect("scripted write")?
            .min(bytes.len());
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn input_retries_interruption_and_partial_writes_without_losing_bytes() {
    let mut writer = ScriptedWriter {
        outcomes: [
            Err(ErrorKind::Interrupted.into()),
            Ok(2),
            Err(ErrorKind::WouldBlock.into()),
            Ok(20),
        ]
        .into(),
        bytes: Vec::new(),
    };
    let mut input = Input::new(b"owned input");
    let mut complete = false;
    for _ in 0..8 {
        complete = input.write(&mut writer).expect("write input");
        if complete {
            break;
        }
    }
    assert!(complete, "finite input did not finish");
    assert_eq!(writer.bytes, b"owned input");
}

#[test]
fn input_failure_and_write_zero_are_typed_failures() {
    for (result, kind) in [
        (Err(ErrorKind::BrokenPipe.into()), ErrorKind::BrokenPipe),
        (Ok(0), ErrorKind::WriteZero),
    ] {
        let mut writer = ScriptedWriter {
            outcomes: [result].into(),
            bytes: Vec::new(),
        };
        let mut input = Input::new(b"unwritten private input");
        assert_eq!(
            input
                .write(&mut writer)
                .expect_err("incomplete input")
                .kind(),
            kind
        );
    }
}
