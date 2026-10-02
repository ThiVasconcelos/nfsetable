//! Regime scenarios for a typical month: the comparison, the "sobra do mês" (leftover) and
//! "quanto cobrar" (pricing).

use super::mei;
use super::money::{brl, from_cents, percent, ratio, round_money, to_cents, to_f64};
use super::month::Month;
use super::payroll::{auto_pro_labore, pro_labore_taxes, ProLaboreTaxes};
use super::presumido;
use super::simples::{self, fator_r, Annex, Das};
use super::tables::{Distribution, Tables, TaxData};
use super::window;
use super::Input;
use crate::model::{
    CashFlowMonth, LeftoverReport, MeiStatus, PricingReport, RegimeCost, SimplesActivity,
    TaxRegime, TaxShare,
};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

pub(crate) const LABEL_MEI: &str = "MEI";
pub(crate) const LABEL_SIMPLES_III: &str = "Simples Nacional (ME) – Anexo III";
pub(crate) const LABEL_SIMPLES_V: &str = "Simples Nacional (ME) – Anexo V";
pub(crate) const LABEL_PRESUMIDO: &str = "Lucro Presumido";

/// What the scenarios share: the input, the tables and the typical month.
pub(crate) struct Ctx<'a> {
    pub input: &'a Input,
    pub tables: &'a Tables,
    /// Average monthly revenue (the typical month).
    pub revenue: Decimal,
    pub rbt12: Decimal,
    /// Pró-labore that brings the Fator R to 28% for the typical month.
    pub auto_pro_labore: Decimal,
    /// Fixed costs of the typical month: the fixed amount plus the dated ones in effect in the
    /// reference month.
    pub fixed_costs: Decimal,
    /// Variable costs of the reference month.
    pub variable_costs: Decimal,
    /// Fixed plus variable costs of the typical month.
    pub costs: Decimal,
}

/// Where the year's revenue is heading against the MEI limit.
pub(crate) struct MeiOutlook {
    pub limit: Decimal,
    pub projection: Decimal,
    pub projected: MeiStatus,
}

/// Taxes of one regime for one month, each rounded to the cent.
pub(crate) struct Cost {
    /// Paid by the company (DAS split, or IRPJ/CSLL/PIS/Cofins/ISS/CPP).
    pub company: Vec<(String, Decimal)>,
    /// Pró-labore and what is withheld from it (zero for the MEI).
    pub owner: ProLaboreTaxes,
}

impl Cost {
    pub fn company_total(&self) -> Decimal {
        self.company.iter().map(|(_, amount)| *amount).sum()
    }

    pub fn total(&self) -> Decimal {
        self.company_total() + self.owner.total()
    }

    fn share(&self, tax: &str) -> Decimal {
        self.company
            .iter()
            .filter(|(name, _)| name == tax)
            .map(|(_, amount)| *amount)
            .sum()
    }
}

/// Contract shares from (tax, amount) pairs.
pub(crate) fn tax_shares(shares: &[(String, Decimal)]) -> Vec<TaxShare> {
    shares
        .iter()
        .map(|(tax, amount)| TaxShare {
            tax: tax.clone(),
            cents: to_cents(*amount),
        })
        .collect()
}

fn mei_cost(tables: &Tables) -> Cost {
    let (inss, iss) = mei::das(tables);
    Cost {
        company: vec![("inss".to_string(), inss), ("iss".to_string(), iss)],
        owner: ProLaboreTaxes::default(),
    }
}

/// The DAS, plus the ISS paid directly to the municipality when the bracket has no ISS inside
/// the DAS (6th), and the taxes withheld from the pró-labore.
fn simples_cost(ctx: &Ctx, das: &Das, revenue: Decimal, pro_labore: Decimal) -> Cost {
    let mut company = das.split.clone();
    if das.iss_outside {
        company.push((
            "iss".to_string(),
            das.outside_iss(revenue, ctx.input.iss_rate),
        ));
    }
    Cost {
        company,
        owner: pro_labore_taxes(pro_labore, ctx.input.dependents, ctx.tables),
    }
}

