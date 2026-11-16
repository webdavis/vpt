use super::*;
use crate::styled::{document, piped, same_in_every_format, shown};

#[test]
fn upcoming_at_a_terminal_marks_each_written_brief() {
    let sandbox = configured();
    reply(&sandbox, vec![event("one", SystemClock.now().secs + 3600)]);
    let written = shown(&sandbox, &["brief", "upcoming"]);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let occasion = ledger.occasions().unwrap().remove(0).id;
    assert_eq!(written, format!("  ✓ {occasion}: brief written\n"));
    assert_eq!(
        shown(&sandbox, &["brief", "upcoming"]),
        "  · no brief written\n"
    );
    assert_eq!(
        piped(&sandbox, &["brief", "upcoming"]),
        "0 briefs written\n"
    );
    assert_eq!(
        document(&sandbox, &["brief", "upcoming"])["written"],
        json!([])
    );
    same_in_every_format(&sandbox, &["brief", "upcoming"]);
}

#[test]
fn a_brief_prints_its_text_unchanged_at_a_terminal() {
    let sandbox = fixture::configured("brief-styled");
    let args = [
        "brief",
        "--title",
        "Private planning",
        "--at",
        "2026-10-01T10:00:00-06:00",
        "--preview",
    ];
    for format in [
        None,
        Some("--output=json"),
        Some("--output=toon"),
        Some("--output=yaml"),
    ] {
        let args = [&args[..], format.as_slice()].concat();
        let text = piped(&sandbox, &args);
        assert!(text.contains("Private planning"), "{text}");
        assert_eq!(steady(&shown(&sandbox, &args)), steady(&text), "{format:?}");
    }
}

fn steady(text: &str) -> String {
    let stamp = |window: &[u8]| {
        window.iter().enumerate().all(|(at, byte)| match at {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            19 => *byte == b'Z',
            _ => byte.is_ascii_digit(),
        })
    };
    let (bytes, mut kept, mut at) = (text.as_bytes(), Vec::new(), 0);
    while at < bytes.len() {
        if bytes.len() - at >= 20 && stamp(&bytes[at..at + 20]) {
            kept.extend_from_slice(b"<generated>");
            at += 20;
        } else {
            kept.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(kept).unwrap()
}
