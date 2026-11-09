use super::validate::invalid;
use vpt_application::ports::{LedgerError, OccasionRecord};

#[derive(Default)]
pub(crate) struct Budget {
    rows: usize,
    bytes: usize,
}
impl Budget {
    pub(crate) fn admit(&mut self, row: &OccasionRecord) -> Result<(), LedgerError> {
        self.rows += 1;
        let details = &row.details;
        self.bytes += row.key.encoded().len() + details.at.as_str().len() + details.title.len();
        self.bytes += details.tags.iter().map(String::len).sum::<usize>();
        self.bytes += details
            .participants
            .iter()
            .map(|p| p.name.len() + p.email.as_ref().map_or(0, String::len))
            .sum::<usize>();
        self.bytes += row.brief_path.as_ref().map_or(0, |p| p.as_os_str().len());
        if self.rows > 65_536 || self.bytes > 64 * 1024 * 1024 {
            Err(invalid())
        } else {
            Ok(())
        }
    }
}
pub(crate) fn sort(rows: &mut [OccasionRecord]) {
    rows.sort_by(|a, b| {
        a.details
            .at
            .cmp_instant(&b.details.at)
            .then_with(|| a.id.as_str().cmp(b.id.as_str()))
    });
}