fn presumido_cost(ctx: &Ctx, revenue: Decimal, pro_labore: Decimal) -> Cost {
    let taxes = presumido::taxes(
        ctx.tables.presumido,
        revenue,
        ctx.input.iss_rate,
        pro_labore,
    );
    Cost {
        company: taxes.shares(),
        owner: pro_labore_taxes(pro_labore, ctx.input.dependents, ctx.tables),
    }
}

/// Pró-labore assumed in the Lucro Presumido: the user's, or the minimum wage.
fn presumido_pro_labore(ctx: &Ctx) -> Decimal {
    ctx.input.user_pro_labore.unwrap_or(ctx.tables.minimum_wage)
}

/// Whether this pró-labore (plus the other payroll) brings the Fator R of the typical month to
/// the threshold, i.e. the revenue would be taxed in Anexo III.
fn reaches_fator_r(ctx: &Ctx, pro_labore: Decimal) -> bool {
    let table = ctx.tables.simples;
    let fs12 = simples::fs12(pro_labore, ctx.input.payroll);
    fator_r(fs12, ctx.rbt12, table) >= table.fator_r_threshold
}

fn regime_cost(
    ctx: &Ctx,
    (regime, label): (&str, &str),
    available: bool,
    note: String,
    cost: &Cost,
) -> RegimeCost {
    let total = cost.total();
    let reserve = reserve(ctx, ctx.revenue);
    RegimeCost {
        regime: regime.to_string(),
        label: label.to_string(),
        available,
        note: Some(note),
        company_taxes_cents: to_cents(cost.company_total()),
        company_taxes: tax_shares(&cost.company),
        pro_labore_cents: to_cents(cost.owner.gross),
        owner_taxes_cents: to_cents(cost.owner.total()),
        total_taxes_cents: to_cents(total),
        total_rate: to_f64(ratio(total, ctx.revenue)),
        reserve_cents: to_cents(reserve),
        owner_net_cents: to_cents(ctx.revenue - total - ctx.costs - reserve),
    }
}

/// Reserve set aside from `revenue` every month, rounded to the cent.
fn reserve(ctx: &Ctx, revenue: Decimal) -> Decimal {
    round_money(revenue * ctx.input.reserve_rate)
}

// ------------------------------------------------------------------ comparison

/// MEI, Simples Anexo III, Simples Anexo V and Lucro Presumido for the typical month, in this order.
pub(crate) fn comparison(ctx: &Ctx, outlook: &MeiOutlook) -> Vec<RegimeCost> {
    vec![
        mei_entry(ctx, outlook),
        simples_iii_entry(ctx),
        simples_v_entry(ctx),
        presumido_entry(ctx),
    ]
}

fn mei_entry(ctx: &Ctx, outlook: &MeiOutlook) -> RegimeCost {
    let it_blocked = mei::it_blocked(ctx.input.activity, ctx.tables);
    let mut note = if it_blocked {
        "Desenvolvimento de software e serviços de TI não podem ser MEI (LC 123/2006, \
         art. 18-A, § 4º, I); o custo aparece só como referência."
            .to_string()
    } else {
        "Só para ocupações permitidas ao MEI (Anexo XI da Res. CGSN 140/2018), como instrutor \
         de informática ou técnico de manutenção de computadores; desenvolvimento de software e \
         serviços de TI não podem ser MEI (LC 123/2006, art. 18-A, § 4º, I)."
            .to_string()
    };
    let over_limit = outlook.projected != MeiStatus::Within;
    if over_limit {
        note.push_str(&format!(
            " No ritmo atual, a receita do ano chega a {}, acima do limite do MEI ({}).",
            brl(outlook.projection),
            brl(outlook.limit)
        ));
    }
    let cost = mei_cost(ctx.tables);
    regime_cost(
        ctx,
        ("mei", LABEL_MEI),
        !it_blocked && !over_limit,
        note,
        &cost,
    )
}

