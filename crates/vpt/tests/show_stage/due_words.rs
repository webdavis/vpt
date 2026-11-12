use super::actions::{action, analyzed_on};
use super::*;
use serde_json::Value;
use vpt_domain::identity::RecordingId;

fn spoken(sandbox: &Sandbox, flagged: bool) -> RecordingId {
    let flags = match flagged {
        true => vec![recording::flag_word(10, "Thursday.")],
        false => vec![],
    };
    recording::seed_segments(
        sandbox,
        flags,
        &[
            "send the notes by Thursday",
            "we should send the notes",
            "Thursday.",
        ],
    )
}

fn due_on(source: &str) -> Value {
    action(
        "send notes",
        json!({"due": {"date": "2026-11-19", "sources": [source]}}),
    )
}

#[test]
fn a_due_date_is_kept_only_when_its_own_cited_words_say_a_time() {
    let (_sandbox, _id, shown, section) = analyzed_on(
        "show-action-due-time-words",
        |sandbox| spoken(sandbox, false),
        "grounding_window_secs=1\n",
        json!([due_on("00:00-00:01"), due_on("00:02-00:03")]),
    );
    assert_eq!(shown["actions"][0]["due"]["date"], "2026-11-19");
    assert_eq!(shown["actions"][0]["due"]["finding"], json!(null));
    assert_eq!(shown["actions"][1]["text"], "send notes");
    assert_eq!(shown["actions"][1]["finding"], json!(null));
    assert_eq!(
        shown["actions"][1]["due"],
        json!({"date": null, "time": null, "sources": [{"start": 2, "end": 3, "range": "00:02-00:03"}], "finding": "unsupported"})
    );
    assert_eq!(
        section,
        "## Actions\n\n- send notes [00:00-00:01] [due:: 2026-11-19]\n- send notes [00:00-00:01]\n"
    );
}

#[test]
fn a_due_date_on_a_flagged_time_word_is_kept_and_flagged() {
    let (_sandbox, _id, shown, section) = analyzed_on(
        "show-action-due-flagged-weekday",
        |sandbox| spoken(sandbox, true),
        "grounding_window_secs=1\n",
        json!([due_on("00:04-00:05")]),
    );
    assert_eq!(shown["actions"][0]["due"]["date"], "2026-11-19");
    assert_eq!(
        shown["actions"][0]["due"]["finding"],
        "built-on-flagged-text"
    );
    assert_eq!(
        section,
        "## Actions\n\n- send notes [00:00-00:01] [due:: 2026-11-19]\n"
    );
}
