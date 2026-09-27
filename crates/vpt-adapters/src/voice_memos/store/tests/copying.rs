use super::super::cloning::copy_contents;
use super::*;
use std::io::{self, Write};

struct Sink<F> {
    bytes: Vec<u8>,
    chunks: Vec<usize>,
    first: Option<F>,
}

impl<F: FnOnce()> Write for Sink<F> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.chunks.push(buffer.len());
        self.bytes.extend_from_slice(buffer);
        if let Some(first) = self.first.take() {
            first();
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn collisions(f: &Fixture) {
    cloning::prepare_collisions(f);
    let before = snapshot(&f.audio);
    f.unchanged(|| {
        for name in ["existing.m4a", "link.m4a", "directory.m4a", "pipe.m4a"] {
            cloning::fallback_collision(f, name);
        }
    });
    assert_eq!(snapshot(&f.audio), before);
}

pub(super) fn bounds(f: &Fixture) {
    let original = vec![7; 200_000];
    let path = f.recordings.join("a.m4a");
    fs::write(&path, &original).unwrap();
    let source = f.source();
    let mut after_fixture_append = None;
    let mut out = Sink {
        bytes: Vec::new(),
        chunks: Vec::new(),
        first: Some(|| {
            fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap()
                .write_all(b"fixture append after copying starts")
                .unwrap();
            after_fixture_append = Some(snapshot(&f.recordings));
        }),
    };
    assert_eq!(copy_contents(&source, &mut out), Ok(()));
    assert!(
        out.bytes == original,
        "copied bytes must match the initial source snapshot"
    );
    assert!(out.chunks.len() > 1);
    assert!(
        out.chunks
            .iter()
            .all(|length| *length > 0 && *length <= 64 * 1024)
    );
    drop(out);
    assert_eq!(snapshot(&f.recordings), after_fixture_append.unwrap());

    fs::write(&path, []).unwrap();
    f.unchanged(|| {
        let mut out = Sink {
            bytes: Vec::new(),
            chunks: Vec::new(),
            first: Some(|| panic!("empty source caused a write")),
        };
        assert_eq!(copy_contents(&source, &mut out), Ok(()));
        assert!(out.bytes.is_empty());
    });
}

struct FailingWriter(i32);

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        if self.0 == 0 {
            Ok(0)
        } else {
            Err(io::Error::from_raw_os_error(self.0))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn owned_error(f: &Fixture) {
    f.unchanged(|| {
        assert_eq!(
            copy_contents(&f.source(), &mut FailingWriter(libc::EEXIST)),
            Err(CloneError::Io(io::ErrorKind::AlreadyExists.to_string()))
        );
    });
}

pub(super) fn errors(f: &Fixture) {
    let source = f.source();
    f.unchanged(|| {
        assert_eq!(
            copy_contents(&source, &mut FailingWriter(libc::ENOSPC)),
            Err(CloneError::NoSpace)
        );
        assert_eq!(
            copy_contents(&source, &mut FailingWriter(libc::EIO)),
            Err(CloneError::Io(
                io::Error::from_raw_os_error(libc::EIO).kind().to_string()
            ))
        );
        assert_eq!(
            copy_contents(&source, &mut FailingWriter(0)),
            Err(CloneError::Io(io::ErrorKind::WriteZero.to_string()))
        );
    });
    fs::write(f.recordings.join("a.m4a"), vec![9; 100_000]).unwrap();
    let mut after_fixture_truncate = None;
    let mut out = Sink {
        bytes: Vec::new(),
        chunks: Vec::new(),
        first: Some(|| {
            fs::OpenOptions::new()
                .write(true)
                .open(f.recordings.join("a.m4a"))
                .unwrap()
                .set_len(0)
                .unwrap();
            after_fixture_truncate = Some(snapshot(&f.recordings));
        }),
    };
    assert_eq!(
        copy_contents(&source, &mut out),
        Err(CloneError::Io(io::ErrorKind::UnexpectedEof.to_string()))
    );
    assert_eq!(out.bytes.len(), 64 * 1024);
    drop(out);
    assert_eq!(snapshot(&f.recordings), after_fixture_truncate.unwrap());
}
