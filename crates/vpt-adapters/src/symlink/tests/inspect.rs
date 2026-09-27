use super::{Fixture, LinkState};
use crate::symlink::inspect;

#[test]
fn missing_link_is_absent_without_creating_anything() {
    let fixture = Fixture::new();
    assert_eq!(inspect(&fixture.link), LinkState::Absent);
    assert!(!fixture.link.exists());
    assert!(!fixture.target.exists());
}

#[test]
fn relative_dangling_link_retains_its_literal_destination() {
    let fixture = Fixture::new();
    fixture.link_to("notes/vpt");
    assert_eq!(
        inspect(&fixture.link),
        LinkState::LinkTo("notes/vpt".into())
    );
    assert!(!fixture.target.exists());
}

#[test]
fn occupied_directory_is_identified_without_changing_it() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.link).expect("occupied directory");
    assert_eq!(
        inspect(&fixture.link),
        LinkState::Other("a directory".into())
    );
}

#[test]
fn occupied_file_is_identified_without_reading_its_contents() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.link, b"private-canary").expect("occupied file");
    assert_eq!(inspect(&fixture.link), LinkState::Other("a file".into()));
}

#[test]
fn unreadable_path_reports_only_the_error_kind() {
    let fixture = Fixture::new();
    let invalid = fixture.link.join("private-canary\0");
    assert_eq!(
        inspect(&invalid),
        LinkState::Unreadable(std::io::ErrorKind::InvalidInput.to_string())
    );
}
