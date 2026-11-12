use super::*;
use vpt_application::ports::ContextWindow;

pub(super) fn collect(
    agenda: &plan::Agenda,
    window: ContextWindow,
    call: &mut impl FnMut(&[String]) -> Result<Vec<u8>, Failure>,
) -> Result<ContextSnapshot, Failure> {
    let mut snapshot = ContextSnapshot::default();
    snapshot.status.calendar = ContextStatus::Present;
    for event in decode::agenda(&call(&agenda.args)?, &agenda.scopes)? {
        let mut occasion = event.occasion;
        let at = occasion.details.at.unix_second();
        if at < window.start || at >= window.end {
            continue;
        }
        if !event.attended {
            let show = ["show", &occasion.id, "--json", "--no-pull"].map(String::from);
            occasion.details.participants = decode::attendees(&call(&show)?, &occasion.id)?;
        }
        snapshot.occasions.push(occasion);
    }
    decode::validated(snapshot)
}
