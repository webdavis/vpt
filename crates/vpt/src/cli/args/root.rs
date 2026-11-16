use super::{Format, PathBuf, examples, verbs::Command};

/// Voice Processing Tool: archive, transcribe and analyze voice recordings.
#[derive(clap::Parser)]
#[command(
    name = "vpt",
    arg_required_else_help = true,
    after_long_help = examples::DOCS
)]
pub(super) struct Cli {
    /// Print the vpt and helper versions.
    #[arg(long)]
    pub version: bool,
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Args)]
#[command(next_help_heading = "Global options")]
pub(super) struct Global {
    /// Print the result in this format.
    #[arg(long, short = 'o', global = true, value_name = "FORMAT")]
    pub output: Option<Format>,
    /// Report progress as JSON lines on stderr.
    #[arg(long, global = true)]
    pub progress: bool,
    /// Read this configuration file.
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
}
