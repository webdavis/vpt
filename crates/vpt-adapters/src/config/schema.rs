mod dynamic;
mod types;

pub(super) use dynamic::table_is_declared;
pub use dynamic::{DYNAMIC_TABLES, DynamicTable, spec_for};
pub use types::{KeySpec, Kind, STORE_KEYS};

mod analysis;
mod brief;
mod engines;
mod notes;
mod operations;
mod sharing;
mod storage;

pub fn keys() -> impl Iterator<Item = &'static KeySpec> {
    [
        storage::KEYS,
        engines::KEYS,
        notes::KEYS,
        analysis::KEYS,
        brief::KEYS,
        sharing::KEYS,
        operations::KEYS,
    ]
    .into_iter()
    .flat_map(|keys| keys.iter())
}
