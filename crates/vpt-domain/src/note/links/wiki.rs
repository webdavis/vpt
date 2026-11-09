use super::*;

pub(super) fn target(target: &Path, root: &Path) -> Option<String> {
    use std::path::Component;
    let relative = target.strip_prefix(root).ok()?;
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let path = if relative
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        relative.with_extension("")
    } else {
        relative.to_path_buf()
    };
    let text = path.to_str()?;
    if text.is_empty()
        || text.chars().any(|ch| {
            ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}') || "#|^:%[]\\<>&".contains(ch)
        })
    {
        return None;
    }
    Some(text.into())
}
