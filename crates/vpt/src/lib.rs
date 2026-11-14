mod cli;
mod commands;
mod compose;
mod dispatch;
mod doctor;
mod documents;
#[cfg(test)]
mod test_temp;

use cli::args::parse;
use cli::format::Format;
use cli::output::{Outcome, emit};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub fn run() -> ! {
    vpt_adapters::install_interrupt_handlers();
    let args: Vec<_> = std::env::args_os().collect();
    let named: Vec<_> = Format::FLAGS
        .into_iter()
        .filter(|(flag, _)| args.iter().skip(1).any(|word| word == flag))
        .collect();
    let format = match named.as_slice() {
        [(_, format)] => Some(*format),
        _ => None,
    };
    let code = match parse(args) {
        Ok(invocation) => dispatch::run(&invocation),
        Err(error) => emit(
            Outcome::ParseFailure(ErrorDocument::new(ErrorKind::Usage, error.0)),
            format,
        ),
    };
    std::process::exit(code);
}
