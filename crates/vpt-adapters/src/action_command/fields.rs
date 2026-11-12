use vpt_domain::proposal::{CheckedAction, Owner};

pub(super) struct Fields<'a> {
    pub text: &'a str,
    pub owner: &'a str,
    pub owner_name: &'a str,
    pub due: String,
    pub due_date: &'a str,
    pub due_time: &'a str,
}

impl<'a> Fields<'a> {
    pub fn of(action: &'a CheckedAction) -> Self {
        let (owner, owner_name) = match &action.owner {
            None => ("unknown", ""),
            Some(Owner::Me) => ("me", ""),
            Some(Owner::Other(name)) => ("other", name.as_deref().unwrap_or("")),
        };
        let kept = action
            .due
            .as_ref()
            .filter(|due| !due.withheld())
            .map(|due| &due.due);
        let due_date = kept.map_or("", |due| due.date.as_str());
        let due_time = kept.and_then(|due| due.time.as_deref()).unwrap_or("");
        let due = match due_time {
            "" => due_date.to_owned(),
            time => format!("{due_date}T{time}"),
        };
        Self {
            text: &action.claim.claim.text,
            owner,
            owner_name,
            due,
            due_date,
            due_time,
        }
    }
}
