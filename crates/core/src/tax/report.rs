//! Assembly of the tax report: the estimate of a typical month, its pt-BR warnings and the
//! mapping to the contract types.

use super::mei;
use super::money::{brl, percent, ratio, to_cents, to_f64};
use super::month::{Date, Month};
use super::payroll::{self, ProLaboreTaxes};
use super::revenue::Revenue;
use super::scenarios::{
    self, Ctx, MeiOutlook, LABEL_MEI, LABEL_PRESUMIDO, LABEL_SIMPLES_III, LABEL_SIMPLES_V,
};
use super::simples::{self, Annex, Das};
use super::tables::{CoverageStatus, OpeningRule, Tables, TaxData};
use super::window;
use super::Input;
use crate::model::{
    LeftoverReport, MeiReport, MeiStatus, ProLaboreReport, RegimeCost, RevenueSummary,
    SimplesActivity, SimplesReport, TaxRegime, TaxReport,
};
use crate::text::count_label;
use rust_decimal::Decimal;

/// A gap in the tables of a month: outside the months covered, or a table not in force (label,
/// start of the version used, note).
#[derive(PartialEq)]
enum Gap {
    Coverage(CoverageStatus),
    Table(&'static str, Date, Option<&'static str>),
}

/// Warnings about the tables of the reference month and, since each month of the cash flow uses
/// its own tables, about the other months considered: once per gap the reference month's warnings
/// do not already name, naming the months it affects.
pub(crate) fn coverage_warnings(
    data: &'static TaxData,
    tables: &Tables,
    reference: Month,
    months: impl IntoIterator<Item = Month>,
    warnings: &mut Vec<String>,
) {
    reference_coverage_warnings(tables, reference, warnings);
    // Only a reference month within the tables warns about its fallbacks; outside them it gets the
    // coverage note alone, so the other months' fallbacks are all still to be told.
    let reference_gaps: Vec<Gap> = match tables.coverage {
        CoverageStatus::Within => tables
            .fallbacks
            .iter()
            .map(|f| Gap::Table(f.label, f.used_from, f.note))
            .collect(),
        _ => Vec::new(),
    };
    let mut groups: Vec<(Gap, Vec<Month>)> = Vec::new();
    for month in months.into_iter().filter(|m| *m != reference) {
        let month_tables = data.tables(month);
        let gaps = match month_tables.coverage {
            CoverageStatus::Within => month_tables
                .fallbacks
                .iter()
                .map(|f| Gap::Table(f.label, f.used_from, f.note))
                .filter(|gap| !reference_gaps.contains(gap))
                .collect(),
            status if status == tables.coverage => Vec::new(),
            status => vec![Gap::Coverage(status)],
        };
        for gap in gaps {
            match groups.iter_mut().find(|(known, _)| *known == gap) {
                Some((_, affected)) => affected.push(month),
                None => groups.push((gap, vec![month])),
            }
        }
    }
    for (gap, affected) in groups {
        let label = match affected.as_slice() {
            [only] => only.display(),
            [first, .., last] => format!("{} a {}", first.display(), last.display()),
            [] => continue,
        };
        warnings.push(match gap {
            Gap::Coverage(CoverageStatus::Before(first, note)) => format!(
                "{note} O fluxo mês a mês de {label} (antes de {first}) usa as tabelas mais \
                 próximas e pode não refletir as regras da época."
            ),
            Gap::Coverage(CoverageStatus::After(last, note)) => format!(
                "{note} O fluxo mês a mês de {label} (depois de {last}) usa as tabelas mais \
                 recentes incluídas."
            ),
            Gap::Coverage(CoverageStatus::Within) => continue,
            Gap::Table(table, used_from, note) => fallback_warning(&label, table, used_from, note),
        });
    }
}

/// "Tabela não incluída para <months>: <table>; foi usada a versão vigente a partir de <date>."
fn fallback_warning(
    months: &str,
    table: &str,
    used_from: Date,
    note: Option<&'static str>,
) -> String {
    let note = note.map(|note| format!(" {note}")).unwrap_or_default();
    format!(
        "Tabela não incluída para {months}: {table}; foi usada a versão vigente a partir de \
         {used_from}.{note}"
    )
}

