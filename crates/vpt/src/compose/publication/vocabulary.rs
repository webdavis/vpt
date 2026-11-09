use vpt_adapters::known_terms::{TermsError, Update};
use vpt_application::{
    Settings,
    ports::{DirtyPublication, VocabularyKind, VocabularyPublication},
};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn publish(
    value: VocabularyPublication,
    expected: &DirtyPublication,
    settings: &Settings,
) -> Result<(), Box<ErrorDocument>> {
    if value.entry != *expected {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Ledger,
            "vocabulary publication changed",
        )));
    }
    let kind = value.kind;
    let (configured, key, rule) = match kind {
        VocabularyKind::Terms => (
            &settings.reconcile.known_terms_path,
            "reconcile.known_terms_path",
            "known_terms_path_changed",
        ),
        VocabularyKind::Tags => (
            &settings.tags.known_tags_path,
            "tags.known_tags_path",
            "known_tags_path_changed",
        ),
    };
    if value.entry.target != *configured {
        return Err(Box::new(
            ErrorDocument::new(
                ErrorKind::Refused,
                format!("pending vocabulary publication does not match {key}"),
            )
            .rule(rule),
        ));
    }
    Update::restore_saved(value)
        .and_then(|update| update.publish())
        .map_err(|error| document(kind, error))
}

pub(crate) fn term_error(error: TermsError) -> Box<ErrorDocument> {
    document(VocabularyKind::Terms, error)
}

pub(crate) fn document(kind: VocabularyKind, error: TermsError) -> Box<ErrorDocument> {
    let name = match kind {
        VocabularyKind::Terms => "known-terms",
        VocabularyKind::Tags => "known-tags",
    };
    let key = match kind {
        VocabularyKind::Terms => "known_terms",
        VocabularyKind::Tags => "known_tags",
    };
    Box::new(match error {
        TermsError::UnsafePath => {
            ErrorDocument::new(ErrorKind::Refused, format!("{name} path is unsafe"))
                .rule("path_escape")
        }
        TermsError::Changed => {
            ErrorDocument::new(ErrorKind::Refused, format!("{name} file changed"))
                .rule("publication_target_modified")
        }
        TermsError::InvalidText => ErrorDocument::new(
            ErrorKind::Refused,
            format!("{name} file contains invalid text"),
        )
        .rule(&format!("invalid_{key}")),
        TermsError::TooLarge => ErrorDocument::new(
            ErrorKind::Refused,
            format!("{name} file exceeds its limits"),
        )
        .rule(&format!("{key}_limit")),
        TermsError::Unavailable => {
            ErrorDocument::new(ErrorKind::Store, format!("{name} file is unavailable"))
        }
    })
}
