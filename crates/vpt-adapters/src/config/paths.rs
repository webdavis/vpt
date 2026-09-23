use std::path::PathBuf;

pub fn config_path(env: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(explicit) = env("VPT_CONFIG") {
        return PathBuf::from(explicit);
    }
    if let Some(xdg) = env("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("vpt/config.toml");
    }
    PathBuf::from(env("HOME").unwrap_or_default()).join(".config/vpt/config.toml")
}

pub fn default_state_dir(env: impl Fn(&str) -> Option<String>) -> String {
    match env("XDG_STATE_HOME") {
        Some(xdg) => format!("{xdg}/vpt"),
        None => "~/.local/state/vpt".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(pairs: &'a [(&str, &str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        }
    }

    #[test]
    fn config_discovery_prefers_explicit_then_xdg_then_home() {
        assert_eq!(
            config_path(env_of(&[
                ("VPT_CONFIG", "relative.toml"),
                ("XDG_CONFIG_HOME", "/x"),
                ("HOME", "/h")
            ])),
            PathBuf::from("relative.toml")
        );
        assert_eq!(
            config_path(env_of(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])),
            PathBuf::from("/x/vpt/config.toml")
        );
        assert_eq!(
            config_path(env_of(&[("HOME", "/h")])),
            PathBuf::from("/h/.config/vpt/config.toml")
        );
        assert_eq!(
            config_path(env_of(&[])),
            PathBuf::from(".config/vpt/config.toml")
        );
    }

    #[test]
    fn state_default_uses_xdg_or_a_tilde_home() {
        assert_eq!(
            default_state_dir(env_of(&[("XDG_STATE_HOME", "/s")])),
            "/s/vpt"
        );
        assert_eq!(default_state_dir(env_of(&[])), "~/.local/state/vpt");
    }
}
