use super::*;
use std::collections::BTreeSet;

pub(super) struct Query {
    pub scope: String,
    pub text: String,
}
impl Query {
    pub fn args(&self) -> [String; 4] {
        ["ls", &self.text, "--json", "--no-pull"].map(String::from)
    }
}
pub(super) struct Agenda {
    pub scopes: Vec<String>,
    pub args: Vec<String>,
}
pub(super) struct Plan {
    pub calendar: Option<Result<Agenda, Failure>>,
    pub tasks: Option<Vec<Query>>,
}

pub(super) fn make(
    command: &[String],
    max_age: &[String],
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
    if !max_age.iter().all(|bound| fresh_bound(bound)) {
        return Err(ContextConfigError {
            key: "context.dam.max_age",
        });
    }
    let calendar = request.window.map(|window| {
        if window.end <= window.start {
            return Err(Failure::Invalid);
        }
        let query = calendars
            .iter()
            .map(|scope| format!("path:{scope}"))
            .collect::<Vec<_>>()
            .join(" | ");
        let args = [
            "agenda",
            &query,
            "--json",
            "--no-pull",
            "--from",
            &window.start.rfc3339(),
            "--to",
            &window.end.rfc3339(),
        ];
        let mut args = args.map(String::from).to_vec();
        for bound in max_age {
            args.extend(["--max-age".into(), bound.clone()]);
        }
        Ok(Agenda {
            args,
            scopes: calendars,
        })
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

fn fresh_bound(bound: &str) -> bool {
    bound.len() <= 256
        && !bound.starts_with('-')
        && !bound.chars().any(|c| c.is_whitespace() || c.is_control())
        && bound
            .split_once('=')
            .is_some_and(|(remote, age)| !remote.is_empty() && !age.is_empty())
}
