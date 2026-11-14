use vpt_domain::occasion::time::OccasionTime;
use vpt_domain::time::UtcInstant;

pub(super) fn time(text: &str) -> Result<OccasionTime, String> {
    OccasionTime::parse(text).map_err(|_| "expected an RFC 3339 time".into())
}

pub(super) fn instant(text: &str) -> Result<UtcInstant, String> {
    time(text).map(|time| time.unix_second())
}

pub(super) fn positive(text: &str) -> Result<usize, String> {
    text.parse()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| "expected a positive whole number".into())
}
