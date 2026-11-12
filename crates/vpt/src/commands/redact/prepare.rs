use super::{errors, output::Summary, source::Source};
use crate::cli::args::redact::Options;
use vpt_adapters::{
    Roots,
    redaction::{self, ContentOptions, ReviewSource, document},
};
use vpt_application::{
    Settings,
    ports::{NewRelease, ReleaseProvenance},
};
use vpt_domain::{
    note::NoteStage,
    redaction::Redactor,
    review::{FlagContent, Resolution, ReviewFlag},
};
use vpt_protocol::error::ErrorDocument;
mod flags;
mod provenance;
pub(crate) mod redactor;
pub(super) struct Prepared {
    pub content: redaction::Content,
    pub report: String,
    pub provenance: ReleaseProvenance,
    pub summary: Summary,
}
pub(crate) struct Masked {
    pub content: redaction::Content,
    pub flags: Vec<ReviewFlag>,
}
pub(crate) fn mask(
    source: &Source,
    stage: NoteStage,
    settings: &Settings,
    redactor: &mut Redactor,
) -> Result<Masked, Box<ErrorDocument>> {
    let accepted = source
        .input
        .review
        .accepted
        .as_ref()
        .ok_or_else(errors::missing)?;
    let (content, _) = document::owned_content(&source.note.text, &source.input.record.id, stage)
        .map_err(errors::redaction)?;
    let flags = flags::current(source, stage, content, settings)?;
    let review = match stage {
        NoteStage::Transcript => ReviewSource::Transcript {
            transcript: &accepted.transcript,
            flags: &flags,
        },
        NoteStage::Analysis => ReviewSource::Analysis { flags: &flags },
    };
    let content = redaction::redact_reviewed(
        content,
        review,
        settings.share.flagged_spans,
        redactor,
        &options(settings),
    )
    .map_err(errors::redaction)?;
    Ok(Masked { content, flags })
}
fn options(settings: &Settings) -> ContentOptions {
    ContentOptions {
        deny_sections: settings.share.deny_sections.clone(),
        keep_timecodes: settings.share.keep_timecodes,
    }
}
pub(super) fn run(
    source: &Source,
    options: &Options,
    settings: &Settings,
    roots: &Roots,
) -> Result<Prepared, Box<ErrorDocument>> {
    let mut redactor = redactor::build(settings, roots)?;
    let Masked { content, flags } = mask(source, options.stage, settings, &mut redactor)?;
    let content = document::assemble(
        content,
        &document::Header {
            title: options.title.as_ref().map(|t| t.as_str().to_owned()),
            date: settings
                .share
                .include_date
                .then(|| options.id.capture_date().to_owned()),
            source: settings.share.source_line.clone(),
        },
        &redactor,
    )
    .map_err(errors::redaction)?;
    let report =
        redaction::report::encode(&content, &source.note.text, source.offset, &source.path)
            .map_err(errors::redaction)?;
    let summary = Summary::from_content(&content);
    Ok(Prepared {
        content,
        report,
        summary,
        provenance: provenance::snapshot(source, options.stage, &flags)?,
    })
}
impl Prepared {
    pub fn draft(
        &self,
        source: &Source,
        options: &Options,
        roots: &Roots,
        destination: std::path::PathBuf,
        at: vpt_domain::time::UtcInstant,
    ) -> NewRelease {
        NewRelease {
            recording: options.id.clone(),
            stage: options.stage,
            destination,
            report_root: roots.stores.drafts.clone(),
            source_digest: source.note.digest,
            text: self.content.text.clone(),
            report: self.report.clone(),
            provenance: self.provenance.clone(),
            created_at: at,
        }
    }
}
