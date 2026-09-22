mod load;
mod render;
mod schema;

pub use load::{CONFIG_VERSION, ConfigError, get, load_file, load_text};
pub use render::template;
pub use schema::{DYNAMIC_TABLES, DynamicTable, KEYS, KeySpec, Kind, STORE_KEYS, spec_for};
