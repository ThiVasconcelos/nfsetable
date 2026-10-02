//! Parsing and formatting of field values: money as integer cents, dates as ISO `yyyy-mm-dd`.

use regex::Regex;
use std::sync::LazyLock;

/// A money amount found inside a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoneyMatch {
    /// Byte offset of the first char of the match (including sign and "R$").
    pub start: usize,
    /// Byte offset one past the last char of the match.
    pub end: usize,
    /// Parsed amount in cents.
    pub cents: i64,
}

/// "R$ 1.234,56", "1234,56", "-R$ 10,00", "R$ -10,00": an optional sign glued to "R$" or to the
/// digits, whole reais with dot thousands separators in groups of three (or none) and two cents
/// digits after a comma.
static MONEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<sign>[-\x{2212}])?              # -R$ 10,00 (also the U+2212 minus)
        (?P<currency>R\$\s*)?               # R$, spaces optional
        (?P<sign_after>[-\x{2212}])?        # R$ -10,00
        (?P<units>\d{1,3}(?:\.\d{3})+|\d+)  # 1.234 or 1234
        ,(?P<cents>\d{2})                    # ,56
        ",
    )
    .expect("valid money regex")
});

/// Finds every money amount in `s` ("R$ 1.234,56", "R$1.234,56", "1.234,56", "1234,56",
/// "-R$ 10,00", "R$ -10,00"), in order of appearance.
pub fn find_money(s: &str) -> Vec<MoneyMatch> {
    let mut out = Vec::new();
    for caps in MONEY.captures_iter(s) {
        let whole = caps.get(0).expect("group 0 always exists");
        // Reject matches glued to other digits ("12.34,56", "1234,567").
        let starts_with_digit = caps.name("sign").is_none()
            && caps.name("currency").is_none()
            && caps.name("sign_after").is_none();
        let before = s[..whole.start()].chars().next_back();
        let after = s[whole.end()..].chars().next();
        if starts_with_digit
            && matches!(before, Some(c) if c.is_ascii_digit() || c == '.' || c == ',')
        {
            continue;
        }
        if matches!(after, Some(c) if c.is_ascii_digit()) {
            continue;
        }
        let integer: String = caps["units"]
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        let Ok(units) = integer.parse::<i64>() else {
            continue;
        };
        let Ok(fraction) = caps["cents"].parse::<i64>() else {
            continue;
        };
        let Some(abs) = units.checked_mul(100).and_then(|v| v.checked_add(fraction)) else {
            continue;
        };
        let negative = caps.name("sign").is_some() || caps.name("sign_after").is_some();
        out.push(MoneyMatch {
            start: whole.start(),
            end: whole.end(),
            cents: if negative { -abs } else { abs },
        });
    }
    out
}

/// Parses the first money amount in `s`, in cents.
pub fn parse_money(s: &str) -> Option<i64> {
    find_money(s).first().map(|m| m.cents)
}

/// "05/03/2026": day and month with two digits, year with four.
static DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<day>\d{2})/(?P<month>\d{2})/(?P<year>\d{4})").expect("valid date regex")
});

/// Days of `month` (1 to 12) in `year`, with the Gregorian leap years.
pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A date found inside a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateMatch {
    /// Byte offset of the first char of the match.
    pub start: usize,
    /// Byte offset one past the last char of the match.
    pub end: usize,
    /// ISO `yyyy-mm-dd`.
    pub iso: String,
}

/// Finds every valid `dd/mm/aaaa` date in `s`, in order of appearance: real calendar days only,
/// and not glued to other digits ("105/03/2026", "05/03/20261").
pub fn find_dates(s: &str) -> Vec<DateMatch> {
    DATE.captures_iter(s)
        .filter_map(|caps| {
            let whole = caps.get(0)?;
            let glued = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit() || c == '/');
            if glued(s[..whole.start()].chars().next_back())
                || glued(s[whole.end()..].chars().next())
            {
                return None;
            }
            let day: u32 = caps["day"].parse().ok()?;
            let month: u32 = caps["month"].parse().ok()?;
            let year: i32 = caps["year"].parse().ok()?;
            ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)).then(
                || DateMatch {
                    start: whole.start(),
                    end: whole.end(),
                    iso: format!("{year:04}-{month:02}-{day:02}"),
                },
            )
        })
        .collect()
}

