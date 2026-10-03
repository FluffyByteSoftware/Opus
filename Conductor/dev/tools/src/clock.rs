//! File:       Opus/Conductor/dev/tools/src/clock.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! UTC wall-clock time, broken into a date and a time of day.  The standard
//! library only gives us seconds since 1970, so the calendar math is done
//! here by hand rather than pulling in a crate for it.  All time in Opus is
//! UTC, so there is no time zone anywhere in this file.

use std::time::{SystemTime, UNIX_EPOCH};

/// One instant in UTC, split into the pieces a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utc {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Utc {
    /// The current time.
    pub fn now() -> Utc {
        // A system clock set before 1970 is broken; it reads as 1970.
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Utc::from_unix(since_epoch.as_secs() as i64)
    }

    /// Builds a `Utc` from seconds since 1970-01-01 00:00:00 UTC.
    pub fn from_unix(seconds: i64) -> Utc {
        let days = seconds.div_euclid(86_400);
        let in_day = seconds.rem_euclid(86_400) as u32;
        let (year, month, day) = civil_from_days(days);
        Utc {
            year,
            month,
            day,
            hour: in_day / 3_600,
            minute: (in_day / 60) % 60,
            second: in_day % 60,
        }
    }

    /// The date on its own, for telling one day from the next.
    pub fn date(&self) -> (i64, u32, u32) {
        (self.year, self.month, self.day)
    }

    /// The date the way a log file is named: `2026_09_28`.  Sorts in order
    /// when the files are listed by name.
    pub fn file_stamp(&self) -> String {
        format!("{:04}_{:02}_{:02}", self.year, self.month, self.day)
    }

    /// The date and time the way a log line shows it:
    /// `12:08:45 PM - 09-28-26 Z`.  Twelve-hour clock, two-digit year, and
    /// the `Z` because every time a person sees ends with one.
    pub fn line_stamp(&self) -> String {
        let (hour, half) = match self.hour {
            0 => (12, "AM"),
            1..=11 => (self.hour, "AM"),
            12 => (12, "PM"),
            _ => (self.hour - 12, "PM"),
        };
        format!(
            "{:02}:{:02}:{:02} {} - {:02}-{:02}-{:02} Z",
            hour,
            self.minute,
            self.second,
            half,
            self.month,
            self.day,
            self.year.rem_euclid(100)
        )
    }
}

/// Turns a count of days since 1970-01-01 into a (year, month, day).
///
/// This is Howard Hinnant's "days to civil" algorithm.  The trick is to
/// start the year on March 1st so that the leap day is the last day of the
/// year and stops getting in the way, then shift back at the end.  It is
/// exact for every date the i64 can hold.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01, the start of a 400-year cycle.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097) as u64;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 } as u32;
    let year = year_of_era as i64 + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_new_years_1970() {
        let t = Utc::from_unix(0);
        assert_eq!(t.date(), (1970, 1, 1));
        assert_eq!((t.hour, t.minute, t.second), (0, 0, 0));
    }

    #[test]
    fn leap_day_2000() {
        // 2000-02-29 12:00:00 UTC
        assert_eq!(Utc::from_unix(951_825_600).date(), (2000, 2, 29));
    }

    #[test]
    fn last_second_before_midnight_stays_on_its_day() {
        // 2026-09-28 23:59:59 UTC
        let t = Utc::from_unix(1_790_639_999);
        assert_eq!(t.date(), (2026, 9, 28));
        assert_eq!(t.line_stamp(), "11:59:59 PM - 09-28-26 Z");
        // One second later is a new day, and a new log file.
        let next = Utc::from_unix(1_790_640_000);
        assert_eq!(next.date(), (2026, 9, 29));
        assert_eq!(next.line_stamp(), "12:00:00 AM - 09-29-26 Z");
        assert_eq!(next.file_stamp(), "2026_09_29");
    }

    #[test]
    fn twelve_hour_clock_edges() {
        let noon = Utc::from_unix(1_790_596_800); // 2026-09-28 12:00:00
        assert!(noon.line_stamp().starts_with("12:00:00 PM"));
        let one_pm = Utc::from_unix(1_790_600_400);
        assert!(one_pm.line_stamp().starts_with("01:00:00 PM"));
        let one_am = Utc::from_unix(1_790_557_200);
        assert!(one_am.line_stamp().starts_with("01:00:00 AM"));
    }
}