/// Warnings about the tables used for the reference month.
fn reference_coverage_warnings(tables: &Tables, reference: Month, warnings: &mut Vec<String>) {
    match tables.coverage {
        CoverageStatus::Within => {
            for fallback in &tables.fallbacks {
                warnings.push(fallback_warning(
                    &reference.display(),
                    fallback.label,
                    fallback.used_from,
                    fallback.note,
                ));
            }
        }
        CoverageStatus::Before(first, note) => warnings.push(format!(
            "{note} O mês de referência ({}) é anterior a {first}: o cálculo usa as tabelas mais \
             próximas e pode não refletir as regras da época.",
            reference.display()
        )),
        CoverageStatus::After(last, note) => warnings.push(format!(
            "{note} O mês de referência ({}) é posterior a {last}: o cálculo usa as tabelas mais \
             recentes incluídas.",
            reference.display()
        )),
    }
}

/// The report of the months considered (at least one).
pub(crate) fn build(
    input: &Input,
    tables: &Tables,
    data: &'static TaxData,
    revenue: &Revenue,
    warnings: Vec<String>,
) -> TaxReport {
    Estimate::new(input, tables, data, revenue).into_report(warnings)
}

/// What is computed for a typical month before it becomes the contract report.
struct Estimate<'a> {
    ctx: Ctx<'a>,
    revenue: &'a Revenue,
    /// Monthly revenue the automatic pró-labore is based on (RBT12 / 12), how many months that
    /// RBT12 covers, and the month that required it (`None`: the typical month).
    basis: Decimal,
    basis_months: u32,
    basis_month: Option<Month>,
    data: &'static TaxData,
    /// Pró-labore in use (the user's or the automatic one) and what is withheld from it.
    owner: ProLaboreTaxes,
    fs12: Decimal,
    fator_r: Decimal,
    /// DAS of the typical month in the annex given by the Fator R.
    das: Das,
    /// Rates inside the DAS of the reference month itself, the ones its NFS-e states.
    nfse_rates: Vec<(String, Decimal)>,
    /// MEI: the year to date against the limit, and where the year is heading.
    status: MeiStatus,
    outlook: MeiOutlook,
    /// DAS-MEI by tax.
    mei_das: Vec<(String, Decimal)>,
}

