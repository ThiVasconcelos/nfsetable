//! Months (`yyyy-mm`) of the revenue and dates (`yyyy-mm-dd`) of the table versions.

use serde::{de, Deserialize, Deserializer};

/// A calendar month. Orders chronologically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Month {
    pub year: i32,
    /// 1 to 12.
    pub month: u32,
}

impl Month {
    /// Parses `yyyy-mm` (surrounding whitespace is ignored).
    pub fn parse(text: &str) -> Option<Month> {
        let text = text.trim();
        let bytes = text.as_bytes();
        if bytes.len() != 7 || bytes[4] != b'-' {
            return None;
        }
        if !bytes[..4].iter().chain(&bytes[5..]).all(u8::is_ascii_digit) {
            return None;
        }
        let year = text[..4].parse().ok()?;
        let month = text[5..].parse().ok()?;
        (1..=12).contains(&month).then_some(Month { year, month })
    }

    /// The month of `yyyy-mm`, or of a date `yyyy-mm-dd` (the day must be two digits; whether it
    /// exists is not checked). Surrounding whitespace is ignored.
    pub fn parse_month_or_date(text: &str) -> Option<Month> {
        let text = text.trim();
        let bytes = text.as_bytes();
        match bytes.len() {
            7 => Month::parse(text),
            10 if text.is_ascii()
                && bytes[7] == b'-'
                && bytes[8..].iter().all(u8::is_ascii_digit) =>
            {
                Month::parse(&text[..7])
            }
            _ => None,
        }
    }

    /// `yyyy-mm`, the contract format.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }

    /// `mm/aaaa`, for pt-BR messages.
    pub fn display(self) -> String {
        format!("{:02}/{:04}", self.month, self.year)
    }

    pub fn first_day(self) -> Date {
        Date {
            year: self.year,
            month: self.month,
            day: 1,
        }
    }

    /// The month `months` later (earlier when negative).
    pub fn plus(self, months: i64) -> Month {
        let index = i64::from(self.year) * 12 + i64::from(self.month) - 1 + months;
        Month {
            year: i32::try_from(index.div_euclid(12)).unwrap_or(i32::MAX),
            month: u32::try_from(index.rem_euclid(12) + 1).unwrap_or(1),
        }
    }

    /// Whole months from `self` to `later` (negative when `later` is earlier).
    pub fn months_until(self, later: Month) -> i64 {
        (i64::from(later.year) - i64::from(self.year)) * 12 + i64::from(later.month)
            - i64::from(self.month)
    }
}

/// A calendar date (`yyyy-mm-dd`), used for the validity of the table versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    /// Parses `yyyy-mm-dd`, checking that the day exists.
    pub fn parse(text: &str) -> Option<Date> {
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        let Month { year, month } = Month::parse(&text[..7])?;
        if !bytes[8..].iter().all(u8::is_ascii_digit) {
            return None;
        }
        let day: u32 = text[8..].parse().ok()?;
        (1..=crate::parse::days_in_month(year, month))
            .contains(&day)
            .then_some(Date { year, month, day })
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}/{:02}/{:04}", self.day, self.month, self.year)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Date::parse(&text)
            .ok_or_else(|| de::Error::custom(format!("data inválida: {text:?} (use AAAA-MM-DD)")))
    }
}

/// `Month::parse` of a month known to be valid (tests).
#[cfg(test)]
pub(crate) fn month(text: &str) -> Month {
    Month::parse(text).expect("valid test month")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_months_across_years() {
        assert_eq!(month("2026-07").plus(12), month("2027-07"));
        assert_eq!(month("2026-01").plus(-1), month("2025-12"));
        assert_eq!(month("2026-03").plus(-15), month("2024-12"));
        assert_eq!(month("2026-12").plus(1), month("2027-01"));
        assert_eq!(month("2026-05").plus(0), month("2026-05"));
    }

    #[test]
    fn parses_months() {
        assert_eq!(
            Month::parse(" 2026-03 "),
            Some(Month {
                year: 2026,
                month: 3
            })
        );
        for bad in [
            "2026-13",
            "2026-00",
            "2026-3",
            "26-03",
            "2026/03",
            "abcd-ef",
            "",
            "2026-03-01",
        ] {
            assert_eq!(Month::parse(bad), None, "{bad}");
        }
        let march = month("2026-03");
        assert_eq!(Month::parse_month_or_date(" 2026-03-15 "), Some(march));
        assert_eq!(Month::parse_month_or_date("2026-03"), Some(march));
        for bad in ["2026-03-1x", "2026-03/15", "2026-3-15", "", "2026-13-01"] {
            assert_eq!(Month::parse_month_or_date(bad), None, "{bad}");
        }
        assert_eq!(march.iso(), "2026-03");
        assert_eq!(march.display(), "03/2026");
        assert_eq!(month("2025-10").months_until(march), 5);
    }

    #[test]
    fn parses_dates() {
        assert!(Date::parse("2025-04-30").is_some());
        assert!(Date::parse("2024-02-29").is_some());
        assert!(Date::parse("2025-02-29").is_none());
        assert!(Date::parse("2025-04-31").is_none());
        assert!(Date::parse("2025-4-30").is_none());
        let first = month("2025-05").first_day();
        assert!(Date::parse("2025-04-30").unwrap() < first);
        assert_eq!(first, Date::parse("2025-05-01").unwrap());
        assert_eq!(first.to_string(), "01/05/2025");
    }
}
