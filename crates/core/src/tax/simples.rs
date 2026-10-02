//! Simples Nacional: Fator R, annex, bracket, effective rate, DAS and its split by tax.

use super::money::{round_money, trunc2};
use super::tables::{AnnexTable, Simples, SimplesBracket};
use crate::model::SimplesActivity;
use rust_decimal::Decimal;

/// Annex of the Simples Nacional used for services (III, or V by the Fator R).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Annex {
    Iii,
    V,
}

impl Annex {
    /// "III" or "V", as in the contract.
    pub fn label(self) -> &'static str {
        match self {
            Annex::Iii => "III",
            Annex::V => "V",
        }
    }

    fn table(self, simples: &Simples) -> &AnnexTable {
        match self {
            Annex::Iii => &simples.annex_iii,
            Annex::V => &simples.annex_v,
        }
    }
}

/// Fator R = FS12 / RBT12, truncated to two decimals like the PGDAS-D (Res. CGSN 140/2018,
/// art. 26): with no payroll it is the "without payroll" value (0,01); with payroll and no
/// revenue, the "without revenue" value (0,28).
pub(crate) fn fator_r(fs12: Decimal, rbt12: Decimal, simples: &Simples) -> Decimal {
    if fs12 <= Decimal::ZERO {
        simples.fator_r_without_payroll
    } else if rbt12 <= Decimal::ZERO {
        simples.fator_r_without_revenue
    } else {
        trunc2(fs12 / rbt12)
    }
}

/// Payroll of 12 months when `pro_labore` and the other `payroll` are paid every month.
pub(crate) fn fs12(pro_labore: Decimal, payroll: Decimal) -> Decimal {
    (pro_labore + payroll) * Decimal::from(12)
}

/// The Fator R of `fs12` over `rbt12`, and the annex it gives to `activity`.
pub(crate) fn fator_r_and_annex(
    activity: SimplesActivity,
    fs12: Decimal,
    rbt12: Decimal,
    simples: &Simples,
) -> (Decimal, Annex) {
    let fator_r = fator_r(fs12, rbt12, simples);
    (fator_r, annex_for(activity, fator_r, simples))
}

/// Annex III for activities always taxed there, or when the Fator R reaches the threshold.
pub(crate) fn annex_for(activity: SimplesActivity, fator_r: Decimal, simples: &Simples) -> Annex {
    match activity {
        SimplesActivity::AnnexIii => Annex::Iii,
        SimplesActivity::FatorR if fator_r >= simples.fator_r_threshold => Annex::Iii,
        SimplesActivity::FatorR => Annex::V,
    }
}

/// DAS of one month.
#[derive(Debug, Clone)]
pub(crate) struct Das {
    pub annex: Annex,
    /// 1 to 6.
    pub bracket: u32,
    pub nominal_rate: Decimal,
    pub deduction: Decimal,
    pub effective_rate: Decimal,
    /// Rounded to the cent.
    pub total: Decimal,
    /// Amount of each tax, rounded to the cent, in the canonical order; sums exactly to `total`.
    pub split: Vec<(String, Decimal)>,
    /// The bracket has no ISS inside the DAS (6th): the ISS is paid directly to the municipality.
    pub iss_outside: bool,
    /// The RBT12 is above the last bracket (the ceiling of the Simples Nacional).
    pub above_ceiling: bool,
}

impl Das {
    /// ISS paid directly to the municipality on `revenue` when the bracket has none inside the
    /// DAS (6th); zero otherwise.
    pub fn outside_iss(&self, revenue: Decimal, iss_rate: Decimal) -> Decimal {
        if self.iss_outside {
            round_money(revenue * iss_rate)
        } else {
            Decimal::ZERO
        }
    }

    /// Amount of one tax inside the DAS (zero when absent).
    pub fn share(&self, tax: &str) -> Decimal {
        self.split
            .iter()
            .find(|(name, _)| name == tax)
            .map_or(Decimal::ZERO, |(_, amount)| *amount)
    }
}