impl<'a> Estimate<'a> {
    fn new(
        input: &'a Input,
        tables: &'a Tables,
        data: &'static TaxData,
        revenue: &'a Revenue,
    ) -> Self {
        let simples_table = tables.simples;
        let twelve = Decimal::from(12);

        // Pró-labore: the user's, or the automatic amount: the smallest that brings the Fator R to
        // 28% in the typical month (RBT12 / 12, the average when fewer than 12 months are
        // considered) and in every month considered, each with the RBT12 of its window (the window
        // of the tables of that month), so no month falls to Anexo V when its revenue is above the
        // average. The floor is the minimum wage of the reference month, the latest one.
        let typical_basis = revenue.rbt12 / twelve;
        let typical_auto = payroll::auto_pro_labore(typical_basis, input.payroll, tables);
        let demanding = revenue
            .months
            .keys()
            .map(|month| {
                let rbt12 = window::rbt12(input, *month, data.tables(*month).simples);
                let needed = payroll::auto_pro_labore(rbt12.amount / twelve, input.payroll, tables);
                (needed, *month, rbt12)
            })
            .max_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let (auto_pro_labore, basis, basis_months, basis_month) = match demanding {
            Some((needed, month, rbt12)) if needed > typical_auto => (
                needed,
                rbt12.amount / twelve,
                u32::try_from(rbt12.months.max(1)).unwrap_or(12),
                Some(month),
            ),
            _ => (typical_auto, typical_basis, revenue.count.min(12), None),
        };
        let pro_labore = input.user_pro_labore.unwrap_or(auto_pro_labore);
        // The pró-labore of the current regime: the Lucro Presumido has no Fator R, so its
        // automatic amount is the minimum wage (as in its taxes and cash flow); the Simples
        // estimates below keep the Fator R amount.
        let owner_pro_labore = match (input.user_pro_labore, input.regime) {
            (None, TaxRegime::Presumido) => tables.minimum_wage,
            _ => pro_labore,
        };
        let owner = payroll::pro_labore_taxes(owner_pro_labore, input.dependents, tables);
        let basis_month = basis_month.filter(|_| input.regime != TaxRegime::Presumido);

        // Simples Nacional with the pró-labore in use.
        let fs12 = simples::fs12(pro_labore, input.payroll);
        let (fator_r, annex) =
            simples::fator_r_and_annex(input.activity, fs12, revenue.rbt12, simples_table);
        let das = simples::das(simples_table, annex, revenue.rbt12, revenue.average);
        // The NFS-e of a month states the rates of that month's own DAS: the bracket of its RBT12
        // window and the annex of its Fator R, as the PGDAS-D computes them (LC 123/2006, art. 23,
        // § 2º), not the typical month's.
        let reference_revenue = input
            .revenue_history
            .get(&input.reference)
            .copied()
            .unwrap_or_default();
        let nfse_rates =
            window::simples_month(input, data, input.reference, reference_revenue, pro_labore)
                .rates;

        // MEI: year to date, and the year to date plus the average for each remaining month.
        let limit = mei::limit(tables, input.reference, input.opening);
        let tolerance = tables.mei.tolerance_ratio;
        let projection =
            revenue.year_to_date + revenue.average * Decimal::from(12 - input.reference.month);

        // Costs of the typical month: the reference month's, never an average. The fixed costs in
        // effect in it (a dated item that ended counts nothing, one that started counts in full)
        // and its variable costs (an expense counts in the month of its note). The cash flow has
        // each month's own.
        let (fixed_costs, variable_costs) = input.costs_in(input.reference);

        Estimate {
            ctx: Ctx {
                input,
                tables,
                revenue: revenue.average,
                rbt12: revenue.rbt12,
                auto_pro_labore,
                fixed_costs,
                variable_costs,
                costs: fixed_costs + variable_costs,
            },
            revenue,
            basis,
            basis_months,
            basis_month,
            data,
            owner,
            fs12,
            fator_r,
            das,
            nfse_rates,
            status: mei::status(revenue.year_to_date, limit, tolerance),
            outlook: MeiOutlook {
                limit,
                projection,
                projected: mei::status(projection, limit, tolerance),
            },
            mei_das: mei::das(tables),
        }
    }

