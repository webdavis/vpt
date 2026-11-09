mod load;
mod paths;
mod render;
mod roots;
mod schema;
mod settings;
mod validate;
mod write;

pub use write::FilesystemSetupWriter;

pub use load::{CONFIG_VERSION, ConfigError, get, load_file, load_text};
pub use paths::{config_path, default_state_dir};
pub use render::template;
pub use roots::{RootError, Roots, planned_directory, resolve, resolve_stores, resolve_vault};
pub use schema::{DYNAMIC_TABLES, DynamicTable, KeySpec, Kind, STORE_KEYS, keys, spec_for};
pub use settings::{DEFAULT_HOME, from_table};
