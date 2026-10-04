//! Revenue of the months considered: merge, average ("typical month"), year to date and RBT12.

use super::money::{from_cents, round_money, to_cents};
use super::month::Month;
use crate::model::{MonthAmount, RevenueSummary};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Revenue of the months considered, in reais.
#[derive(Debug, Clone)]
pub(crate) struct Revenue {
    /// Oldest first.
    pub months: BTreeMap<Month, Decimal>,
    pub count: u32,
    pub total: Decimal,
    /// total / count rounded to the cent: the typical month.
    pub average: Decimal,
    /// Revenue of the reference month.
    pub month: Decimal,
    /// Months of the reference year up to the reference month.
    pub year_to_date: Decimal,
    pub rbt12: Decimal,
    /// True when `rbt12` is the average × 12 (fewer than 12 months).
    pub annualized: bool,
}

/// Amounts by month: duplicates are summed, malformed months are left out and negative totals
/// count as zero, each with a pt-BR warning (`invalid` gets the text as typed, `negative` the
/// month as mm/aaaa).
pub(crate) fn sum_by_month(
    entries: &[MonthAmount],
    warnings: &mut Vec<String>,
    invalid: impl Fn(&str) -> String,
    negative: impl Fn(&str) -> String,
) -> BTreeMap<Month, Decimal> {
    let mut months: BTreeMap<Month, Decimal> = BTreeMap::new();
    for entry in entries {
        match Month::parse(&entry.month) {
            Some(month) => *months.entry(month).or_default() += from_cents(entry.cents),
            None => warnings.push(invalid(&entry.month)),
        }
    }
    for (month, amount) in &mut months {
        if *amount < Decimal::ZERO {
            warnings.push(negative(&month.display()));
            *amount = Decimal::ZERO;
        }
    }
    months
}

/// Valid months with their revenue (see `sum_by_month`).
pub(crate) fn collect(
    entries: &[MonthAmount],
    warnings: &mut Vec<String>,
) -> BTreeMap<Month, Decimal> {
    sum_by_month(
        entries,
        warnings,
        |text| format!("Mês inválido ignorado: \"{text}\" (use o formato aaaa-mm)."),
        |month| format!("A receita de {month} é negativa e foi considerada zero."),
    )
}

/// `total` of `months` months scaled to 12 (the average × 12), rounded to the cent.
pub(crate) fn annualize(total: Decimal, months: usize) -> Decimal {
    round_money(total * Decimal::from(12) / Decimal::from(months))
}

/// Summary of the months relative to `reference`. `months` may be empty (all zeros).
pub(crate) fn summarize(months: BTreeMap<Month, Decimal>, reference: Month) -> Revenue {
    let count = u32::try_from(months.len()).unwrap_or(u32::MAX);
    let total: Decimal = months.values().sum();
    let average = if count == 0 {
        Decimal::ZERO
    } else {
        round_money(total / Decimal::from(count))
    };
    let month = months.get(&reference).copied().unwrap_or_default();
    let year_to_date = months
        .iter()
        .filter(|(m, _)| m.year == reference.year && m.month <= reference.month)
        .map(|(_, amount)| *amount)
        .sum();
    let annualized = count < 12;
    let rbt12 = if count == 0 {
        Decimal::ZERO
    } else if annualized {
        // Like a company in its first months: the arithmetic mean times 12 (Resolução CGSN
        // 140/2018, art. 22), from the exact mean rather than the rounded `average`.
        annualize(total, usize::try_from(count).unwrap_or(usize::MAX))
    } else {
        months.values().rev().take(12).sum()
    };
    Revenue {
        months,
        count,
        total,
        average,
        month,
        year_to_date,
        rbt12,
        annualized,
    }
}

impl Revenue {
    pub fn to_contract(&self, year_projection: Decimal) -> RevenueSummary {
        RevenueSummary {
            months: self
                .months
                .iter()
                .map(|(month, amount)| MonthAmount {
                    month: month.iso(),
                    cents: to_cents(*amount),
                })
                .collect(),
            months_count: self.count,
            total_cents: to_cents(self.total),
            average_monthly_cents: to_cents(self.average),
            month_cents: to_cents(self.month),
            year_to_date_cents: to_cents(self.year_to_date),
            year_projection_cents: to_cents(year_projection),
            rbt12_cents: to_cents(self.rbt12),
            rbt12_annualized: self.annualized && self.count > 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::month::month;
    use rust_decimal_macros::dec;

    fn amount(month: &str, cents: i64) -> MonthAmount {
        MonthAmount {
            month: month.to_string(),
            cents,
        }
    }

    #[test]
    fn merges_duplicates_and_skips_bad_months() {
        let mut warnings = Vec::new();
        let months = collect(
            &[
                amount("2026-02", 100_000),
                amount("2026-01", 50_000),
                amount("2026-02", 25_050),
                amount("2026-13", 1),
                amount("2026-03", -10),
            ],
            &mut warnings,
        );
        let got: Vec<(String, Decimal)> = months.iter().map(|(m, a)| (m.iso(), *a)).collect();
        assert_eq!(
            got,
            [
                ("2026-01".to_string(), dec!(500)),
                ("2026-02".to_string(), dec!(1250.50)),
                ("2026-03".to_string(), dec!(0))
            ]
        );
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("2026-13"));
        assert!(warnings[1].contains("03/2026"));
    }

    #[test]
    fn average_ytd_and_rbt12() {
        let mut warnings = Vec::new();
        // 13 months: 2025-09 (1.000) and 2025-10..2026-09 (10.000 each).
        let mut entries = vec![amount("2025-09", 100_000)];
        for m in ["2025-10", "2025-11", "2025-12"] {
            entries.push(amount(m, 1_000_000));
        }
        for m in 1..=9 {
            entries.push(amount(&format!("2026-{m:02}"), 1_000_000));
        }
        let reference = month("2026-06");
        let revenue = summarize(collect(&entries, &mut warnings), reference);
        assert_eq!(revenue.count, 13);
        // (1.000 + 12 × 10.000) / 13 = 9.307,6923… → 9.307,69.
        assert_eq!(revenue.average, dec!(9307.69));
        assert_eq!(revenue.month, dec!(10000));
        // January to June 2026.
        assert_eq!(revenue.year_to_date, dec!(60000));
        // The 12 most recent months: 2025-10..2026-09.
        assert_eq!((revenue.rbt12, revenue.annualized), (dec!(120000), false));
    }

    #[test]
    fn annualized_rbt12_uses_the_exact_mean() {
        let mut warnings = Vec::new();
        let entries = [
            amount("2026-03", 330_000),
            amount("2026-04", 395_000),
            amount("2026-12", 500_000),
        ];
        let reference = month("2026-12");
        let revenue = summarize(collect(&entries, &mut warnings), reference);
        // 12.250 / 3 = 4.083,333…: the typical month shows 4.083,33, but RBT12 is 12.250 × 12 / 3
        // = 49.000,00, not 4.083,33 × 12 = 48.999,96.
        assert_eq!(revenue.average, dec!(4083.33));
        assert_eq!((revenue.rbt12, revenue.annualized), (dec!(49000), true));
    }
}
