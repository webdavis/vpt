use super::{OccasionTime, TimeError};
use crate::time::{Civil, UtcOffset};

pub(super) fn parse(text: &str) -> Result<OccasionTime, TimeError> {
    if text.len() > 65_536 {
        return Err(TimeError::CapacityExceeded);
    }
    let bytes = text.as_bytes();
    if bytes.len() < 20
        || !text.is_ascii()
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err(TimeError::Invalid);
    }
    let field = |start, end| digits(&text[start..end]);
    let second = field(17, 19)?;
    if second > 60 {
        return Err(TimeError::Invalid);
    }
    let civil = Civil {
        year: i64::from(field(0, 4)?),
        month: field(5, 7)?,
        day: field(8, 10)?,
        hour: field(11, 13)?,
        minute: field(14, 16)?,
        second: second.min(59),
    };
    if !civil.is_valid() {
        return Err(TimeError::Invalid);
    }
    let (fraction, zone) = match text[19..].strip_prefix('.') {
        Some(tail) => {
            let end = tail.bytes().take_while(u8::is_ascii_digit).count();
            if end == 0 {
                return Err(TimeError::Invalid);
            }
            (tail[..end].trim_end_matches('0'), &tail[end..])
        }
        None => ("", &text[19..]),
    };
    let offset = offset(zone)?;
    let instant = civil.instant(offset).ok_or(TimeError::Invalid)?;
    if second == 60 {
        let utc = instant.civil(UtcOffset { secs: 0 });
        let next = crate::time::UtcInstant {
            secs: instant.secs + 1,
        }
        .civil(UtcOffset { secs: 0 });
        if utc.hour != 23 || utc.minute != 59 || next.day != 1 {
            return Err(TimeError::Invalid);
        }
    }
    let zone = match zone {
        "z" | "Z" | "+00:00" => "Z",
        other => other,
    };
    let fraction_text = if fraction.is_empty() {
        String::new()
    } else {
        format!(".{fraction}")
    };
    let canonical = format!("{}T{}{}{}", &text[..10], &text[11..19], fraction_text, zone);
    Ok(OccasionTime {
        text: canonical,
        seconds: instant.secs,
        leap: second == 60,
        fraction: fraction.into(),
    })
}

fn digits(text: &str) -> Result<u32, TimeError> {
    if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse().map_err(|_| TimeError::Invalid)
    } else {
        Err(TimeError::Invalid)
    }
}

fn offset(text: &str) -> Result<UtcOffset, TimeError> {
    if matches!(text, "Z" | "z") {
        return Ok(UtcOffset { secs: 0 });
    }
    let bytes = text.as_bytes();
    if bytes.len() != 6 || !matches!(bytes[0], b'+' | b'-') || bytes[3] != b':' {
        return Err(TimeError::Invalid);
    }
    let hours = digits(&text[1..3])?;
    let minutes = digits(&text[4..6])?;
    if hours >= 24 || minutes >= 60 {
        return Err(TimeError::Invalid);
    }
    let secs = (hours * 3600 + minutes * 60) as i32;
    Ok(UtcOffset {
        secs: if bytes[0] == b'-' { -secs } else { secs },
    })
}
