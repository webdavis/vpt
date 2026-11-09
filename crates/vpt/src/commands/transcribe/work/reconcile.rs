use vpt_application::{Settings, transcribe::Transcription};
use vpt_domain::uncertainty::Classification;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn classify(
    result: &Transcription,
    settings: &Settings,
) -> Result<Classification, Box<ErrorDocument>> {
    let terms = if result.lexical_classification {
        vpt_adapters::known_terms::read(&settings.reconcile.known_terms_path).map_err(|error| {
            Box::new(ErrorDocument::new(
                ErrorKind::Store,
                format!("known terms unavailable: {error:?}"),
            ))
        })?
    } else {
        vec![]
    };
    let terms: Vec<_> = terms.iter().map(String::as_str).collect();
    result
        .classify(settings.reconcile.options, &terms)
        .map_err(|error| {
            Box::new(ErrorDocument::new(
                ErrorKind::Command,
                format!("uncertainty classification failed: {error:?}"),
            ))
        })
}
