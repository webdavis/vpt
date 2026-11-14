use super::Verb;
use vpt_domain::copy::Kind;

#[derive(clap::Args)]
pub(super) struct Args {
    /// The recording id.
    id: String,
    /// One stage's text: released, transcript, analysis or brief.
    #[arg(long)]
    stage: Option<String>,
    /// Mask the stage's text as a release would.
    #[arg(long)]
    redact: bool,
}

impl Args {
    pub(super) fn verb(self) -> Result<Verb, String> {
        let stage = self
            .stage
            .map(|stage| {
                Kind::parse(&stage).ok_or_else(|| {
                    "show --stage must be released, transcript, analysis, or brief".to_string()
                })
            })
            .transpose()?;
        match (self.redact, stage) {
            (true, None) => return Err("--redact requires --stage".into()),
            (true, Some(Kind::Brief)) => {
                return Err(crate::commands::show::stage::BRIEF_REDACT.into());
            }
            (true, Some(Kind::Released)) => {
                return Err(
                    "--redact does not apply to --stage released, which is already redacted".into(),
                );
            }
            _ => (),
        }
        Ok(Verb::Show {
            id: self.id,
            stage,
            redact: self.redact,
        })
    }
}
