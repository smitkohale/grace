//! Time helpers. Timestamps stay RFC 3339 strings everywhere; this module turns
//! them into Unix milliseconds when Grace needs to compare or sort them.

use std::time::{SystemTime, UNIX_EPOCH};

/// Parses `YYYY-MM-DDTHH:MM:SS[.fff…](Z|±HH:MM)` into Unix milliseconds.
pub fn parse_rfc3339_ms(text: &str) -> Option<i64> {
    let b = text.trim().as_bytes();
    if b.len() < 20 {
        return None;
    }
    let num = |range: std::ops::Range<usize>| -> Option<i64> {
        b.get(range)?.iter().try_fold(0i64, |acc, &c| {
            c.is_ascii_digit().then(|| acc * 10 + i64::from(c - b'0'))
        })
    };
    let separators_ok = b[4] == b'-'
        && b[7] == b'-'
        && matches!(b[10], b'T' | b't' | b' ')
        && b[13] == b':'
        && b[16] == b':';
    if !separators_ok {
        return None;
    }
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let mut i = 19;
    let mut millis = 0i64;
    if b[i] == b'.' {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        let digits = &b[start..i];
        for pos in 0..3 {
            let digit = digits.get(pos).map_or(0, |&c| i64::from(c - b'0'));
            millis = millis * 10 + digit;
        }
    }

    let offset_minutes = match b.get(i)? {
        b'Z' | b'z' if i + 1 == b.len() => 0,
        &sign @ (b'+' | b'-') if b.len() == i + 6 && b[i + 3] == b':' => {
            let minutes = num(i + 1..i + 3)? * 60 + num(i + 4..i + 6)?;
            if sign == b'+' { minutes } else { -minutes }
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second - offset_minutes * 60;
    Some(seconds * 1_000 + millis)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The current time in Unix milliseconds.
pub fn now_ms() -> i64 {
    system_time_ms(SystemTime::now()).unwrap_or(0)
}

/// A `SystemTime` in Unix milliseconds, if it's after 1970.
pub fn system_time_ms(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utc_with_millis() {
        assert_eq!(
            parse_rfc3339_ms("2026-11-23T07:45:29.927Z"),
            Some(1_795_419_929_927)
        );
    }

    #[test]
    fn parses_offsets_and_missing_fraction() {
        assert_eq!(
            parse_rfc3339_ms("2026-11-23T13:15:29+05:30"),
            Some(1_795_419_929_000)
        );
        assert_eq!(
            parse_rfc3339_ms("2026-11-23T07:45:29Z"),
            Some(1_795_419_929_000)
        );
        assert_eq!(parse_rfc3339_ms("1970-01-01T00:00:00.5Z"), Some(500));
        assert_eq!(
            parse_rfc3339_ms("2026-11-23T07:45:29.123456789Z"),
            Some(1_795_419_929_123)
        );
    }

    #[test]
    fn rejects_garbage() {
        for text in [
            "nope",
            "",
            "2026-13-01T00:00:00Z",
            "2026-11-23T07:45:29",
            "2026-11-23T07:45:29.Z",
        ] {
            assert_eq!(parse_rfc3339_ms(text), None, "{text}");
        }
    }

    #[test]
    fn now_is_after_2026() {
        assert!(now_ms() > 1_767_225_600_000);
    }
}
