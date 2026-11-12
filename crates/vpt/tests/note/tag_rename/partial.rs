use super::*;

#[test]
fn a_failed_recording_is_listed_and_running_the_command_again_finishes_only_the_rest() {
    let (sandbox, rows) = configured_with(&[7, 8]);
    let path = known_tags(&sandbox);
    fs::write(&path, "old-topic\n").unwrap();
    for row in &rows {
        super::super::tags::update(
            &sandbox,
            &row.id,
            vec![proposed("old-topic", State::Confirmed)],
        );
        success(&invoke(&sandbox, row.id.as_str()));
    }
    let broken = saved(&sandbox, &rows[1]);
    let edited = broken.text.replace("<!-- vpt:content end -->", "removed");
    assert_ne!(edited, broken.text);
    fs::write(&broken.target, &edited).unwrap();
    let output = rename(&sandbox, &["old-topic", "new-topic"]);
    assert_ne!(output.status.code(), Some(0), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["ids"],
        serde_json::json!([rows[1].id.as_str()])
    );
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([rows[0].id.as_str()])
    );
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not renamed on 1 recording; run the same command again")
    );
    assert_eq!(saved(&sandbox, &rows[0]).managed_tags, ["new-topic"]);
    assert_eq!(saved_tags(&sandbox, &rows[1])[0].0, "old-topic");
    assert_eq!(fs::read_to_string(&path).unwrap(), "new-topic\n");
    let finished = saved(&sandbox, &rows[0]);
    fs::write(&broken.target, &broken.text).unwrap();
    let result = success(&rename(&sandbox, &["old-topic", "new-topic"]));
    assert_eq!(result["recordings"].as_array().unwrap().len(), 1);
    assert_eq!(result["recordings"][0]["id"], rows[1].id.as_str());
    assert_eq!(result["known_tags_changed"], false);
    assert_eq!(saved(&sandbox, &rows[0]), finished);
    assert_eq!(saved(&sandbox, &rows[1]).managed_tags, ["new-topic"]);
    assert_eq!(
        success(&rename(&sandbox, &["old-topic", "new-topic"]))["changed"],
        false
    );
}