/// Parses the first `dd/mm/aaaa` date in `s` as ISO `yyyy-mm-dd`.
pub fn parse_date(s: &str) -> Option<String> {
    find_dates(s).into_iter().next().map(|d| d.iso)
}

/// Formats cents with a decimal comma and no thousands separator: `123456` -> `"1234,56"`.
pub fn format_decimal(cents: i64) -> String {
    format_decimal_with(cents, ',')
}

/// Formats cents with `separator` before the cents and no thousands separator: `'.'` gives
/// `"1234.56"` (SQL, spreadsheets).
pub fn format_decimal_with(cents: i64, separator: char) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}{separator}{:02}", abs / 100, abs % 100)
}

/// Formats cents as Brazilian currency: `123456` -> `"R$ 1.234,56"`, `-1050` -> `"-R$ 10,50"`.
pub fn format_brl(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let units = (abs / 100).to_string();
    let mut grouped = String::with_capacity(units.len() + units.len() / 3);
    for (i, c) in units.chars().enumerate() {
        if i > 0 && (units.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!("{sign}R$ {grouped},{:02}", abs % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_money_variants() {
        assert_eq!(parse_money("R$ 1.234,56"), Some(123_456));
        assert_eq!(parse_money("R$1.234,56"), Some(123_456));
        assert_eq!(parse_money("1.234,56"), Some(123_456));
        assert_eq!(parse_money("1234,56"), Some(123_456));
        assert_eq!(parse_money("-R$ 10,00"), Some(-1_000));
        assert_eq!(parse_money("R$ -10,00"), Some(-1_000));
        assert_eq!(parse_money("R$ 0,00"), Some(0));
        assert_eq!(parse_money("\u{2212}R$ 10,00"), Some(-1_000));
        // A dash separated by a space is not a sign.
        assert_eq!(parse_money("- R$ 845,10"), Some(84_510));
        // Rejected: bad grouping, a third cents digit, one cents digit, the US format, no cents.
        for text in [
            "sem valor",
            "1234,567",
            "12.34,56",
            "1.2345,67",
            "1234,5",
            "1,234.56",
            "R$ 10",
        ] {
            assert_eq!(parse_money(text), None, "{text}");
        }
    }

    #[test]
    fn finds_all_amounts_in_order() {
        let found = find_money("R$3.210,98 R$0,00");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].cents, 321_098);
        assert_eq!((found[0].start, found[0].end), (0, 10));
        assert_eq!(found[1].cents, 0);
    }

    #[test]
    fn parses_dates() {
        let accepted = [
            ("Emissão: 05/03/2026", "2026-03-05"),
            ("29/02/2024", "2024-02-29"),
            ("31/12/2025", "2025-12-31"),
            ("01/01/2026 10:30", "2026-01-01"),
            ("Data:05/03/2026.", "2026-03-05"),
        ];
        for (text, iso) in accepted {
            assert_eq!(parse_date(text).as_deref(), Some(iso), "{text}");
        }
        // Rejected: impossible days, a non-leap 29/02, digits glued before or after, short parts.
        for text in [
            "99/99/2026",
            "31/02/2026",
            "29/02/2025",
            "31/04/2026",
            "05/03/20261",
            "105/03/2026",
            "5/3/2026",
        ] {
            assert_eq!(parse_date(text), None, "{text}");
        }
    }

    #[test]
    fn formats_amounts() {
        assert_eq!(format_decimal(123_456), "1234,56");
        assert_eq!(format_decimal(-1_050), "-10,50");
        assert_eq!(format_decimal(0), "0,00");
        assert_eq!(format_decimal_with(123_456, '.'), "1234.56");
        assert_eq!(format_decimal_with(-1_050, '.'), "-10.50");
        assert_eq!(format_decimal_with(5, '.'), "0.05");
        assert_eq!(format_decimal_with(i64::MIN, '.'), "-92233720368547758.08");
        assert_eq!(format_brl(123_456), "R$ 1.234,56");
        assert_eq!(format_brl(84_510), "R$ 845,10");
        assert_eq!(format_brl(-1_050), "-R$ 10,50");
        assert_eq!(format_brl(123_456_789), "R$ 1.234.567,89");
    }
}
