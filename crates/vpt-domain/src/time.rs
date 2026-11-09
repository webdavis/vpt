#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcInstant {
    pub secs: i64,
}

const NANOS_PER_SECOND: i128 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UtcOffset {
    pub secs: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileTime {
    pub secs: i64,
    pub nanos: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Civil {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl UtcInstant {
    pub fn rfc3339(self) -> String {
        let civil = self.civil(UtcOffset { secs: 0 });
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            civil.year, civil.month, civil.day, civil.hour, civil.minute, civil.second
        )
    }

    pub fn rfc3339_with(self, offset: UtcOffset) -> String {
        let civil = self.civil(offset);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
            civil.year,
            civil.month,
            civil.day,
            civil.hour,
            civil.minute,
            civil.second,
            offset.label()
        )
    }

    pub fn civil(self, offset: UtcOffset) -> Civil {
        let local = i128::from(self.secs) + i128::from(offset.secs);
        let days = local.div_euclid(86_400);
        let seconds = local.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Civil {
            year,
            month,
            day,
            hour: (seconds / 3_600) as u32,
            minute: ((seconds % 3_600) / 60) as u32,
            second: (seconds % 60) as u32,
        }
    }
}

impl UtcOffset {
    pub fn label(self) -> String {
        let sign = if self.secs < 0 { '-' } else { '+' };
        let magnitude = self.secs.unsigned_abs();
        format!(
            "{sign}{:02}:{:02}",
            magnitude / 3_600,
            (magnitude % 3_600) / 60
        )
    }
}

impl Civil {
    pub fn instant(self, offset: UtcOffset) -> Option<UtcInstant> {
        let seconds =
            i128::from(self.hour) * 3_600 + i128::from(self.minute) * 60 + i128::from(self.second);
        let local = days_from_civil(self.year, self.month, self.day) * 86_400 + seconds;
        i64::try_from(local - i128::from(offset.secs))
            .ok()
            .map(|secs| UtcInstant { secs })
    }

    pub fn is_valid(self) -> bool {
        (1..=12).contains(&self.month)
            && (1..=days_in_month(self.year, self.month)).contains(&self.day)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
    }
}

impl FileTime {
    pub fn age_secs(self, now: UtcInstant) -> i64 {
        let elapsed = (i128::from(now.secs) - i128::from(self.secs)) * NANOS_PER_SECOND
            - i128::from(self.nanos);
        elapsed
            .div_euclid(NANOS_PER_SECOND)
            .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        _ => 28,
    }
}

// Gregorian era arithmetic from Howard Hinnant's date algorithms.
fn civil_from_days(days: i128) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (
        i64::try_from(year).expect("an i64 instant spans fewer than i64 years"),
        m,
        d,
    )
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i128 {
    let y = i128::from(year) - i128::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i128::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i128::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests;
