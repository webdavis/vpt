use super::Failure;
use jiff::{Timestamp, Zoned, civil::Date, tz::TimeZone};
use vpt_domain::occasion::time::OccasionTime;

pub(super) fn parse(text: &str, zone: Option<&str>) -> Result<(OccasionTime, Timestamp), Failure> {
    let (text, timestamp) = if text.contains('[') {
        let value: Zoned = text.parse().map_err(|_| Failure::Invalid)?;
        let formatted = value.to_string();
        let rfc = formatted
            .split_once('[')
            .ok_or(Failure::Invalid)?
            .0
            .to_owned();
        (rfc, value.timestamp())
    } else if text.len() == 10 {
        let date: Date = text.parse().map_err(|_| Failure::Invalid)?;
        let zone = match zone {
            Some(name) => TimeZone::get(name).map_err(|_| Failure::Invalid)?,
            None => TimeZone::try_system().map_err(|_| Failure::Invalid)?,
        };
        let value = date.to_zoned(zone).map_err(|_| Failure::Invalid)?;
        let formatted = value.to_string();
        let rfc = formatted
            .split_once('[')
            .ok_or(Failure::Invalid)?
            .0
            .to_owned();
        (rfc, value.timestamp())
    } else {
        (
            text.to_owned(),
            text.parse::<Timestamp>().map_err(|_| Failure::Invalid)?,
        )
    };
    Ok((
        OccasionTime::parse(&text).map_err(|_| Failure::Invalid)?,
        timestamp,
    ))
}
