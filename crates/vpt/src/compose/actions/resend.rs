use super::Delivery;
use vpt_application::{Settings, ports::*};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn require_command(settings: &Settings) -> Result<(), Box<ErrorDocument>> {
    if settings.actions_command.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "actions resend requires [actions] command",
        )));
    }
    Ok(())
}

impl<E: EventSink> Delivery<'_, E> {
    /// Makes the refused actions of one recording, or of every recording, pending again.
    pub(crate) fn resend_refused(
        &mut self,
        only: Option<&RecordingId>,
    ) -> Result<(), Box<ErrorDocument>> {
        self.commit_change(only, ActionChange::ResendRefused(only.cloned()))
    }
}
