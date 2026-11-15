use super::checks::Context;
use super::report::Row;
use vpt_adapters::{BUILT_AGAINST_MAJOR, HelperClient, HelperError};
use vpt_application::Settings;

const NAME: &str = "vpt-macos";
const INSTALL_HINT: &str = "install with `cargo install --git https://github.com/webdavis/vpt vpt`, then \
`swift build -c release` in helper/vpt-macos and copy .build/release/vpt-macos beside vpt";

pub(super) fn rows(context: &Context) -> Vec<Row> {
    vec![context.helper().clone()]
}

pub(super) fn probe(settings: &Settings) -> Row {
    let helper = HelperClient::new(settings.helper_path.clone());
    let path = settings.helper_path.display();
    let mut row = match helper.version() {
        Ok(version) => Row::ok(NAME, format!("{version} at {path}")),
        Err(HelperError::Absent) => {
            Row::issue(NAME, format!("{path} not found")).advice(INSTALL_HINT)
        }
        Err(HelperError::MajorMismatch { found }) => Row::issue(
            NAME,
            format!("major version {found}, this build needs {BUILT_AGAINST_MAJOR}"),
        )
        .advice(INSTALL_HINT),
        Err(HelperError::Failed(detail) | HelperError::Unknown(detail)) => Row::issue(NAME, detail),
    };
    for diagnostic in helper.take_diagnostics() {
        row.detail.push_str(&format!("; {diagnostic}"));
    }
    row
}