    fn into_report(self, mut warnings: Vec<String>) -> TaxReport {
        let ctx = &self.ctx;
        let (input, tables, revenue) = (ctx.input, ctx.tables, self.revenue);
        let ytd = revenue.year_to_date;

        let comparison = scenarios::comparison(ctx, &self.outlook);
        let (leftover, distribution) = scenarios::leftover(ctx, &self.das, self.owner.gross);
        let cash_flow = scenarios::cash_flow(ctx, self.data, self.owner.gross, &revenue.months);
        let pricing = input
            .desired_net
            .and_then(|desired| scenarios::pricing(ctx, desired));

        self.push_warnings(&mut warnings);
        let regime_label = match input.regime {
            TaxRegime::Mei => LABEL_MEI,
            TaxRegime::Simples => "Simples Nacional",
            TaxRegime::Presumido => LABEL_PRESUMIDO,
        };
        if distribution.is_unverified() {
            warnings.push(format!(
                "O limite de lucro isento do {regime_label} é estimativa; a regra não foi \
                 reconferida na fonte: {}.",
                distribution.source()
            ));
        }
        if input.desired_net.is_some() && pricing.is_none() {
            warnings.push(
                "Não foi possível calcular o faturamento necessário para o valor desejado."
                    .to_string(),
            );
        }
        let mut sources = tables.sources.clone();
        sources.push(distribution.source_line(&format!("Lucro isento ({regime_label})")));
        if input.regime == TaxRegime::Simples {
            sources.push(if self.lagged_window() {
                "RBT12 e Fator R de cada mês do fluxo (12 meses anteriores; a partir de 2027, os 12 \
                 meses anteriores ao mês anterior, com a 1ª faixa e Fator R de 0,28 nos 2 primeiros \
                 meses de atividade): LC 123/2006, art. 18, §§ 1º, 5º-K e 24 (redação da LC 214/2025); Res. CGSN \
                 140/2018, arts. 21, 22 e 26 (redação da Res. CGSN 190/2026)."
                    .to_string()
            } else {
                "RBT12 e Fator R de cada mês do fluxo (12 meses anteriores; proporcional no começo \
                 da atividade): Res. CGSN 140/2018, arts. 22 e 26."
                    .to_string()
            });
        }

        let (das, outlook, owner) = (&self.das, &self.outlook, &self.owner);
        TaxReport {
            reference_month: input.reference.iso(),
            revenue: revenue.to_contract(outlook.projection),
            mei: MeiReport {
                limit_cents: to_cents(outlook.limit),
                year_to_date_cents: to_cents(ytd),
                used_ratio: to_f64(ratio(ytd, outlook.limit)),
                status: self.status,
                year_projection_cents: to_cents(outlook.projection),
                projection_ratio: to_f64(ratio(outlook.projection, outlook.limit)),
                projected_status: outlook.projected,
                das_cents: to_cents(self.mei_das.iter().map(|(_, amount)| *amount).sum()),
                das_split: scenarios::tax_shares(&self.mei_das),
                it_services_allowed: tables.mei.it_services_allowed,
            },
            simples: SimplesReport {
                fator_r: to_f64(self.fator_r),
                fs12_cents: to_cents(self.fs12),
                annex: das.annex.label().to_string(),
                bracket: das.bracket,
                nominal_rate: to_f64(das.nominal_rate),
                deduction_cents: to_cents(das.deduction),
                effective_rate: to_f64(das.effective_rate),
                das_cents: to_cents(das.total),
                split: scenarios::tax_shares(&das.split),
                rates: scenarios::tax_rates(&das.rates),
                nfse_rates: scenarios::tax_rates(&self.nfse_rates),
                pro_labore_for_annex_iii_cents: to_cents(ctx.auto_pro_labore),
                me_limit_remaining_cents: to_cents(
                    (tables.simples.me_limit - ytd).max(Decimal::ZERO),
                ),
                me_limit_cents: to_cents(tables.simples.me_limit),
                epp_limit_cents: to_cents(tables.simples.epp_limit),
            },
            pro_labore: ProLaboreReport {
                gross_cents: to_cents(owner.gross),
                automatic: input.user_pro_labore.is_none(),
                basis_average_cents: to_cents(self.basis),
                basis_months: self.basis_months,
                basis_month: self.basis_month.map(Month::iso),
                inss_cents: to_cents(owner.inss),
                irrf_cents: to_cents(owner.irrf),
                irrf_reduction_cents: to_cents(owner.reduction),
                net_cents: to_cents(owner.net()),
            },
            comparison,
            leftover,
            pricing,
            cash_flow,
            warnings,
            sources,
        }
    }

    /// How the PGDAS-D taxes the first 13 months of activity from `opening`, each month by the
    /// rule of its own tables, as the cash flow does: a company opened in 2026 has its 2027 months
    /// under the 2027 rule.
    fn opening_rules(&self, opening: Month) -> String {
        let rule_at = |month: Month| self.data.tables(month).simples.opening.as_ref();
        let describe = |rule: Option<&OpeningRule>| match rule {
            Some(rule) => format!(
                "nos {} primeiros meses de atividade o PGDAS-D usa a 1ª faixa e Fator R de {}, e \
                 até o 13º mês a média dos meses anteriores ao mês anterior × 12",
                rule.first_bracket_months,
                percent(rule.fator_r)
            ),
            None => "nos primeiros meses o PGDAS-D usa a receita do 1º mês × 12 e depois a média \
                     dos meses anteriores × 12"
                .to_string(),
        };
        let first = rule_at(opening);
        let change = (1..13)
            .map(|k| opening.plus(k))
            .find(|month| rule_at(*month).is_some() != first.is_some());
        match change {
            None => describe(first),
            Some(at) => format!(
                "até {}, {}; a partir de {}, {}",
                at.plus(-1).display(),
                describe(first),
                at.display(),
                describe(rule_at(at))
            ),
        }
    }

