//! The months of the cash flow in the Simples Nacional, each with its own RBT12 and Fator R.
//!
//! The PGDAS-D taxes each month at the rates the revenue of the 12 previous months gives (RBT12)
//! and in the annex the payroll of those months gives (Fator R; LC 123/2006, art. 18, §24; Res.
//! CGSN 140/2018, art. 26). Here the 12 previous months are the ones known to the app (notes and
//! projections), in proportion when there are fewer (Res. CGSN 140/2018, art. 22, §3º) and the
//! month itself × 12 when there is none (§2º). The pró-labore is assumed paid in all of them: the
//! app keeps no payroll history (e.g. the months as MEI), which is left to the accountant.

use super::month::Month;
use super::payroll::{pro_labore_for_fator_r, pro_labore_taxes, ProLaboreTaxes};
use super::revenue::annualize;
use super::simples::{self, Annex};
use super::tables::TaxData;
use super::Input;
use rust_decimal::Decimal;

/// RBT12 of a month and how many known months it covers (0 = the month itself × 12).
pub(crate) struct Rbt12 {
    pub amount: Decimal,
    pub months: usize,
}

/// A month of the Simples Nacional with the pró-labore paid every month.
pub(crate) struct SimplesMonth {
    pub rbt12: Decimal,
    pub fator_r: Decimal,
    pub annex: Annex,
    /// DAS, plus the ISS paid to the municipality when the bracket has none inside it.
    pub company: Decimal,
    pub owner: ProLaboreTaxes,
    /// Smallest monthly pró-labore whose Fator R reaches the threshold in this month (rounded up to
    /// the cent, so the truncated Fator R does not fall short).
    pub pro_labore_for_annex_iii: Decimal,
}

/// RBT12 of `month`: the months known to the app among the 12 before it (from the opening month
/// on), in proportion when there are fewer than 12; the month's own revenue × 12 when there is none.
pub(crate) fn rbt12(input: &Input, month: Month) -> Rbt12 {
    let known: Vec<Decimal> = (1..=12)
        .map(|back| month.plus(-back))
        .filter(|m| input.opening.is_none_or(|opening| *m >= opening))
        .filter_map(|m| input.revenue_history.get(&m).copied())
        .collect();
    let twelve = Decimal::from(12);
    let total: Decimal = known.iter().sum();
    let amount = match known.len() {
        0 => {
            input
                .revenue_history
                .get(&month)
                .copied()
                .unwrap_or_default()
                * twelve
        }
        12 => total,
        n => annualize(total, n),
    };
    Rbt12 {
        amount,
        months: known.len(),
    }
}

/// Taxes of `month` (with `revenue`) in the Simples Nacional, with `pro_labore` paid every month,
/// at the tables of that month.
pub(crate) fn simples_month(
    input: &Input,
    data: &'static TaxData,
    month: Month,
    revenue: Decimal,
    pro_labore: Decimal,
) -> SimplesMonth {
    let tables = data.tables(month);
    let table = tables.simples;
    let rbt12 = rbt12(input, month).amount;
    let fs12 = simples::fs12(pro_labore, input.payroll);
    let (fator_r, annex) = simples::fator_r_and_annex(input.activity, fs12, rbt12, table);
    let das = simples::das(table, annex, rbt12, revenue);
    let iss = das.outside_iss(revenue, input.iss_rate);
    SimplesMonth {
        rbt12,
        fator_r,
        annex,
        company: das.total + iss,
        owner: pro_labore_taxes(pro_labore, input.dependents, &tables),
        pro_labore_for_annex_iii: pro_labore_for_fator_r(
            rbt12 / Decimal::from(12),
            input.payroll,
            table.fator_r_threshold,
        ),
    }
}
