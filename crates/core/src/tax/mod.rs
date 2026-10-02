//! Tax estimates (MEI, Simples Nacional, Lucro Presumido) from the monthly revenue.
//!
//! Everything here is an estimate for planning; the tables live in `crates/core/tax/br.json`,
//! versioned by date, and are chosen by the reference month, never by today's date.
//!
//! Model ("typical month"): the user picks the months to consider and their average is the
//! revenue of a typical month, used by every estimate (DAS, pró-labore, regime comparison,
//! "sobra do mês" and "quanto cobrar"). Amounts are computed with `rust_decimal` in reais and
//! rounded to the cent (half away from zero) at each tax step; the contract carries integer
//! cents, and rates as `f64` converted from the exact decimals.
//!
//! Modules: `tables` (the embedded data and the choice of versions), `revenue` (months, average,
//! RBT12), `payroll` (pró-labore, INSS, IRRF), `mei`, `simples` and `presumido` (each regime),
//! `scenarios` (comparison, leftover, pricing, cash flow), `window` (each month's RBT12 and Fator
//! R in the cash flow) and `report` (warnings and the contract types).

mod mei;
mod money;
pub(crate) mod month;
mod payroll;
mod presumido;
mod report;
mod revenue;
mod scenarios;
mod simples;
mod tables;
mod window;

use crate::error::CoreError;
use crate::model::{MonthAmount, SimplesActivity, TaxCatalog, TaxInput, TaxRegime, TaxReport};
use money::{from_cents, from_f64};
use month::Month;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Computes the tax report of `input.reference_month`.
///
/// Fails only when the reference month or the ISS rate is invalid (or the embedded tables are
/// broken); problems with individual months become warnings.
pub fn tax_report(input: &TaxInput) -> Result<TaxReport, CoreError> {
    let data = tables::data()?;
    let mut warnings = Vec::new();
    let mut normalized = Input::new(input, &mut warnings)?;
    let months = revenue::collect(&input.revenue_by_month, &mut warnings);
    // The months considered are known months too (and win over the history).
    normalized
        .revenue_history
        .extend(months.iter().map(|(month, amount)| (*month, *amount)));
    if months.is_empty() {
        warnings.push(
            "Nenhum mês selecionado: escolha ao menos um mês para ver as estimativas.".to_string(),
        );
        return Ok(report::zero_report(&normalized, warnings));
    }
    let tables = data.tables(normalized.reference);
    report::coverage_warnings(&tables, normalized.reference, &mut warnings);
    let revenue = revenue::summarize(months, normalized.reference);
    Ok(report::build(
        &normalized,
        &tables,
        data,
        &revenue,
        warnings,
    ))
}

/// CNAE codes known to the tax calculations (IT services first) and the suggested default.
pub fn tax_catalog() -> TaxCatalog {
    tables::catalog()
}

/// The input, validated and converted to decimals (reais).
pub(crate) struct Input {
    pub reference: Month,
    pub regime: TaxRegime,
    pub activity: SimplesActivity,
    pub opening: Option<Month>,
    pub user_pro_labore: Option<Decimal>,
    /// Revenue of every known month (history and months considered), for the RBT12 of each month
    /// of the cash flow and the automatic pró-labore.
    pub revenue_history: BTreeMap<Month, Decimal>,
    pub payroll: Decimal,
    pub dependents: u32,
    /// Fixed monthly costs.
    pub costs: Decimal,
    /// Variable costs per month.
    pub variable_costs: BTreeMap<Month, Decimal>,
    /// Fixed costs per month (dated items), on top of `costs`.
    pub fixed_costs: BTreeMap<Month, Decimal>,
    /// Revenue of "sobra do mês" when it is not the average.
    pub leftover_revenue: Option<Decimal>,
    /// Share of the revenue set aside as a reserve, 0 to 0.9.
    pub reserve_rate: Decimal,
    pub iss_rate: Decimal,
    pub desired_net: Option<Decimal>,
}