fn simples_iii_entry(ctx: &Ctx) -> RegimeCost {
    let auto = ctx.auto_pro_labore;
    let threshold = percent(ctx.tables.simples.fator_r_threshold);
    let (pro_labore, mut note) = match (ctx.input.activity, ctx.input.user_pro_labore) {
        (SimplesActivity::AnnexIii, user) => {
            let (pl, origin) = user.map_or((auto, "automático"), |pl| (pl, "informado"));
            (
                pl,
                format!(
                    "Anexo III direto: a atividade não depende do Fator R. Pró-labore {origin} \
                     de {}.",
                    brl(pl)
                ),
            )
        }
        (SimplesActivity::FatorR, Some(user)) if user >= auto => (
            user,
            format!(
                "Pró-labore informado de {}, que já leva o Fator R a pelo menos {threshold}.",
                brl(user)
            ),
        ),
        (SimplesActivity::FatorR, user) => {
            let mut note = if auto == ctx.tables.minimum_wage {
                format!(
                    "Pró-labore de um salário mínimo ({}), que já leva o Fator R a pelo menos \
                     {threshold}.",
                    brl(auto)
                )
            } else {
                format!(
                    "Pró-labore de {}: {threshold} da receita mensal média (RBT12 ÷ 12), \
                     descontada a folha, para o Fator R chegar a {threshold} e a receita ficar no \
                     Anexo III.",
                    brl(auto)
                )
            };
            match user {
                Some(user) if reaches_fator_r(ctx, user) => note.push_str(&format!(
                    " O informado ({}) fica abaixo do salário mínimo.",
                    brl(user)
                )),
                Some(user) => note.push_str(&format!(
                    " O informado ({}) deixaria o Fator R abaixo de {threshold}.",
                    brl(user)
                )),
                None => {}
            }
            (auto, note)
        }
    };
    let das = simples::das(ctx.tables.simples, Annex::Iii, ctx.rbt12, ctx.revenue);
    if let Some(band) = band_note(ctx, &das) {
        note.push(' ');
        note.push_str(&band);
    }
    let cost = simples_cost(ctx, &das, ctx.revenue, pro_labore);
    regime_cost(
        ctx,
        ("simplesIii", LABEL_SIMPLES_III),
        !das.above_ceiling,
        note,
        &cost,
    )
}

fn simples_v_entry(ctx: &Ctx) -> RegimeCost {
    let auto = ctx.auto_pro_labore;
    let minimum = ctx.tables.minimum_wage;
    let threshold = percent(ctx.tables.simples.fator_r_threshold);
    let fator_r_activity = ctx.input.activity == SimplesActivity::FatorR;
    // The typed pró-labore when it is below the automatic one, otherwise one minimum wage.
    let (pro_labore, typed) = match ctx.input.user_pro_labore {
        Some(user) if user < auto || !fator_r_activity => (user, true),
        _ => (minimum, false),
    };
    let what = if typed {
        format!("o pró-labore informado ({})", brl(pro_labore))
    } else {
        format!("pró-labore de um salário mínimo ({})", brl(pro_labore))
    };
    let mut note = if !fator_r_activity {
        "A atividade é sempre do Anexo III.".to_string()
    } else if reaches_fator_r(ctx, pro_labore) {
        format!(
            "Com {what} o Fator R já chega a {threshold} e a receita iria para o Anexo III; o \
             Anexo V aparece só como referência."
        )
    } else {
        format!("Com {what} o Fator R fica abaixo de {threshold} e a receita vai para o Anexo V.")
    };
    let das = simples::das(ctx.tables.simples, Annex::V, ctx.rbt12, ctx.revenue);
    if let Some(band) = band_note(ctx, &das) {
        note.push(' ');
        note.push_str(&band);
    }
    let cost = simples_cost(ctx, &das, ctx.revenue, pro_labore);
    regime_cost(
        ctx,
        ("simplesV", LABEL_SIMPLES_V),
        fator_r_activity && !das.above_ceiling,
        note,
        &cost,
    )
}

