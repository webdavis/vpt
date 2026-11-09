use super::checks::check;
use vpt_adapters::config::resolve_vault;
use vpt_application::Settings;
use vpt_protocol::error::Check;

pub(super) fn inspect(settings: &Settings) -> Check {
    match resolve_vault(settings) {
        Ok(None) => check("vault", true, "not required by the portable profile"),
        Ok(Some(path)) => check("vault", true, format!("directory {}", path.display())),
        Err(error) => check(
            "vault",
            false,
            crate::compose::errors::root_error(&error).message,
        ),
    }
}
