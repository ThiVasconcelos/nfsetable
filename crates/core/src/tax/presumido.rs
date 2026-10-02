//! Lucro Presumido for a typical month.
//!
//! IRPJ and CSLL are due quarterly; here they are the monthly average (the quarterly surcharge
//! threshold of R$ 60 mil is R$ 20 mil per month). PIS, Cofins and ISS are monthly.

use super::money::round_money;
use super::tables::Presumido;
use rust_decimal::Decimal;

/// Company taxes of one month, each rounded to the cent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PresumidoTaxes {
    pub irpj: Decimal,
    pub csll: Decimal,
    pub pis: Decimal,
    pub cofins: Decimal,
    pub iss: Decimal,
    /// Employer contribution on the pró-labore.
    pub cpp: Decimal,
}

impl PresumidoTaxes {
    /// Shares in the order irpj, csll, pis, cofins, iss, cpp.
    pub fn shares(&self) -> Vec<(String, Decimal)> {
        [
            ("irpj", self.irpj),
            ("csll", self.csll),
            ("pis", self.pis),
            ("cofins", self.cofins),
            ("iss", self.iss),
            ("cpp", self.cpp),
        ]
        .into_iter()
        .map(|(tax, amount)| (tax.to_string(), amount))
        .collect()
    }
}

/// IRPJ: 15% of the presumed base (32% of the revenue) plus 10% of the part of the base above
/// R$ 20 mil; CSLL: 9% of 32%; PIS 0,65% and Cofins 3% (cumulative); ISS at the municipal rate;
/// CPP: 20% of the pró-labore.
pub(crate) fn taxes(
    table: &Presumido,
    revenue: Decimal,
    iss_rate: Decimal,
    pro_labore: Decimal,
) -> PresumidoTaxes {
    let irpj_base = round_money(revenue * table.irpj_presumption);
    let surcharge_base = (irpj_base - table.irpj_surcharge_monthly_threshold).max(Decimal::ZERO);
    let irpj = round_money(irpj_base * table.irpj_rate)
        + round_money(surcharge_base * table.irpj_surcharge_rate);
    let csll = round_money(round_money(revenue * table.csll_presumption) * table.csll_rate);
    PresumidoTaxes {
        irpj,
        csll,
        pis: round_money(revenue * table.pis_rate),
        cofins: round_money(revenue * table.cofins_rate),
        iss: round_money(revenue * iss_rate),
        cpp: round_money(pro_labore * table.cpp_rate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::tables_for;
    use rust_decimal_macros::dec;

    #[test]
    fn surcharge_applies_above_20_thousand_of_base() {
        let tables = tables_for("2026-01");
        // Revenue 100.000: base 32.000; IRPJ 15% × 32.000 = 4.800 + 10% × 12.000 = 1.200.
        let t = taxes(tables.presumido, dec!(100000), dec!(0.05), dec!(1621));
        assert_eq!(t.irpj, dec!(6000.00));
        // CSLL 9% × 32.000 = 2.880; PIS 650; Cofins 3.000; ISS 5.000; CPP 20% × 1.621 = 324,20.
        assert_eq!(
            (t.csll, t.pis, t.cofins, t.iss, t.cpp),
            (
                dec!(2880.00),
                dec!(650.00),
                dec!(3000.00),
                dec!(5000.00),
                dec!(324.20)
            )
        );
    }
}