/// ME/EPP band, 6th bracket and ceiling remarks for a Simples scenario.
fn band_note(ctx: &Ctx, das: &Das) -> Option<String> {
    let simples = ctx.tables.simples;
    if das.above_ceiling {
        return Some(format!(
            "RBT12 de {} acima do teto do Simples Nacional ({}): a empresa não pode ser ME/EPP \
             optante; os valores usam a 6ª faixa só como referência.",
            brl(ctx.rbt12),
            brl(simples.epp_limit)
        ));
    }
    if ctx.rbt12 <= simples.me_limit {
        return None;
    }
    let mut note = format!(
        "Com RBT12 de {}, acima de {} por ano, a empresa é EPP (ME/EPP do Simples Nacional, até \
         {}).",
        brl(ctx.rbt12),
        brl(simples.me_limit),
        brl(simples.epp_limit)
    );
    if das.iss_outside {
        note.push_str(&format!(
            " Na 6ª faixa o ISS sai do DAS e é pago à parte ao município (considerado a {}).",
            percent(ctx.input.iss_rate)
        ));
    }
    Some(note)
}

fn presumido_entry(ctx: &Ctx) -> RegimeCost {
    let table = ctx.tables.presumido;
    let pro_labore = presumido_pro_labore(ctx);
    let origin = if ctx.input.user_pro_labore.is_some() {
        "informado"
    } else {
        "de um salário mínimo"
    };
    let mut note = format!(
        "IRPJ ({} sobre a presunção de {}, mais {} sobre a parte da base acima de {} por mês) e \
         CSLL ({} sobre {}) são apurados por trimestre; aqui aparecem como média mensal. PIS \
         {} e Cofins {} cumulativos, ISS de {} e contribuição patronal de {} sobre o pró-labore \
         {origin} ({}). IRRF e CSRF retidos pelo tomador são antecipação e não mudam o custo.",
        percent(table.irpj_rate),
        percent(table.irpj_presumption),
        percent(table.irpj_surcharge_rate),
        brl(table.irpj_surcharge_monthly_threshold),
        percent(table.csll_rate),
        percent(table.csll_presumption),
        percent(table.pis_rate),
        percent(table.cofins_rate),
        percent(ctx.input.iss_rate),
        percent(table.cpp_rate),
        brl(pro_labore)
    );
    if ctx.input.payroll > Decimal::ZERO {
        note.push_str(" Não inclui os encargos patronais sobre a folha de empregados.");
    }
    let cost = presumido_cost(ctx, ctx.revenue, pro_labore);
    regime_cost(ctx, ("presumido", LABEL_PRESUMIDO), true, note, &cost)
}

// ------------------------------------------------------------------ leftover

/// "Sobra do mês" in the current regime, and the rule of its tax-free distribution limit.
///
/// `das` and `pro_labore` are the Simples DAS and pró-labore in use (user's or automatic). The
/// revenue is the average, or the one chosen by the user (e.g. without bonuses); the annex and
/// the effective rate still come from the real RBT12, and so does the pró-labore.
pub(crate) fn leftover(
    ctx: &Ctx,
    das: &Das,
    pro_labore: Decimal,
) -> (LeftoverReport, &'static Distribution) {
    let tables = ctx.tables;
    let revenue = ctx.input.leftover_revenue.unwrap_or(ctx.revenue);
    let (cost, distribution, deductible) = match ctx.input.regime {
        TaxRegime::Mei => (
            mei_cost(tables),
            &tables.mei.tax_free_distribution,
            Decimal::ZERO,
        ),
        // Res. CGSN 140/2018, art. 145, § 1º: 32% of the revenue minus the IRPJ in the DAS.
        TaxRegime::Simples => {
            // Same annex and effective rate (real RBT12), applied to the chosen revenue.
            let das = simples::das(tables.simples, das.annex, ctx.rbt12, revenue);
            (
                simples_cost(ctx, &das, revenue, pro_labore),
                &tables.simples.tax_free_distribution,
                das.share("irpj"),
            )
        }
        // Presumed base minus IRPJ, CSLL, PIS and Cofins.
        TaxRegime::Presumido => {
            let cost = presumido_cost(ctx, revenue, presumido_pro_labore(ctx));
            let deductible = ["irpj", "csll", "pis", "cofins"]
                .iter()
                .map(|tax| cost.share(tax))
                .sum();
            (cost, &tables.presumido.tax_free_distribution, deductible)
        }
    };
    let limit = (round_money(revenue * distribution.presumption) - deductible).max(Decimal::ZERO);
    let reserve = reserve(ctx, revenue);
    let report = LeftoverReport {
        revenue_cents: to_cents(revenue),
        custom_revenue: ctx.input.leftover_revenue.is_some(),
        company_taxes_cents: to_cents(cost.company_total()),
        pro_labore_inss_cents: to_cents(cost.owner.inss),
        pro_labore_irrf_cents: to_cents(cost.owner.irrf),
        costs_cents: to_cents(ctx.fixed_costs),
        variable_costs_cents: to_cents(ctx.variable_costs),
        reserve_cents: to_cents(reserve),
        leftover_cents: to_cents(revenue - cost.total() - ctx.costs - reserve),
        tax_free_distribution_limit_cents: to_cents(limit),
    };
    (report, distribution)
}

