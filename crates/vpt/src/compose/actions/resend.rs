use super::Delivery;
use vpt_application::{Settings, ports::*};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn require_command(settings: &Settings) -> Result<(), Box<ErrorDocument>> {
    if settings.actions_command.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "--resend-refused requires [actions] command",
        )));
    }
    Ok(())
}

pub(super) fn refused<E: EventSink>(
    delivery: &mut Delivery<'_, E>,
    only: Option<&RecordingId>,
) -> Result<(), Box<ErrorDocument>> {
    delivery.commit_change(only, ActionChange::ResendRefused(only.cloned()))
}