    /// Some month considered has the RBT12 window of 2027 on (the 12 months before the previous
    /// one).
    fn lagged_window(&self) -> bool {
        self.revenue
            .months
            .keys()
            .any(|month| self.data.tables(*month).simples.rbt12_window_offset > 1)
    }

    /// Assumptions and limits of the estimate, only when they matter.
    fn push_warnings(&self, warnings: &mut Vec<String>) {
        let (input, tables, revenue) = (self.ctx.input, self.ctx.tables, self.revenue);
        let months = count_label(
            usize::try_from(revenue.count).unwrap_or(usize::MAX),
            "mês",
            "meses",
        );
        // Only the Simples looks at an RBT12 month by month.
        let flow = match (input.regime, self.lagged_window()) {
            (TaxRegime::Simples, true) => {
                "; no fluxo mês a mês, cada mês usa a receita dos 12 meses anteriores (a partir de \
                 2027, dos 12 meses anteriores ao mês anterior), como o PGDAS-D"
            }
            (TaxRegime::Simples, false) => {
                "; no fluxo mês a mês, cada mês usa a receita dos 12 meses anteriores, como o PGDAS-D"
            }
            _ => "",
        };
        warnings.push(format!(
            "O mês típico usa a média de {months} ({} por mês){flow}.",
            brl(revenue.average)
        ));
        if revenue.annualized {
            // Until 2026 the typical month follows the opening rule; from 2027 that rule is the 1st
            // bracket in the first months, which the typical month does not model.
            let like = if tables.simples.opening.is_none() {
                ", como na regra de início de atividade"
            } else {
                ""
            };
            warnings.push(format!(
                "Com menos de 12 meses, o RBT12 foi estimado como a média × 12 ({}){like}.",
                brl(revenue.rbt12)
            ));
        }
        if let Some(opening) = input.opening {
            let since = opening.months_until(input.reference);
            if opening.year == input.reference.year || (0..13).contains(&since) {
                warnings.push(format!(
                    "Regras do início de atividade simplificadas: no ano de abertura os limites da \
                     ME e do Simples Nacional são proporcionais aos meses de atividade, e {}.",
                    self.opening_rules(opening)
                ));
            }
        }
        match input.regime {
            TaxRegime::Mei => self.mei_warnings(warnings),
            TaxRegime::Simples => self.me_limit_warnings(warnings),
            TaxRegime::Presumido => {}
        }
        self.simples_bracket_warnings(warnings);
        if input.activity == SimplesActivity::FatorR {
            let window = if self.lagged_window() {
                "nos 12 meses anteriores (a partir de 2027, nos 12 meses anteriores ao mês anterior)"
            } else {
                "nos 12 meses anteriores"
            };
            warnings.push(format!(
                "O Fator R supõe pró-labore constante: o PGDAS-D usa a folha paga {window}, então \
                 uma mudança no pró-labore leva meses para aparecer no Fator R."
            ));
        }
        if let Some(user) = input.user_pro_labore {
            if user < tables.minimum_wage && input.regime != TaxRegime::Mei {
                warnings.push(format!(
                    "Pró-labore informado ({}) abaixo do salário mínimo ({}): a competência pode \
                     não contar para a Previdência sem complementação.",
                    brl(user),
                    brl(tables.minimum_wage)
                ));
            }
        }
        if input.payroll > Decimal::ZERO {
            warnings.push(format!(
                "A folha informada ({} por mês) entra no Fator R, mas não é descontada da sobra do \
                 mês: inclua salários e encargos nos custos mensais.",
                brl(input.payroll)
            ));
        }
    }