/// DAS of a month with `revenue`, at the rates that `rbt12` gives in `annex`.
///
/// Bracket: the first whose upper limit (inclusive) is not below the RBT12; above the last one,
/// the last one (and `above_ceiling`). Effective rate: (RBT12 × nominal − deduction) / RBT12,
/// with an RBT12 of zero counted as R$ 1,00 (LC 123/2006, art. 18, § 1º-A; Res. CGSN 140/2018,
/// art. 21).
pub(crate) fn das(simples: &Simples, annex: Annex, rbt12: Decimal, revenue: Decimal) -> Das {
    let table = annex.table(simples);
    let found = table
        .brackets
        .iter()
        .position(|bracket| rbt12 <= bracket.rbt12_up_to);
    // `brackets` is never empty (checked when the tables are loaded).
    let bracket = &table.brackets[found.unwrap_or(table.brackets.len() - 1)];
    let base = rbt12.max(Decimal::ONE);
    let effective_rate = (base * bracket.nominal_rate - bracket.deduction) / base;
    let total = round_money(revenue * effective_rate);
    let (split, iss_outside) = split(table, bracket, revenue, effective_rate, total);
    Das {
        annex,
        bracket: bracket.bracket,
        nominal_rate: bracket.nominal_rate,
        deduction: bracket.deduction,
        effective_rate,
        total,
        split,
        iss_outside,
        above_ceiling: found.is_none(),
    }
}

