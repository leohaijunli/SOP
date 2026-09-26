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
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
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