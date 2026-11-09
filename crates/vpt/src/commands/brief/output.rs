use super::*;
use vpt_application::ports::BriefArtifact;
pub(super) fn success(
    artifact: &BriefArtifact,
    written: bool,
    explain: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let bytes =
        vpt_protocol::brief::encode(&artifact.snapshot.document()).map_err(|_| invalid())?;
    let document = serde_json::from_str(&bytes).map_err(|_| invalid())?;
    let mut human = artifact.text.clone();
    if explain {
        human.push_str("\nSelection\n");
        for decision in &artifact.snapshot.pack.selection.decisions {
            let state = if decision.selected {
                "selected"
            } else {
                "not selected"
            };
            let selector = decision
                .matched
                .as_ref()
                .map_or("none", |m| m.selector.as_str());
            let matched = decision
                .matched
                .as_ref()
                .map(|m| {
                    m.values
                        .iter()
                        .map(|v| v.escape_debug().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let reason = decision
                .rejected
                .map(|r| r.as_str())
                .unwrap_or(if decision.selected {
                    "selected"
                } else {
                    "note-limit"
                });
            human.push_str(&format!(
                "{}: {state}; {selector}; matched: {matched}; {reason}\n",
                decision.recording
            ));
        }
    }
    Ok(Outcome::Success {
        document,
        human,
        completed: if written {
            vec![artifact.snapshot.id.to_string()]
        } else {
            vec![]
        },
        diagnostics: vec![],
    })
}
