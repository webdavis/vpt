use super::*;

pub(super) fn validate(path: &Path) -> Result<(), RenderError> {
    let text = path.to_str().ok_or(RenderError::InvalidName)?;
    if text.len() > 262_144 || text.chars().count() > 65_536 {
        return Err(RenderError::CapacityExceeded);
    }
    Ok(())
}

pub(super) fn relative(target: &Path, origin: &Path) -> Result<String, RenderError> {
    let from: Vec<_> = origin.components().collect();
    let to: Vec<_> = target.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in common..from.len() {
        relative.push("..");
    }
    for part in &to[common..] {
        relative.push(part.as_os_str());
    }
    let relative = relative.to_str().ok_or(RenderError::InvalidName)?;
    Ok(relative
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect())
}
