use super::*;

pub(super) fn failure(stderr: &[u8], sent: &[String]) -> Failure {
    let word = match vpt_protocol::read_bounded(stderr, &vpt_protocol::Limits::incoming()) {
        Ok(document) => reported(&document["error"]["usage"]),
        Err(_) => printed(&String::from_utf8_lossy(stderr)),
    };
    if word.is_some_and(|word| sent.contains(&word)) {
        Failure::Usage
    } else {
        Failure::Rejected
    }
}

fn reported(usage: &serde_json::Value) -> Option<String> {
    matches!(
        usage["problem"].as_str()?,
        "unknown_subcommand" | "unknown_flag"
    )
    .then(|| usage["word"].as_str().map(str::to_owned))?
}

fn printed(text: &str) -> Option<String> {
    let line = text.lines().next()?;
    ["unrecognized subcommand '", "unexpected argument '"]
        .iter()
        .find_map(|opening| line.split_once(opening))
        .and_then(|(_, rest)| rest.split_once('\''))
        .map(|(word, _)| word.to_owned())
}