/// Each tax gets revenue × effective rate × its share (LC 123/2006, art. 18, § 1º-B). The
/// effective ISS is capped (5%): above the cap, the ISS stays at the cap and the rest of the
/// effective rate is split by `iss_excess_split`. Each amount is rounded to the cent and the
/// rounding difference goes to the largest share (by its exact amount, i.e. the tax with the
/// largest percentage), so the split adds up to the DAS.
fn split(
    table: &AnnexTable,
    bracket: &SimplesBracket,
    revenue: Decimal,
    effective_rate: Decimal,
    total: Decimal,
) -> (Vec<(String, Decimal)>, bool) {
    let iss_share = bracket
        .split
        .iter()
        .find(|(tax, _)| tax == "iss")
        .and_then(|(_, share)| *share);
    let apply = |shares: &[(String, Option<Decimal>)], rate: Decimal| -> Vec<(String, Decimal)> {
        shares
            .iter()
            .filter_map(|(tax, share)| share.map(|share| (tax.clone(), revenue * rate * share)))
            .collect()
    };
    let mut parts = match iss_share {
        Some(iss) if effective_rate * iss > table.iss_cap => {
            let mut parts = apply(&table.iss_excess_split, effective_rate - table.iss_cap);
            parts.push(("iss".to_string(), revenue * table.iss_cap));
            parts
        }
        _ => apply(&bracket.split, effective_rate),
    };
    let mut largest = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.1 > parts[largest].1 {
            largest = i;
        }
    }
    for part in &mut parts {
        part.1 = round_money(part.1);
    }
    let remainder = total - parts.iter().map(|(_, amount)| *amount).sum::<Decimal>();
    if let Some(part) = parts.get_mut(largest) {
        part.1 += remainder;
    }
    (parts, iss_share.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::tables_for;
    use rust_decimal_macros::dec;

    fn simples() -> &'static Simples {
        tables_for("2026-01").simples
    }

    #[test]
    fn fator_r_edge_cases() {
        let s = simples();
        assert_eq!(fator_r(dec!(0), dec!(0), s), dec!(0.01));
        assert_eq!(fator_r(dec!(0), dec!(100000), s), dec!(0.01));
        assert_eq!(fator_r(dec!(1000), dec!(0), s), dec!(0.28));
        // 27.999 / 100.000 = 0,27999 → truncated to 0,27 (not rounded to 0,28).
        assert_eq!(fator_r(dec!(27999), dec!(100000), s), dec!(0.27));
        assert_eq!(fator_r(dec!(33600), dec!(120000), s), dec!(0.28));
        assert_eq!(annex_for(SimplesActivity::FatorR, dec!(0.27), s), Annex::V);
        assert_eq!(
            annex_for(SimplesActivity::FatorR, dec!(0.28), s),
            Annex::Iii
        );
        assert_eq!(
            annex_for(SimplesActivity::AnnexIii, dec!(0.01), s),
            Annex::Iii
        );
    }

    #[test]
    fn research_example_annex_iii_bracket_2() {
        // Worked example: RBT12 240.000, revenue 20.000 → 7,30% → R$ 1.460,00;
        // IRPJ 58,40, CSLL 51,10, Cofins 205,13, PIS 44,53, CPP 633,64, ISS 467,20.
        let das = das(simples(), Annex::Iii, dec!(240000), dec!(20000));
        assert_eq!(das.bracket, 2);
        assert_eq!(das.effective_rate, dec!(0.073));
        assert_eq!(das.total, dec!(1460.00));
        let amounts: Vec<Decimal> = das.split.iter().map(|(_, a)| *a).collect();
        assert_eq!(
            amounts,
            [
                dec!(58.40),
                dec!(51.10),
                dec!(205.13),
                dec!(44.53),
                dec!(633.64),
                dec!(467.20)
            ]
        );
    }

    #[test]
    fn bracket_limits_are_inclusive() {
        assert_eq!(das(simples(), Annex::Iii, dec!(180000), dec!(1)).bracket, 1);
        assert_eq!(
            das(simples(), Annex::Iii, dec!(180000.01), dec!(1)).bracket,
            2
        );
        let top = das(simples(), Annex::V, dec!(4800000), dec!(1));
        assert_eq!((top.bracket, top.above_ceiling), (6, false));
        let above = das(simples(), Annex::V, dec!(4800000.01), dec!(1));
        assert_eq!((above.bracket, above.above_ceiling), (6, true));
        assert!(above.iss_outside);
    }

    #[test]
    fn zero_rbt12_counts_as_one_real() {
        let das = das(simples(), Annex::V, dec!(0), dec!(1000));
        assert_eq!((das.bracket, das.effective_rate), (1, dec!(0.155)));
        assert_eq!(das.total, dec!(155.00));
    }

    #[test]
    fn rounding_remainder_goes_to_the_largest_share() {
        // Annex V, bracket 1, revenue 333,33: DAS = 51,66615 → 51,67. Shares:
        // IRPJ 12,9165375 → 12,92; CSLL 7,7499225 → 7,75; Cofins 7,28492715 → 7,28;
        // PIS 1,57581758 → 1,58; CPP 14,90568428 → 14,91; ISS 7,233261 → 7,23: sum 51,67.
        let das = das(simples(), Annex::V, dec!(100000), dec!(333.33));
        let sum: Decimal = das.split.iter().map(|(_, a)| *a).sum();
        assert_eq!(sum, das.total);
        assert_eq!(das.share("cpp"), dec!(14.91));
        // Revenue 0,07 at 6%: DAS 0,0042 → 0,00; every share rounds to 0,00.
        let tiny = self::das(simples(), Annex::Iii, dec!(100000), dec!(0.07));
        assert_eq!(tiny.total, dec!(0));
        assert!(tiny.split.iter().all(|(_, a)| a.is_zero()));
        // Revenue 1,00 at 15,5%: DAS 0,155 → 0,16. Shares: IRPJ 0,03875 → 0,04;
        // CSLL 0,02325 → 0,02; Cofins 0,021855 → 0,02; PIS 0,0047275 → 0,00;
        // CPP 0,0447175 → 0,04; ISS 0,0217 → 0,02: sum 0,14. The missing 0,02 goes to CPP,
        // the largest share (28,85%), even though IRPJ also rounds to 0,04.
        let one = self::das(simples(), Annex::V, dec!(100000), dec!(1));
        assert_eq!(one.total, dec!(0.16));
        assert_eq!(one.share("cpp"), dec!(0.06));
        assert_eq!(one.share("irpj"), dec!(0.04));
        let sum: Decimal = one.split.iter().map(|(_, a)| *a).sum();
        assert_eq!(sum, dec!(0.16));
    }
}