// ------------------------------------------------------------------ cash flow

/// Each month considered in the current regime: its own revenue, costs and pró-labore. In the
/// Simples Nacional each month has its own RBT12, Fator R and annex (see `window`); the MEI pays the
/// DAS-MEI and the Lucro Presumido its taxes on the month's revenue.
pub(crate) fn cash_flow(
    ctx: &Ctx,
    data: &'static TaxData,
    pro_labore: Decimal,
    months: &BTreeMap<Month, Decimal>,
) -> Vec<CashFlowMonth> {
    months
        .iter()
        .map(|(month, revenue)| {
            let revenue = *revenue;
            let mut simples = None;
            let (company, owner) = match ctx.input.regime {
                TaxRegime::Mei => {
                    let cost = mei_cost(ctx.tables);
                    (cost.company_total(), cost.owner)
                }
                TaxRegime::Simples => {
                    let figures =
                        window::simples_month(ctx.input, data, *month, revenue, pro_labore);
                    let taxes = (figures.company, figures.owner);
                    simples = Some(figures);
                    taxes
                }
                TaxRegime::Presumido => {
                    let cost = presumido_cost(ctx, revenue, presumido_pro_labore(ctx));
                    (cost.company_total(), cost.owner)
                }
            };
            let (fixed, variable) = ctx.input.costs_in(*month);
            let reserve = reserve(ctx, revenue);
            let leftover = revenue - company - owner.total() - fixed - variable - reserve;
            let profit = revenue - company - fixed - variable - owner.gross;
            let fator_r_activity = ctx.input.activity == SimplesActivity::FatorR;
            CashFlowMonth {
                month: month.iso(),
                revenue_cents: to_cents(revenue),
                company_taxes_cents: to_cents(company),
                owner_taxes_cents: to_cents(owner.total()),
                fixed_costs_cents: to_cents(fixed),
                variable_costs_cents: to_cents(variable),
                reserve_cents: to_cents(reserve),
                leftover_cents: to_cents(leftover),
                pro_labore_cents: to_cents(owner.gross),
                inss_cents: to_cents(owner.inss),
                irrf_cents: to_cents(owner.irrf),
                profit_cents: to_cents(profit),
                rbt12_cents: simples.as_ref().map(|m| to_cents(m.rbt12)),
                fator_r: simples.as_ref().map(|m| to_f64(m.fator_r)),
                annex: simples.as_ref().map(|m| m.annex.label().to_string()),
                pro_labore_for_annex_iii_cents: simples
                    .as_ref()
                    .filter(|_| fator_r_activity)
                    .map(|m| to_cents(m.pro_labore_for_annex_iii)),
            }
        })
        .collect()
}

// ------------------------------------------------------------------ pricing

