use super::*;
use vpt_adapters::known_terms;

pub(crate) fn build(settings: &Settings, roots: &Roots) -> Result<Redactor, Box<ErrorDocument>> {
    let index = crate::compose::index::read(settings, roots)?;
    if !index.diagnostics().is_empty() {
        return Err(Box::new(
            vpt_protocol::error::ErrorDocument::new(
                vpt_protocol::error::ErrorKind::Refused,
                "note index contains unreadable names or aliases",
            )
            .rule("redact_index"),
        ));
    }
    let terms = known_terms::read(&settings.reconcile.known_terms_path)
        .map_err(crate::compose::publication::vocabulary::term_error)?;
    let terms: Vec<_> = terms.iter().map(String::as_str).collect();
    let names: Vec<_> = index
        .notes()
        .iter()
        .flat_map(|n| {
            std::iter::once(n.name.as_str()).chain(n.metadata.aliases.iter().map(String::as_str))
        })
        .collect();
    Redactor::new(&terms, &names, settings.share.redact.clone()).map_err(errors::redaction)
}