    /// Current regime MEI: IT services and the annual limit.
    fn mei_warnings(&self, warnings: &mut Vec<String>) {
        let (input, tables, outlook) = (self.ctx.input, self.ctx.tables, &self.outlook);
        if let Some(note) = &tables.mei.note {
            warnings.push(note.clone());
        }
        if mei::it_blocked(input.activity, tables) {
            warnings.push(
                "Desenvolvimento de software e serviços de TI não podem ser MEI (LC 123/2006, \
                 art. 18-A, § 4º, I; Res. CGSN 140/2018, art. 100-A, VI): exercer atividade fora \
                 do Anexo XI obriga a comunicar o desenquadramento."
                    .to_string(),
            );
        }
        let tolerance = percent(tables.mei.tolerance_ratio);
        let limit = brl(outlook.limit);
        let ytd = brl(self.revenue.year_to_date);
        let projection = brl(outlook.projection);
        match (self.status, outlook.projected) {
            (MeiStatus::UpToTolerance, _) => warnings.push(format!(
                "A receita do ano ({ytd}) passou do limite do MEI ({limit}) em até {tolerance}: a \
                 empresa sai do MEI em 1º de janeiro do ano seguinte e paga a diferença com o DAS \
                 de janeiro."
            )),
            (MeiStatus::AboveTolerance, _) => warnings.push(format!(
                "A receita do ano ({ytd}) passou do limite do MEI ({limit}) em mais de \
                 {tolerance}: o desenquadramento é retroativo a janeiro (ou à abertura), com os \
                 meses recalculados pelo Simples Nacional."
            )),
            (MeiStatus::Within, MeiStatus::UpToTolerance) => warnings.push(format!(
                "No ritmo atual, a receita do ano chega a {projection}, acima do limite do MEI \
                 ({limit}) em até {tolerance}: a empresa sairia do MEI em 1º de janeiro do ano \
                 seguinte."
            )),
            (MeiStatus::Within, MeiStatus::AboveTolerance) => warnings.push(format!(
                "No ritmo atual, a receita do ano chega a {projection}, mais de {tolerance} acima \
                 do limite do MEI ({limit}): o desenquadramento seria retroativo a janeiro (ou à \
                 abertura)."
            )),
            (MeiStatus::Within, MeiStatus::Within) => {}
        }
    }

    /// Current regime Simples: the ME limit (above it the company is an EPP).
    fn me_limit_warnings(&self, warnings: &mut Vec<String>) {
        let simples = self.ctx.tables.simples;
        let ytd = self.revenue.year_to_date;
        if ytd > simples.me_limit {
            warnings.push(format!(
                "A receita do ano ({}) passou do limite da ME ({}): a empresa passa a ser EPP \
                 (LC 123/2006, art. 3º, II), e o Simples Nacional vale até {} por ano.",
                brl(ytd),
                brl(simples.me_limit),
                brl(simples.epp_limit)
            ));
        } else if self.outlook.projection > simples.me_limit {
            warnings.push(format!(
                "No ritmo atual, a receita do ano chega a {} e passa do limite da ME ({}): a \
                 empresa passa a ser EPP (LC 123/2006, art. 3º, II).",
                brl(self.outlook.projection),
                brl(simples.me_limit)
            ));
        }
    }

