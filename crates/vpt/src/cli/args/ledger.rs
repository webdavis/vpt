use super::examples;

#[derive(Debug, PartialEq, Eq, clap::Subcommand)]
pub(crate) enum Action {
    /// Show the ledger's folder, database id, write counter and recordings, and this machine's
    /// last write.
    #[command(after_long_help = examples::LEDGER_SHOW)]
    Show,
    /// Trust the ledger as it is now, so this machine's writing commands build on it.
    ///
    /// Refused with exit 2 when there is nothing to accept: the ledger holds this machine's last
    /// write, or this machine has not written to it.
    #[command(after_long_help = examples::LEDGER_ACCEPT)]
    Accept {
        /// Show the ledger and this machine's last write, and change nothing.
        #[arg(long)]
        preview: bool,
    },
}
