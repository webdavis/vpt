use super::*;

pub(super) fn validate(document: &Document<'_>) -> Result<(), EncodeError> {
    document.validate()
}
