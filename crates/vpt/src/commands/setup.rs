use crate::cli::output::Outcome;
use crate::compose::Environment;
use serde_json::json;
use std::path::Path;
use vpt_adapters::TtyPrompt;
use vpt_adapters::config::{FilesystemSetupWriter, from_table, load_text, resolve, template};
use vpt_application::{Setup, SetupError, SetupWriter};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub(crate) fn run(environment: &Environment, config: Option<&Path>, force: bool) -> Outcome {
    match prepare(environment, config, force) {
        Ok(outcome) => outcome,
        Err(error) => Outcome::Failure(error),
    }
}

#[allow(clippy::result_large_err, reason = "one-shot CLI result stays inline")]
fn prepare(
    environment: &Environment,
    config: Option<&Path>,
    force: bool,
) -> Result<Outcome, ErrorDocument> {
    let selected = match config {
        Some(path) => path.to_path_buf(),
        None => environment.config_path().map_err(config_error)?,
    };
    let state_dir = environment.state_dir_default().map_err(config_error)?;
    let home_dir = environment.home_dir().map_err(config_error)?;
    let settings = load_text(&template("apple", &state_dir))
        .and_then(|table| from_table(&table, &home_dir))
        .map_err(config_error)?;
    let name = selected.file_name().ok_or_else(|| {
        ErrorDocument::new(ErrorKind::Usage, "the configuration path needs a file name")
    })?;
    let parent = selected
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let roots = resolve(&settings, parent).map_err(config_error)?;
    let mut directories = vec![roots.home.clone(), roots.state_dir.clone()];
    directories.extend(
        StoreKey::all()
            .iter()
            .map(|key| roots.stores.get(*key).to_path_buf()),
    );
    let writer = FilesystemSetupWriter::new(roots.config_dir.join(name), &directories)
        .map_err(setup_error)?;
    if writer.config_exists() && !force {
        return Err(setup_error(SetupError::Exists(writer.config_path())));
    }
    let setup = Setup {
        engines: vec!["apple".into(), "whisply".into()],
        render: |engine: &str| template(engine, &state_dir),
        directories,
    };
    let mut prompt = TtyPrompt::open().map_err(|_| setup_error(SetupError::NoTerminal))?;
    let outcome = setup
        .run(&mut prompt, &writer, force)
        .map_err(setup_error)?;
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: document(
            "setup",
            json!({"written": outcome.written, "main_engine": outcome.main_engine}),
        ),
        human: format!(
            "wrote {} (main engine {})\n",
            outcome.written.display(),
            outcome.main_engine
        ),
    })
}

fn config_error(error: impl std::fmt::Debug) -> ErrorDocument {
    ErrorDocument::new(ErrorKind::Config, format!("{error:?}"))
}

fn setup_error(error: SetupError) -> ErrorDocument {
    match error {
        SetupError::NoTerminal => {
            ErrorDocument::new(ErrorKind::Usage, "vpt setup needs a controlling terminal")
        }
        SetupError::Exists(path) => ErrorDocument::new(
            ErrorKind::Usage,
            format!("{} exists; pass --force to overwrite it", path.display()),
        ),
        SetupError::PathEscape(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("{} escapes its prepared root", path.display()),
        )
        .rule("path_escape"),
        SetupError::NonPrivateConfig(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!(
                "{} must already be private (mode 0700) and owned by the current user",
                path.display()
            ),
        )
        .rule("nonprivate_config_directory"),
        SetupError::NonPrivateDirectory(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!(
                "{} must already be private (mode 0700) and owned by the current user",
                path.display()
            ),
        )
        .rule("nonprivate_directory"),
        SetupError::MultiplyLinkedConfig(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("{} has multiple hard links", path.display()),
        )
        .rule("multiply_linked_config"),
        SetupError::Io(detail) => ErrorDocument::new(ErrorKind::Store, detail),
    }
}
