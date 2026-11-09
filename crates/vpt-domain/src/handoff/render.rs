use super::*;
use crate::note::writer::Writer;
pub fn content(document: &Document<'_>) -> Result<String, Error> {
    document.validate()?;
    let mut output = Writer::default();
    let write = (|| {
        output.literal(&format!("# vpt {} copy, ", document.kind.as_str()))?;
        output.text(document.stem)?;
        output.literal(&format!(
            "\n\nIdentity: {}\nOpen flags: {}\nReview state: {}\nThis is a derived text copy.\n\n",
            document.identity.as_str(),
            document.provenance.open_flags,
            if document.provenance.reviewed {
                "reviewed"
            } else {
                "unreviewed"
            }
        ))?;
        output.literal(document.body)
    })();
    write.map_err(|_| Error::CapacityExceeded)?;
    Ok(output.finish())
}
