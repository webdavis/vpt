use super::checks::Context;
use super::engines::selected;
use super::parts::{not_configured, not_tested};
use super::report::Row;
use crate::style::Verdict;
use vpt_application::engines::EngineKind;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let settings = &context.settings.engines;
    let selected = selected(settings);
    let best = settings
        .definitions
        .iter()
        .filter_map(|(name, engine)| match &engine.kind {
            EngineKind::Whisply { hf_token, .. } => Some((name, hf_token.is_some())),
            _ => None,
        })
        .max_by_key(|(name, token)| (*token, selected.contains(name)));
    let name = match best {
        Some((name, true)) => name,
        Some((name, false)) => {
            return vec![not_configured("token", &format!("engines.{name}.hf_token"))];
        }
        None => return vec![not_configured("token", "engines.whisply.hf_token")],
    };
    let token = Row::ok(
        "token",
        format!(
            "set on `engines.{name}`; whisply sends it to Hugging Face to fetch the speaker model"
        ),
    );
    let engine = if selected.contains(name) {
        Row::ok("whisply", "selected")
    } else {
        Row::new(
            "whisply",
            Verdict::Warning,
            "not selected, so the token gives no speaker labels",
        )
        .advice("select it as `engines.main` or `engines.checker` in the config")
    };
    vec![token, engine, not_tested("speakers")]
}
