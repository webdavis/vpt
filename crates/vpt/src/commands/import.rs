use crate::cli::args::import::Options;
use crate::cli::output::Outcome;
use crate::compose::{Environment, settings_from};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use vpt_adapters::{ExtractError, extract_audio};
use vpt_application::{Import, Mode};
use vpt_domain::time::UtcInstant;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(environment: &Environment, config: Option<&Path>, options: &Options) -> Outcome {
    let Some((origin, modified)) = std::fs::canonicalize(&options.file)
        .and_then(|origin| std::fs::metadata(&origin).map(|metadata| (origin, metadata)))
        .ok()
        .filter(|(_, metadata)| metadata.is_file())
    else {
        return Outcome::Failure(ErrorDocument::new(
            ErrorKind::Usage,
            "import file is not a readable file",
        ));
    };
    let import = Import {
        title: options.title.as_ref().map_or_else(
            || {
                origin
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            },
            |title| title.as_str().to_owned(),
        ),
        captured_at: options.captured_at,
        modified_at: UtcInstant {
            secs: modified.mtime(),
        },
        origin: origin.clone(),
    };
    if origin
        .extension()
        .is_some_and(|extension| extension == "m4a")
    {
        return ingest(environment, config, origin, import);
    }
    let settings = match settings_from(environment, config) {
        Ok(settings) => settings,
        Err(error) => return Outcome::Failure(*error),
    };
    match extract_audio(&settings.import_extract_command, &origin) {
        Ok(extracted) => ingest(environment, config, extracted.path.clone(), import),
        Err(error) => Outcome::Failure(extract_error(error)),
    }
}

pub(super) fn recordings_dir<'a>(mode: &'a Mode, configured: &'a Path) -> &'a Path {
    match (&mode.import, mode.once.as_deref().and_then(Path::parent)) {
        (Some(_), Some(directory)) => directory,
        _ => configured,
    }
}

fn ingest(
    environment: &Environment,
    config: Option<&Path>,
    audio: PathBuf,
    import: Import,
) -> Outcome {
    super::ingest::run(
        environment,
        config,
        &Mode {
            dry_run: false,
            once: Some(audio),
            import: Some(import),
        },
    )
}

fn extract_error(error: ExtractError) -> ErrorDocument {
    match error {
        ExtractError::NotConfigured => ErrorDocument::new(
            ErrorKind::Refused,
            "import.extract_command is empty; audio extraction is not configured",
        ),
        ExtractError::Missing(program) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("audio extractor {program} was not found; set import.extract_command"),
        ),
        ExtractError::Failed(detail) => {
            ErrorDocument::new(ErrorKind::Refused, "audio extraction failed")
                .diagnostics(vec![detail])
        }
    }
    .rule("audio_extract")
}
