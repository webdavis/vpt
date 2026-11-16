use super::{Format, root::Cli};
use clap::{Arg, Command, CommandFactory};
use serde_json::{Value, json};

pub(super) fn rendered(words: &[String]) -> Option<Result<String, &'static str>> {
    let probe = Cli::command()
        .disable_help_subcommand(true)
        .subcommand(Command::new("help").arg(Arg::new("command").num_args(0..)));
    let matches = probe.try_get_matches_from(words).ok()?;
    let format = *matches.get_one::<Format>("output")?;
    let names = matches
        .subcommand_matches("help")?
        .get_many::<String>("command");
    let mut root = Cli::command();
    root.build();
    let mut path = vec![root.get_name().to_owned()];
    let mut command = &mut root;
    for name in names.into_iter().flatten() {
        match command.find_subcommand_mut(name) {
            Some(subcommand) => command = subcommand,
            None => return Some(Err("unknown command")),
        }
        path.push(command.get_name().to_owned());
    }
    Some(Ok(format.render(&document(command, path))))
}

fn document(command: &mut Command, path: Vec<String>) -> Value {
    let visible = || command.get_arguments().filter(|arg| !arg.is_hide_set());
    let arguments: Vec<Value> = visible()
        .filter(|arg| arg.is_positional())
        .map(argument)
        .collect();
    let options: Vec<Value> = visible()
        .filter(|arg| !arg.is_positional())
        .map(option)
        .collect();
    let subcommands: Vec<Value> = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .map(|subcommand| json!({"name": subcommand.get_name(), "about": text(subcommand.get_about())}))
        .collect();
    let examples = examples(command.get_after_long_help().map(ToString::to_string));
    let usage = command.render_usage().to_string();
    vpt_protocol::result::document(
        "help",
        json!({
            "name": command.get_name(),
            "path": path,
            "about": text(command.get_about()),
            "usage": usage.trim_start_matches("Usage: "),
            "arguments": arguments,
            "options": options,
            "subcommands": subcommands,
            "examples": examples,
        }),
    )
}

fn argument(arg: &Arg) -> Value {
    json!({
        "name": value_name(arg),
        "required": arg.is_required_set(),
        "multiple": arg.get_num_args().is_some_and(|count| count.max_values() > 1),
        "values": values(arg),
        "help": text(arg.get_help()),
    })
}

fn option(arg: &Arg) -> Value {
    let takes_value = arg.get_action().takes_values();
    json!({
        "long": arg.get_long().map(|long| format!("--{long}")),
        "short": arg.get_short().map(|short| format!("-{short}")),
        "value_name": takes_value.then(|| value_name(arg)),
        "values": if takes_value { values(arg) } else { vec![] },
        "global": arg.is_global_set(),
        "help": text(arg.get_help()),
    })
}

fn value_name(arg: &Arg) -> String {
    arg.get_value_names().and_then(<[_]>::first).map_or_else(
        || arg.get_id().to_string().to_uppercase(),
        ToString::to_string,
    )
}

fn values(arg: &Arg) -> Vec<String> {
    arg.get_possible_values()
        .iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| value.get_name().to_owned())
        .collect()
}

fn text(styled: Option<&clap::builder::StyledStr>) -> Option<String> {
    styled.map(ToString::to_string)
}

fn examples(after: Option<String>) -> Vec<String> {
    let after = after.unwrap_or_default();
    let Some((_, lines)) = after.split_once("Examples:\n") else {
        return vec![];
    };
    lines
        .lines()
        .take_while(|line| line.starts_with("  "))
        .map(|line| line.trim().to_owned())
        .collect()
}