impl Input {
    /// Fixed costs (every month plus the dated ones in effect) and variable costs of `month`.
    pub fn costs_in(&self, month: Month) -> (Decimal, Decimal) {
        let fixed = self.costs + self.fixed_costs.get(&month).copied().unwrap_or_default();
        let variable = self.variable_costs.get(&month).copied().unwrap_or_default();
        (fixed, variable)
    }

    /// Errors for an invalid reference month or ISS rate; a malformed opening month is ignored and
    /// negative amounts count as zero, with pt-BR warnings.
    fn new(input: &TaxInput, warnings: &mut Vec<String>) -> Result<Input, CoreError> {
        let reference = Month::parse(&input.reference_month).ok_or_else(|| {
            CoreError::Tax(format!(
                "mês de referência inválido: \"{}\" (use o formato aaaa-mm)",
                input.reference_month
            ))
        })?;
        let iss_rate = Some(input.iss_rate)
            .filter(|rate| (0.0..=1.0).contains(rate))
            .and_then(from_f64)
            .ok_or_else(|| {
                CoreError::Tax(format!(
                    "alíquota de ISS inválida: {} (use decimal, por exemplo 0.05 para 5%)",
                    input.iss_rate
                ))
            })?;
        let reserve_rate = Some(input.reserve_rate)
            .filter(|rate| (0.0..=0.9).contains(rate))
            .and_then(from_f64)
            .ok_or_else(|| {
                CoreError::Tax(format!(
                    "reserva inválida: {} (use decimal entre 0 e 0.9, por exemplo 0.1 para 10%)",
                    input.reserve_rate
                ))
            })?;
        let opening = match input.opening_month.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(text) => {
                let parsed = Month::parse(text);
                if parsed.is_none() {
                    warnings.push(format!(
                        "Mês de abertura inválido ignorado: \"{text}\" (use o formato aaaa-mm)."
                    ));
                }
                parsed
            }
        };
        // Problems in the history repeat the ones of the months considered: not reported twice.
        let revenue_history = revenue::collect(&input.revenue_history, &mut Vec::new());
        let variable_costs = costs_by_month(&input.costs_by_month, "gasto variável", warnings);
        let fixed_costs = costs_by_month(&input.fixed_costs_by_month, "custo fixo", warnings);
        let mut non_negative = |cents: i64, warning: &str| {
            if cents < 0 {
                warnings.push(warning.to_string());
                Decimal::ZERO
            } else {
                from_cents(cents)
            }
        };
        let user_pro_labore = input.pro_labore_cents.map(|cents| {
            non_negative(
                cents,
                "O pró-labore informado é negativo e foi considerado zero.",
            )
        });
        let payroll = non_negative(
            input.payroll_cents,
            "A folha informada é negativa e foi considerada zero.",
        );
        let costs = non_negative(
            input.monthly_costs_cents,
            "Os custos mensais informados são negativos e foram considerados zero.",
        );
        let leftover_revenue = input.leftover_revenue_cents.map(|cents| {
            non_negative(
                cents,
                "A receita informada para a sobra do mês é negativa e foi considerada zero.",
            )
        });
        let desired_net = input.desired_net_cents.map(|cents| {
            non_negative(
                cents,
                "O valor desejado por mês é negativo e foi considerado zero.",
            )
        });
        Ok(Input {
            reference,
            regime: input.regime,
            activity: input.activity,
            opening,
            user_pro_labore,
            revenue_history,
            payroll,
            dependents: input.dependents,
            costs,
            variable_costs,
            fixed_costs,
            leftover_revenue,
            reserve_rate,
            iss_rate,
            desired_net,
        })
    }
}

/// Costs by month (`what` names them in the warnings, e.g. "gasto variável"; see
/// `revenue::sum_by_month`).
fn costs_by_month(
    entries: &[MonthAmount],
    what: &str,
    warnings: &mut Vec<String>,
) -> BTreeMap<Month, Decimal> {
    revenue::sum_by_month(
        entries,
        warnings,
        |text| format!("Mês de {what} inválido ignorado: \"{text}\" (use o formato aaaa-mm)."),
        |month| format!("O {what} de {month} é negativo e foi considerado zero."),
    )
}
