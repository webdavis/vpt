#[derive(Debug, PartialEq, Eq)]
pub enum DurationError {
    MissingUnit,
    UnknownUnit(char),
    NotAPositiveInteger,
    Overflow,
}

pub fn parse_duration(text: &str) -> Result<u64, DurationError> {
    if text == "0" {
        return Ok(0);
    }
    let Some(unit) = text.chars().last() else {
        return Err(DurationError::NotAPositiveInteger);
    };
    if unit.is_ascii_digit() {
        return Err(DurationError::MissingUnit);
    }
    let digits = &text[..text.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(DurationError::NotAPositiveInteger);
    }
    let count: u64 = digits.parse().map_err(|_| DurationError::Overflow)?;
    if count == 0 {
        return Err(DurationError::NotAPositiveInteger);
    }
    let multiplier = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3_600,
        'd' => 86_400,
        other => return Err(DurationError::UnknownUnit(other)),
    };
    count.checked_mul(multiplier).ok_or(DurationError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_means_never_and_needs_no_unit() {
        assert_eq!(parse_duration("0"), Ok(0));
    }

    #[test]
    fn each_unit_converts_to_seconds() {
        for (text, seconds) in [
            ("45s", 45),
            ("2m", 120),
            ("24h", 86_400),
            ("90d", 7_776_000),
        ] {
            assert_eq!(parse_duration(text), Ok(seconds), "{text}");
        }
    }

    #[test]
    fn a_positive_number_without_a_unit_is_refused() {
        assert_eq!(parse_duration("90"), Err(DurationError::MissingUnit));
    }

    #[test]
    fn an_unknown_unit_is_refused() {
        for unit in ['w', 'S', '日'] {
            assert_eq!(
                parse_duration(&format!("3{unit}")),
                Err(DurationError::UnknownUnit(unit))
            );
        }
    }

    #[test]
    fn malformed_counts_are_refused() {
        for text in [
            "", "s", "+3s", "-3d", " 3s", "3 s", "3s ", "3s\n", "1.5h", "３s", "0d", "00s",
        ] {
            assert_eq!(
                parse_duration(text),
                Err(DurationError::NotAPositiveInteger),
                "{text:?}"
            );
        }
    }

    #[test]
    fn leading_zeroes_do_not_change_a_positive_count() {
        assert_eq!(parse_duration("0003s"), Ok(3));
    }

    #[test]
    fn the_largest_seconds_value_is_accepted() {
        assert_eq!(parse_duration("18446744073709551615s"), Ok(u64::MAX));
    }

    #[test]
    fn a_count_above_64_bits_is_refused() {
        assert_eq!(
            parse_duration("18446744073709551616s"),
            Err(DurationError::Overflow)
        );
    }

    #[test]
    fn multiplication_overflow_is_refused() {
        assert_eq!(
            parse_duration("307445734561825860m"),
            Ok(18_446_744_073_709_551_600)
        );
        assert_eq!(
            parse_duration("307445734561825861m"),
            Err(DurationError::Overflow)
        );
        assert_eq!(
            parse_duration("999999999999999999d"),
            Err(DurationError::Overflow)
        );
    }
}
