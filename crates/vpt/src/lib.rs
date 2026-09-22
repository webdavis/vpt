mod cli;
mod commands;

use cli::args::{Invocation, Verb, parse};
use std::io::Write;

pub fn run() -> ! {
    let code = match parse(std::env::args_os()) {
        Ok(invocation) => dispatch(&invocation),
        Err(error) => diagnostic(&format!("vpt: {}\n{}", error.0, cli::args::USAGE), 2),
    };
    std::process::exit(code);
}

fn dispatch(invocation: &Invocation) -> i32 {
    match &invocation.verb {
        Verb::Version => {
            let text = if invocation.json {
                format!("{}\n", commands::version::document(None))
            } else {
                commands::version::human(None)
            };
            if let Err(error) = std::io::stdout().write_all(text.as_bytes()) {
                return diagnostic(&format!("vpt: write stdout: {error}\n"), 1);
            }
            0
        }
        _ => diagnostic(
            &format!("vpt: verb not implemented yet\n{}", cli::args::USAGE),
            2,
        ),
    }
}

fn diagnostic(text: &str, code: i32) -> i32 {
    if std::io::stderr().write_all(text.as_bytes()).is_ok() {
        code
    } else {
        1
    }
}
