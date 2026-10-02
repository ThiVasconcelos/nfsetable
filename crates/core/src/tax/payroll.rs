//! Pró-labore: the automatic amount for the Fator R and the INSS and IRRF withheld from it.

use super::money::{ceil_money, round_money};
use super::tables::{IrrfReduction, Tables};
use rust_decimal::Decimal;

/// INSS and IRRF withheld from a monthly pró-labore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ProLaboreTaxes {
    pub gross: Decimal,
    pub inss: Decimal,
    /// IRRF withheld (after the reduction and the R$ 10 dispensation).
    pub irrf: Decimal,
    /// Reduction of Lei 15.270/2025 applied to the table tax.
    pub reduction: Decimal,
}

impl ProLaboreTaxes {
    pub fn total(&self) -> Decimal {
        self.inss + self.irrf
    }

    pub fn net(&self) -> Decimal {
        self.gross - self.inss - self.irrf
    }
}

/// Pró-labore that brings the Fator R to the threshold (28%): the threshold times the monthly
/// revenue it is measured against (RBT12 / 12, which is the average when fewer than 12 months
/// are considered) minus the other payroll, rounded UP to the cent so the truncated Fator R does
/// not fall short, and never below the minimum wage (the minimum wage itself without revenue).
pub(crate) fn auto_pro_labore(monthly: Decimal, payroll: Decimal, tables: &Tables) -> Decimal {
    if monthly.is_zero() {
        return tables.minimum_wage;
    }
    ceil_money(tables.simples.fator_r_threshold * monthly - payroll).max(tables.minimum_wage)
}

/// INSS (11% up to the ceiling) and IRRF (monthly table, legal deductions or the simplified
/// discount, whichever is larger, and the reduction of Lei 15.270/2025 on the GROSS amount)
/// withheld from `gross`. Each step is rounded to the cent.
pub(crate) fn pro_labore_taxes(gross: Decimal, dependents: u32, tables: &Tables) -> ProLaboreTaxes {
    let gross = gross.max(Decimal::ZERO);
    let inss = round_money(gross.min(tables.inss.ceiling) * tables.inss.pro_labore_rate);

    let irrf = tables.irrf;
    let legal_deductions = inss + irrf.dependent_deduction * Decimal::from(dependents);
    let base = (gross - legal_deductions.max(irrf.simplified_discount)).max(Decimal::ZERO);
    let bracket = irrf
        .brackets
        .iter()
        .find(|b| b.up_to.is_none_or(|limit| base <= limit))
        .unwrap_or_else(|| &irrf.brackets[irrf.brackets.len() - 1]);
    let tax = round_money(base * bracket.rate - bracket.deduction).max(Decimal::ZERO);

    let reduction = tables
        .irrf_reduction
        .map_or(Decimal::ZERO, |r| monthly_reduction(r, gross, tax));
    let mut withheld = tax - reduction;
    if withheld <= irrf.min_withholding {
        withheld = Decimal::ZERO;
    }
    ProLaboreTaxes {
        gross,
        inss,
        irrf: withheld,
        reduction,
    }
}

/// Lei 9.250/1995, art. 3º-A: up to the full-reduction limit the tax is reduced by up to the
/// maximum (so it becomes zero); in the phase-out range by `constant − coefficient × gross`;
/// above it, nothing. Never more than the tax itself.
fn monthly_reduction(reduction: &IrrfReduction, gross: Decimal, tax: Decimal) -> Decimal {
    if gross <= reduction.full_reduction_up_to {
        tax.min(reduction.max_reduction)
    } else if gross <= reduction.phase_out_up_to {
        let phase_out =
            round_money(reduction.phase_out_constant - reduction.phase_out_coefficient * gross);
        tax.min(phase_out.max(Decimal::ZERO))
    } else {
        Decimal::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    use crate::tax::tables::tables_for as tables;

    #[test]
    fn examples_of_the_research_2026() {
        // Worked examples of the IRRF on the pró-labore in 2026, without dependents.
        let t = tables("2026-06");
        let cases = [
            // gross, INSS, IRRF, reduction
            (dec!(1621.00), dec!(178.31), dec!(0), dec!(0)),
            (dec!(2800.00), dec!(308.00), dec!(0), dec!(0)),
            (dec!(5000.00), dec!(550.00), dec!(0), dec!(312.89)),
            (dec!(5600.00), dec!(616.00), dec!(228.86), dec!(233.01)),
            (dec!(7350.00), dec!(808.50), dec!(890.18), dec!(0)),
            (dec!(11200.00), dec!(932.31), dec!(1914.88), dec!(0)),
        ];
        for (gross, inss, irrf, reduction) in cases {
            let taxes = pro_labore_taxes(gross, 0, &t);
            assert_eq!(
                (taxes.inss, taxes.irrf, taxes.reduction),
                (inss, irrf, reduction),
                "{gross}"
            );
        }
    }

    #[test]
    fn dependents_and_dispensation() {
        let t = tables("2025-06");
        // 2025, no reduction yet. Gross 3.000, 1 dependent: INSS 330,00; legal deductions
        // 330,00 + 189,59 = 519,59 < 607,20 → base 2.392,80 ≤ 2.428,80 → no tax.
        assert_eq!(pro_labore_taxes(dec!(3000), 1, &t).irrf, dec!(0));
        // Gross 3.100, no dependents: base 3.100 − 607,20 = 2.492,80; 7,5% − 182,16 = 4,80,
        // which is at most R$ 10 and therefore not withheld.
        assert_eq!(pro_labore_taxes(dec!(3100), 0, &t).irrf, dec!(0));
        // Gross 3.300: base 2.692,80 × 7,5% − 182,16 = 19,80 → withheld.
        assert_eq!(pro_labore_taxes(dec!(3300), 0, &t).irrf, dec!(19.80));
        // Two dependents: 363,00 + 379,18 = 742,18 > 607,20 → base 2.557,82 → 9,68 → dispensed.
        assert_eq!(pro_labore_taxes(dec!(3300), 2, &t).irrf, dec!(0));
    }

    #[test]
    fn automatic_pro_labore() {
        let t = tables("2026-03");
        assert_eq!(auto_pro_labore(dec!(5000), dec!(0), &t), dec!(1621));
        assert_eq!(auto_pro_labore(dec!(20000), dec!(0), &t), dec!(5600));
        assert_eq!(auto_pro_labore(dec!(20000), dec!(2000), &t), dec!(3600));
        assert_eq!(auto_pro_labore(dec!(0), dec!(0), &t), dec!(1621));
        // 28% × 10.000,01 = 2.800,0028 → rounded UP to 2.800,01: rounding half-up would give
        // 2.800,00, and 2.800,00 / 10.000,01 = 0,27999… truncates to a Fator R of 0,27.
        assert_eq!(auto_pro_labore(dec!(10000.01), dec!(0), &t), dec!(2800.01));
    }
}
