use super::{PathBuf, verbs::Command};

/// Voice Processing Tool: archive, transcribe and analyze voice recordings.
#[derive(clap::Parser)]
#[command(name = "vpt")]
pub(super) struct Cli {
    #[command(flatten)]
    pub global: Global,
    /// Print the vpt and helper versions.
    #[arg(long)]
    pub version: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Args)]
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
