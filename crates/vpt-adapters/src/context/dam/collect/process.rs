use super::*;
use crate::spawn::Status;

pub(super) fn output(result: Result<Outcome, SpawnError>) -> Result<Vec<u8>, Failure> {
    let outcome = result.map_err(|error| match error {
        SpawnError::NotFound(_) => Failure::Unavailable,
        SpawnError::Io(_) => Failure::Io,
    })?;
    match outcome.status {
        Status::Exited(0) => (),
        Status::DeadlineExceeded => return Err(Failure::Deadline),
        Status::Interrupted => return Err(Failure::Interrupted),
        _ => return Err(Failure::Exit),
    }
    if outcome.stdout.len() > 65_536 || outcome.stderr.len() > 65_536 {
        return Err(Failure::Capacity);
    }
    if vpt_protocol::read_bounded(&outcome.stderr, &vpt_protocol::Limits::incoming())
        .is_ok_and(|v| v.get("error").is_some())
    {
        return Err(Failure::Exit);
    }
    for line in outcome.stderr.split(|c| *c == b'\n') {
        if vpt_protocol::read_bounded(line, &vpt_protocol::Limits::incoming())
            .is_ok_and(|v| v.get("error").is_some())
        {
            return Err(Failure::Exit);
        }
    }
    Ok(outcome.stdout)
}

pub(super) fn merge(target: &mut ContextSnapshot, input: ContextSnapshot) -> Result<(), Failure> {
    for row in input.occasions {
        match target.occasions.iter().find(|other| other.id == row.id) {
            Some(other) if other != &row => return Err(Failure::Invalid),
            Some(_) => (),
            None => target.occasions.push(row),
        }
    }
    for row in input.tasks {
        match target.tasks.iter().find(|other| other.id == row.id) {
            Some(other) if other != &row => return Err(Failure::Invalid),
            Some(_) => (),
            None => target.tasks.push(row),
        }
    }
    vpt_protocol::context::encode(target).map_err(|_| Failure::Capacity)?;
    Ok(())
}
