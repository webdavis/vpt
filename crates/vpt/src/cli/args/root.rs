use super::{PathBuf, examples, verbs::Command};

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
    /// Print the result as JSON.
    #[arg(long, global = true, conflicts_with_all = ["toon", "yaml"])]
    pub json: bool,
    /// Print the result as TOON.
    #[arg(long, global = true, conflicts_with = "yaml")]
    pub toon: bool,
    /// Print the result as YAML.
    #[arg(long, global = true)]
    pub yaml: bool,
    /// Report progress as JSON lines on stderr.
    #[arg(long, global = true)]
    pub progress: bool,
    /// Read this configuration file.
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
}
