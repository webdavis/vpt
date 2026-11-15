#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Ok,
    Warning,
    Issue,
    NotRun,
}

impl Verdict {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Issue => "issue",
            Self::NotRun => "not_run",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Self::Ok => "✓",
            Self::Warning => "⚠",
            Self::Issue => "✗",
            Self::NotRun => "·",
        }
    }
}

pub(crate) fn version_line() -> String {
    format!("vpt {}\n", env!("CARGO_PKG_VERSION"))
}

pub(crate) fn heading(title: &str, blurb: &str) -> String {
    format!("\n{title} ── {blurb}\n")
}

pub(crate) fn row(verdict: Verdict, name: &str, detail: &str) -> String {
    format!("  {} {name}: {detail}\n", verdict.glyph())
}

pub(crate) fn mark(verdict: Verdict, text: &str) -> String {
    format!("  {} {text}\n", verdict.glyph())
}

pub(crate) fn advice(text: &str) -> String {
    format!("    → {text}\n")
}

pub(crate) fn needs_attention(entries: &[(Verdict, String)]) -> String {
    let issues = count(entries, Verdict::Issue, "issue to fix", "issues to fix");
    let warnings = count(
        entries,
        Verdict::Warning,
        "warning to look at",
        "warnings to look at",
    );
    let mut text = heading_alone("Needs attention");
    let headline = issues.iter().chain(&warnings).cloned();
    let headline = headline.collect::<Vec<_>>().join(", ");
    let verdict = match (&issues, &warnings) {
        (None, None) => return text + &mark(Verdict::Ok, "nothing needs attention"),
        (None, Some(_)) => Verdict::Warning,
        (Some(_), _) => Verdict::Issue,
    };
    text.push_str(&mark(verdict, &headline));
    let listed = [Verdict::Issue, Verdict::Warning]
        .into_iter()
        .flat_map(|wanted| {
            entries
                .iter()
                .filter(move |(verdict, _)| *verdict == wanted)
        });
    for (index, (_, entry)) in listed.enumerate() {
        text.push_str(&format!("  {}. {entry}\n", index + 1));
    }
    text
}

fn heading_alone(title: &str) -> String {
    format!("\n{title}\n")
}

fn count(entries: &[(Verdict, String)], wanted: Verdict, one: &str, many: &str) -> Option<String> {
    match entries
        .iter()
        .filter(|(verdict, _)| *verdict == wanted)
        .count()
    {
        0 => None,
        1 => Some(format!("1 {one}")),
        number => Some(format!("{number} {many}")),
    }
}