/// The current regime at a constant monthly revenue.
struct PricingScenario {
    cost: Cost,
    das: Option<Das>,
    net: Decimal,
}

/// Current regime with `revenue` every month: RBT12 = 12 × revenue and the automatic pró-labore
/// (Simples), the Presumido pró-labore, or just the DAS-MEI.
fn pricing_scenario(ctx: &Ctx, revenue: Decimal) -> PricingScenario {
    let (cost, das) = match ctx.input.regime {
        TaxRegime::Mei => (mei_cost(ctx.tables), None),
        TaxRegime::Simples => {
            let simples = ctx.tables.simples;
            let rbt12 = revenue * Decimal::from(12);
            let pro_labore = auto_pro_labore(revenue, ctx.input.payroll, ctx.tables);
            let fs12 = simples::fs12(pro_labore, ctx.input.payroll);
            let (_, annex) = simples::fator_r_and_annex(ctx.input.activity, fs12, rbt12, simples);
            let das = simples::das(simples, annex, rbt12, revenue);
            (simples_cost(ctx, &das, revenue, pro_labore), Some(das))
        }
        TaxRegime::Presumido => (
            presumido_cost(ctx, revenue, presumido_pro_labore(ctx)),
            None,
        ),
    };
    let net = revenue - cost.total() - ctx.costs - reserve(ctx, revenue);
    PricingScenario { cost, das, net }
}

/// "Quanto cobrar": the smallest monthly revenue whose owner net reaches `desired`, or `None`
/// if the search does not converge (never expected).
pub(crate) fn pricing(ctx: &Ctx, desired: Decimal) -> Option<PricingReport> {
    let desired_cents = to_cents(desired);
    let net = |cents: i64| to_cents(pricing_scenario(ctx, from_cents(cents)).net);
    let required = required_revenue(desired_cents, to_cents(ctx.costs), net)?;
    let revenue = from_cents(required);
    let scenario = pricing_scenario(ctx, revenue);
    Some(PricingReport {
        desired_net_cents: desired_cents,
        required_revenue_cents: required,
        note: pricing_note(ctx, desired, revenue, &scenario),
    })
}

/// Smallest revenue (cents) with `net(revenue) >= desired`.
///
/// Starts at `desired + costs` (taxes are never negative, so nothing below can reach it) and adds
/// the shortfall at each step. Taxes never decrease as the revenue grows, so the net grows by at
/// most the revenue added: no revenue inside a step reaches the target and the first hit is the
/// smallest. A final bisection keeps "one cent less does not reach it" true even if some tax
/// decreased (e.g. the ISS leaving the DAS in the 6th bracket with a low municipal rate).
fn required_revenue(desired: i64, costs: i64, net: impl Fn(i64) -> i64) -> Option<i64> {
    const MAX_STEPS: usize = 1_000;
    /// R$ 10 trilhões.
    const MAX_REVENUE: i64 = 1_000_000_000_000_000;
    let mut revenue = desired.saturating_add(costs).max(0);
    let mut below: Option<i64> = None;
    for _ in 0..MAX_STEPS {
        if revenue > MAX_REVENUE {
            return None;
        }
        let value = net(revenue);
        if value >= desired {
            return Some(match below {
                Some(low) if revenue - 1 > low && net(revenue - 1) >= desired => {
                    bisect(low, revenue, desired, &net)
                }
                _ => revenue,
            });
        }
        below = Some(revenue);
        revenue = revenue.checked_add(desired.checked_sub(value)?)?;
    }
    None
}

/// With `net(low) < desired <= net(high)`, a revenue where the target is first reached from the
/// cent below.
fn bisect(mut low: i64, mut high: i64, desired: i64, net: &impl Fn(i64) -> i64) -> i64 {
    while high - low > 1 {
        let mid = low + (high - low) / 2;
        if net(mid) >= desired {
            high = mid;
        } else {
            low = mid;
        }
    }
    high
}

