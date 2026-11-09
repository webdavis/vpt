use super::*;
use vpt_application::ShareSettings;
use vpt_domain::redaction::{Class, FlagPolicy, Options, Redactor};
pub(super) fn from_table(table: &Table) -> Result<ShareSettings, ConfigError> {
    let classes = strings(table, "share.redact.classes")?;
    if classes.len() > 7 {
        return Err(invalid("share.redact.classes", "at most 7 entries"));
    }
    let classes = classes
        .iter()
        .map(|class| match class.as_str() {
            "email" => Ok(Class::Email),
            "url" => Ok(Class::Url),
            "phone" => Ok(Class::Phone),
            "number" => Ok(Class::Number),
            "money" => Ok(Class::Money),
            _ => Err(invalid(
                "share.redact.classes",
                "email, url, phone, number, or money",
            )),
        })
        .collect::<Result<_, _>>()?;
    let redact = Options {
        classes,
        mask: text(table, "share.redact.mask")?.into(),
    };
    Redactor::new(&[], &[], redact.clone()).map_err(|_| invalid("share.redact.mask",
        "at most 256 bytes, no controls, exactly one {n}, optional {class}, and no other braces"))?;
    let flagged_spans = match text(table, "share.redact.flagged_spans")? {
        "omit" => FlagPolicy::Omit,
        "mark" => FlagPolicy::Mark,
        _ => return Err(invalid("share.redact.flagged_spans", "omit or mark")),
    };
    let source_line = text(table, "share.source_line")?;
    if source_line.len() > 65_536 || source_line.contains('\0') {
        return Err(invalid(
            "share.source_line",
            "at most 65536 bytes, without NUL",
        ));
    }
    let deny_sections = strings(table, "share.deny_sections")?;
    if deny_sections.len() > 256
        || deny_sections.iter().any(|s| s.contains('\0'))
        || deny_sections
            .iter()
            .try_fold(0usize, |n, s| n.checked_add(s.len()))
            .is_none_or(|n| n > 65_536)
    {
        return Err(invalid(
            "share.deny_sections",
            "at most 256 sections and 65536 total bytes, without NUL",
        ));
    }
    Ok(ShareSettings {
        automatic: boolean(table, "share.automatic")?,
        source_line: source_line.into(),
        include_date: boolean(table, "share.include_date")?,
        keep_timecodes: boolean(table, "share.keep_timecodes")?,
        deny_sections,
        redact,
        flagged_spans,
    })
}
fn invalid(key: &str, rule: &str) -> ConfigError {
    ConfigError::OutOfRange {
        key: key.into(),
        rule: rule.into(),
    }
}
