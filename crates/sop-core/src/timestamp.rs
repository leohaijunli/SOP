//! Current time as an RFC 3339 UTC timestamp.
//!
//! The record format demands timestamps in UTC with a `Z` suffix. A dependency-free
//! implementation is a civil-from-days conversion (Howard Hinnant's algorithm) applied
//! to the system clock, so there is no `chrono` to pull in for one string.

/// The current time as `YYYY-MM-DDTHH:MM:SSZ` in UTC.
pub fn now_utc_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    rfc3339_from_epoch(secs as i64)
}

/// An RFC 3339 UTC timestamp for a whole number of seconds since the epoch.
pub fn rfc3339_from_epoch(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Convert days since 1970-01-01 to a (year, month, day) civil date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Seconds since the epoch for an RFC 3339 UTC timestamp this build writes
/// (`YYYY-MM-DDTHH:MM:SSZ`), or `None` for a value it does not recognise.
///
/// The tool only ever writes whole seconds with a `Z` suffix, so only that shape is
/// parsed here; a fractional second or a `+00:00` offset is read as unknown. Unknown is
/// the safe answer for a timestamp comparison: nothing overlaps a value we cannot place.
pub fn epoch_seconds(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let digits = |start: usize, end: usize| {
        bytes[start..end]
            .iter()
            .all(|b| b.is_ascii_digit())
            .then(|| {
                bytes[start..end]
                    .iter()
                    .fold(0u64, |n, b| n * 10 + (b - b'0') as u64)
            })
    };
    let year = digits(0, 4)? as i64;
    let month = digits(5, 7)?;
    let day = digits(8, 10)?;
    let hour = digits(11, 13)?;
    let minute = digits(14, 16)?;
    let second = digits(17, 19)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let days = days_from_civil(year, month as u32, day as u32);
    Some(days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64)
}

/// Days since 1970-01-01 for a civil date (the inverse of `civil_from_days`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A human duration for a span of seconds: `45s`, `1m33s`, `2h01m`.
pub fn format_duration(secs: i64) -> String {
    if secs < 0 {
        return format!("-{}", format_duration(-secs));
    }
    let h = secs / 3_600;
    let m = (secs % 3_600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h{m:02}m")
    } else if m > 0 {
        format!("{m}m{s:02}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_1970_01_01() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(1), (1970, 1, 2));
    }

    #[test]
    fn a_timestamp_looks_like_rfc3339_utc() {
        let value = now_utc_rfc3339();
        assert!(value.ends_with('Z'), "{value}");
        assert_eq!(value.len(), 20, "{value}");
        assert_eq!(&value[4..5], "-");
        assert_eq!(&value[10..11], "T");
    }
}
