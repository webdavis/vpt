use crate::ports::StoreEntry;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreInventory {
    pub files: u64,
    pub bytes: u64,
    pub oldest: Option<FileTime>,
    pub newest: Option<FileTime>,
}

pub fn inventory(entries: &[StoreEntry]) -> Option<StoreInventory> {
    let key = |time: &FileTime| (time.secs, time.nanos);
    Some(StoreInventory {
        files: u64::try_from(entries.len()).ok()?,
        bytes: entries
            .iter()
            .try_fold(0_u64, |total, entry| total.checked_add(entry.size))?,
        oldest: entries.iter().map(|entry| entry.mtime).min_by_key(key),
        newest: entries.iter().map(|entry| entry.mtime).max_by_key(key),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(size: u64, secs: i64, nanos: u32) -> StoreEntry {
        StoreEntry {
            path: PathBuf::from(format!("/owned/{secs}-{nanos}")),
            size,
            mtime: FileTime { secs, nanos },
        }
    }

    #[test]
    fn empty_inventory_has_zero_counts_and_no_extremes() {
        assert_eq!(
            inventory(&[]),
            Some(StoreInventory {
                files: 0,
                bytes: 0,
                oldest: None,
                newest: None,
            })
        );
    }

    #[test]
    fn inventory_sums_sizes_and_uses_nanosecond_extremes() {
        assert_eq!(
            inventory(&[entry(10, 300, 1), entry(5, 100, 9), entry(1, 100, 2)]),
            Some(StoreInventory {
                files: 3,
                bytes: 16,
                oldest: Some(FileTime {
                    secs: 100,
                    nanos: 2
                }),
                newest: Some(FileTime {
                    secs: 300,
                    nanos: 1
                }),
            })
        );
    }

    #[test]
    fn inventory_refuses_unrepresentable_total() {
        assert_eq!(inventory(&[entry(u64::MAX, 1, 0), entry(1, 2, 0)]), None);
    }
}
