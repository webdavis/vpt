mod archive;
mod arguments;
mod clock;
pub mod markdown_claims;
pub use clock::SystemClock;
pub mod config;
mod contained;
pub mod context;
mod database;
mod engines;
mod events;
mod extract;
mod flag_identity;
pub use engines::{
    AppleEngine, AudioError, CommandEngine, EngineAdapters, GuardedEngines, WhisplyEngine,
    inspect_archived_audio,
};
pub use flag_identity::{diagnostic_flag_id, lexical_flag_id, verification_flag_id};
mod analysis;
mod git_tree;
mod helper;
pub mod known_tags;
pub mod known_terms;
mod ledger;
mod lock;
pub mod note_document;
pub mod note_index;
mod prompt;
pub mod proposal;
mod spawn;
mod stores;
mod symlink;
#[cfg(test)]
mod test_temp;
pub use analysis::CommandAgent;
mod action_command;
pub use action_command::{
    ActionSend, SendOutcome, WithdrawOutcome, action_digest, note_link, send_action,
    withdraw_action,
};
mod transcript_document;
pub mod transcription;
pub mod verification;
mod voice_memos;
pub use extract::{ExtractError, Extracted, extract_audio};
pub use git_tree::enclosing_git_tree;
pub use helper::{BUILT_AGAINST_MAJOR, CALL_DEADLINE, HelperClient, HelperError};
pub use prompt::TtyPrompt;
pub use spawn::{
    GRACE, OUTPUT_LIMIT, Outcome, SpawnError, Status, install_interrupt_handlers, run, run_with_env,
};

pub use archive::{ArchiveReader, ClonefileArchive, STAGING_PREFIX};
pub use config::{RootError, Roots};
pub use contained::{
    Access, AppendEffects, AppendFailure, AppendReceipt, AppendSync, ContainedError, EventLog,
    Kind, LogError, LogInspection, LogRefusal, PrivateStateError, RootDir, Stat, TailRepair,
    leaf_below,
};
pub use events::LoggedEvents;
pub use ledger::{
    MemoryLedger, OpenError, SCHEMA_VERSION, SqliteLedger, grounding_basis_for,
    prepare_brief_artifact, prepare_occasion,
};
pub use lock::{LockError, WriteLock};
pub use stores::{BUFFER, FilesystemStores, digest_open, kept_files};
pub use stores::{ReleaseFileError, ReleaseFiles, ReleaseTarget};
pub use symlink::{
    LinkState, SymlinkError, deploy as deploy_symlink, inspect as inspect_symlink,
    verify as verify_symlink,
};
pub use voice_memos::{APPLE_SUBDIRECTORIES, COPY_DIRECTORY, TitleCopy, VoiceMemosStore};

pub mod redaction;
