use super::*;
use crate::actions::{configure_with, note_path, sent, spoken};

#[test]
fn the_obsidian_profile_sends_an_obsidian_open_link_to_the_note() {
    let (sandbox, record) = spoken();
    let vault = sandbox.path().join("home/.vpt");
    let store = vault.join("My Notes");
    fs::create_dir_all(&store).unwrap();
    let log = configure_with(
        &sandbox,
        &format!(
            "[note]\nprofile='obsidian'\n[note.obsidian]\nvault_root={vault:?}\n[stores]\nanalysis={store:?}\n"
        ),
    );
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let note = note_path(&sandbox, &record.id);
    assert!(note.starts_with(store.to_str().unwrap()), "{note}");
    let link = format!("obsidian://open?path={}", note.replace(' ', "%20"));
    let sent = sent(&log);
    assert_eq!(sent.len(), 4);
    for args in sent {
        let at = args.iter().position(|arg| arg == "--link").unwrap();
        assert_eq!(args[at + 1], link);
    }
}
