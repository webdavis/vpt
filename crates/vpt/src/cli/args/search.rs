use super::values;

#[derive(Debug, PartialEq, Eq, clap::Args)]
pub struct Options {
    #[arg(value_name = "WORD", required = true)]
    pub words: Vec<String>,
    /// Print at most this many matches.
    #[arg(long, default_value_t = 50, value_parser = values::positive)]
    pub limit: usize,
}