fn pricing_note(ctx: &Ctx, desired: Decimal, revenue: Decimal, at: &PricingScenario) -> String {
    let mut costs = if ctx.costs.is_zero() {
        String::new()
    } else {
        format!(" e de {} de custos", brl(ctx.costs))
    };
    if !ctx.input.reserve_rate.is_zero() {
        costs.push_str(&format!(
            ", guardando {} da receita como reserva",
            percent(ctx.input.reserve_rate)
        ));
    }
    let base = format!(
        "Receita mensal constante para sobrar {} por mês (pró-labore líquido mais lucro), depois \
         dos impostos{costs}",
        brl(desired)
    );
    let annual = revenue * Decimal::from(12);
    match (ctx.input.regime, &at.das) {
        (TaxRegime::Simples, Some(das)) => {
            let simples = ctx.tables.simples;
            let pro_labore = if ctx.input.activity == SimplesActivity::FatorR {
                format!(
                    "pró-labore automático de {} para o Fator R",
                    brl(at.cost.owner.gross)
                )
            } else {
                format!("pró-labore automático de {}", brl(at.cost.owner.gross))
            };
            let mut note = format!(
                "{base}, no Simples Nacional: RBT12 de 12 × a receita ({}), Anexo {}, {}ª faixa, \
                 e {pro_labore}; inclui o DAS, o INSS e o IRRF do pró-labore.",
                brl(annual),
                das.annex.label(),
                das.bracket
            );
            if das.above_ceiling {
                note.push_str(&format!(
                    " Esse faturamento passa do teto do Simples Nacional ({} por ano).",
                    brl(simples.epp_limit)
                ));
            } else if annual > simples.me_limit {
                note.push_str(&format!(
                    " Com esse faturamento a empresa é EPP (acima de {} por ano).",
                    brl(simples.me_limit)
                ));
            }
            note
        }
        (TaxRegime::Presumido, _) => format!(
            "{base}, no Lucro Presumido: pró-labore de {}, ISS de {} e IRPJ e CSLL como média \
             mensal dos trimestres.",
            brl(at.cost.owner.gross),
            percent(ctx.input.iss_rate)
        ),
        _ => {
            let mut note = format!(
                "Receita mensal constante para sobrar {} por mês no MEI, depois do DAS-MEI de \
                 {}{costs}.",
                brl(desired),
                brl(at.cost.company_total())
            );
            let limit = ctx.tables.mei.annual_limit;
            if annual > limit {
                note.push_str(&format!(
                    " Atenção: são {} por ano, acima do limite do MEI ({}).",
                    brl(annual),
                    brl(limit)
                ));
            }
            if mei::it_blocked(ctx.input.activity, ctx.tables) {
                note.push_str(" Serviços de TI não podem ser MEI.");
            }
            note
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_the_first_revenue_that_reaches_the_target() {
        // net = 80% of the revenue − 1.000,00 (in cents): 4.200,00 needs 5.200,00 / 0,8 = 6.500,00.
        let net = |r: i64| r * 4 / 5 - 100_000;
        assert_eq!(required_revenue(420_000, 0, net), Some(650_000));
        // A tax that jumps by R$ 10 at 8.200,00: nets from 7.190,00 to 7.199,99 are reached
        // both below the jump and after it; the smallest revenue (below) is found.
        let jumpy = |r: i64| if r >= 820_000 { r - 1_000 } else { r } - 100_000;
        assert_eq!(required_revenue(600_000, 0, jumpy), Some(700_000));
        assert_eq!(required_revenue(719_600, 0, jumpy), Some(819_600));
        // 7.200,00 is only reached after the jump: 8.210,00.
        assert_eq!(required_revenue(720_000, 0, jumpy), Some(821_000));
    }

    #[test]
    fn search_keeps_the_local_property_when_taxes_decrease() {
        // Taxes drop by R$ 50 at 1.000,00: nets from 950,00 are reached twice.
        let dropping = |r: i64| if r >= 100_000 { r } else { r - 5_000 };
        let desired = 96_000;
        let found = required_revenue(desired, 0, dropping).unwrap();
        assert!(dropping(found) >= desired);
        assert!(dropping(found - 1) < desired);
    }
}
