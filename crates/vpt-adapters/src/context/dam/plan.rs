use super::*;
use std::collections::BTreeSet;
use vpt_domain::time::UtcInstant;

pub(super) struct Query {
    pub scope: String,
    pub text: String,
}
pub(super) struct Plan {
    pub calendar: Option<Result<Vec<Query>, Failure>>,
    pub tasks: Option<Vec<Query>>,
}

pub(super) fn make(
    command: &[String],
    request: &ContextRequest<'_>,
) -> Result<Plan, ContextConfigError> {
    if (request.window.is_some() || request.tasks) && !crate::arguments::valid(command, 65_536) {
        return Err(ContextConfigError {
            key: "context.dam.command",
        });
    }
    let calendars = scopes(
        request.calendars,
        request.window.is_some(),
        "context.calendars",
    )?;
    let projects = scopes(request.projects, request.tasks, "context.projects")?;
    let calendar = request.window.map(|window| {
        if window.end <= window.start {
            return Err(Failure::Invalid);
        }
        // dam dates are local to each event, so include the adjacent UTC days at both boundaries.
        let first = window
            .start
            .secs
            .div_euclid(86_400)
            .checked_sub(1)
            .ok_or(Failure::Capacity)?;
        let last = window
            .end
            .secs
            .checked_sub(1)
            .ok_or(Failure::Capacity)?
            .div_euclid(86_400)
            .checked_add(1)
            .ok_or(Failure::Capacity)?;
        let days = last
            .checked_sub(first)
            .and_then(|n| n.checked_add(1))
            .ok_or(Failure::Capacity)?;
        if days > 4096 || days as usize * calendars.len() > 4096 {
            return Err(Failure::Capacity);
        }
        let mut queries = Vec::new();
        let mut remaining = 1_048_576usize;
        for scope in &calendars {
            for day in first..=last {
                remaining = remaining
                    .checked_sub(scope.len() * 2 + 64)
                    .ok_or(Failure::Capacity)?;
                let secs = day.checked_mul(86_400).ok_or(Failure::Capacity)?;
                let date = UtcInstant { secs }.rfc3339();
                let date = date
                    .get(..10)
                    .filter(|s| s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-')
                    .ok_or(Failure::Invalid)?;
                queries.push(Query {
                    scope: scope.clone(),
                    text: format!("kind:event & path:{scope} & start:{date}"),
                });
            }
        }
        Ok(queries)
    });
    let tasks = request.tasks.then(|| {
        projects
            .into_iter()
            .map(|scope| Query {
                text: format!("kind:task & path:{scope} & !done"),
                scope,
            })
            .collect()
    });
    Ok(Plan { calendar, tasks })
}

fn scopes(
    values: &[String],
    requested: bool,
    key: &'static str,
) -> Result<Vec<String>, ContextConfigError> {
    if !requested {
        return Ok(vec![]);
    }
    let mut bytes = 65_536usize;
    if values.is_empty() || values.len() > 256 {
        return Err(ContextConfigError { key });
    }
    let mut unique = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        bytes = bytes
            .checked_sub(value.len())
            .ok_or(ContextConfigError { key })?;
        if value.is_empty()
            || value
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || "&|!()".contains(c))
        {
            return Err(ContextConfigError { key });
        }
        let path = value.trim_matches('/');
        if !path.is_empty()
            && path
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == "..")
        {
            return Err(ContextConfigError { key });
        }
        let path = format!("{path}/");
        if unique.insert(path.clone()) {
            result.push(path);
        }
    }
    Ok(result)
}