    /// The Simples estimate in the 6th bracket or above the ceiling (whatever the regime, since
    /// the Simples section is always shown).
    fn simples_bracket_warnings(&self, warnings: &mut Vec<String>) {
        let (input, simples, das) = (self.ctx.input, self.ctx.tables.simples, &self.das);
        if das.above_ceiling {
            warnings.push(format!(
                "RBT12 de {} acima do teto do Simples Nacional ({}): a empresa não pode continuar \
                 no Simples; os valores usam a 6ª faixa só como referência.",
                brl(self.revenue.rbt12),
                brl(simples.epp_limit)
            ));
        } else if das.iss_outside {
            // The 6th bracket alone keeps them in the DAS, at the rate of the 5th; they leave it
            // when the company is barred for passing the sublimit, which the estimate assumes.
            let (taxes, ibs, before) = if das.ibs_outside {
                (
                    "o ISS e o IBS",
                    " e o IBS pago à parte não entra nesta estimativa",
                    "os dois ficam",
                )
            } else {
                ("o ISS", "", "ele fica")
            };
            warnings.push(format!(
                "RBT12 de {} na 6ª faixa do Simples Nacional: a estimativa supõe a empresa já \
                 impedida de recolher {taxes} pelo Simples por ter passado do sublimite (Res. CGSN \
                 140/2018, art. 12): o ISS é pago à parte ao município (considerado a {}){ibs}. \
                 Antes do impedimento, {before} no DAS pelo percentual da 5ª faixa.",
                brl(self.revenue.rbt12),
                percent(input.iss_rate)
            ));
        }
    }
}

/// Report of zeros for when no month is considered.
pub(crate) fn zero_report(input: &Input, warnings: Vec<String>) -> TaxReport {
    let note = Some("Nenhum mês selecionado.".to_string());
    let zero_cost = |regime: &str, label: &str| RegimeCost {
        regime: regime.to_string(),
        label: label.to_string(),
        available: false,
        note: note.clone(),
        company_taxes_cents: 0,
        company_taxes: Vec::new(),
        pro_labore_cents: 0,
        owner_taxes_cents: 0,
        total_taxes_cents: 0,
        total_rate: 0.0,
        reserve_cents: 0,
        owner_net_cents: 0,
    };
    TaxReport {
        reference_month: input.reference.iso(),
        revenue: RevenueSummary {
            months: Vec::new(),
            months_count: 0,
            total_cents: 0,
            average_monthly_cents: 0,
            month_cents: 0,
            year_to_date_cents: 0,
            year_projection_cents: 0,
            rbt12_cents: 0,
            rbt12_annualized: false,
        },
        mei: MeiReport {
            limit_cents: 0,
            year_to_date_cents: 0,
            used_ratio: 0.0,
            status: MeiStatus::Within,
            year_projection_cents: 0,
            projection_ratio: 0.0,
            projected_status: MeiStatus::Within,
            das_cents: 0,
            das_split: Vec::new(),
            it_services_allowed: false,
        },
        simples: SimplesReport {
            fator_r: 0.0,
            fs12_cents: 0,
            annex: Annex::Iii.label().to_string(),
            bracket: 1,
            nominal_rate: 0.0,
            deduction_cents: 0,
            effective_rate: 0.0,
            das_cents: 0,
            split: Vec::new(),
            rates: Vec::new(),
            nfse_rates: Vec::new(),
            pro_labore_for_annex_iii_cents: 0,
            me_limit_remaining_cents: 0,
            me_limit_cents: 0,
            epp_limit_cents: 0,
        },
        pro_labore: ProLaboreReport {
            gross_cents: 0,
            automatic: input.user_pro_labore.is_none(),
            basis_average_cents: 0,
            basis_months: 0,
            basis_month: None,
            inss_cents: 0,
            irrf_cents: 0,
            irrf_reduction_cents: 0,
            net_cents: 0,
        },
        comparison: vec![
            zero_cost("mei", LABEL_MEI),
            zero_cost("simplesIii", LABEL_SIMPLES_III),
            zero_cost("simplesV", LABEL_SIMPLES_V),
            zero_cost("presumido", LABEL_PRESUMIDO),
        ],
        leftover: LeftoverReport {
            revenue_cents: 0,
            custom_revenue: false,
            company_taxes_cents: 0,
            pro_labore_inss_cents: 0,
            pro_labore_irrf_cents: 0,
            costs_cents: 0,
            variable_costs_cents: 0,
            reserve_cents: 0,
            leftover_cents: 0,
            tax_free_distribution_limit_cents: 0,
        },
        pricing: None,
        cash_flow: Vec::new(),
        warnings,
        sources: Vec::new(),
    }
}
