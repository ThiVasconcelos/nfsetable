//! Tax tables embedded from `crates/core/tax/br.json`: data model, validation and the choice of
//! the version in force in a reference month.
//!
//! Every versioned block carries its legal basis (`source`) and its validity (`valid_from`,
//! `valid_to`; `null` = still in force when the file was checked). Numbers are read into
//! [`Decimal`] from their JSON text, never through binary floating point arithmetic.

use super::month::{Date, Month};
use crate::error::CoreError;
use crate::model::{CnaeInfo, SimplesActivity, TaxCatalog};
use rust_decimal::Decimal;
use serde::{de, Deserialize, Deserializer};
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;
use std::sync::OnceLock;

const BR_JSON: &str = include_str!("../../tax/br.json");
const SCHEMA_VERSION: u32 = 1;

/// Canonical order of the taxes of a split (the order of the tables of LC 123/2006). Unknown
/// taxes (e.g. CBS and IBS from 2027) go after these, alphabetically.
const TAX_ORDER: [&str; 6] = ["irpj", "csll", "cofins", "pis", "cpp", "iss"];

static DATA: OnceLock<Result<TaxData, String>> = OnceLock::new();

/// The embedded tables, parsed and validated once.
pub(crate) fn data() -> Result<&'static TaxData, CoreError> {
    DATA.get_or_init(|| parse(BR_JSON))
        .as_ref()
        .map_err(|e| CoreError::Tax(e.clone()))
}

fn parse(json: &str) -> Result<TaxData, String> {
    let invalid = |e: String| format!("tabelas tributárias inválidas (br.json): {e}");
    let data: TaxData = serde_json::from_str(json).map_err(|e| invalid(e.to_string()))?;
    data.validate().map_err(invalid)?;
    Ok(data)
}

/// CNAE codes known to the tax calculations.
///
/// Panics only if the embedded `br.json` is invalid, which the test suite rules out.
pub(crate) fn catalog() -> TaxCatalog {
    let data = data().unwrap_or_else(|e| panic!("{e}"));
    let catalog = &data.cnae_catalog;
    TaxCatalog {
        cnaes: catalog
            .cnaes
            .iter()
            .map(|row| CnaeInfo {
                code: row.code.clone(),
                description: row.description.clone(),
                activity: row.activity,
                mei_allowed: row.mei_allowed,
                mei_occupation: row.mei_occupation.clone(),
                note: row.note.clone(),
            })
            .collect(),
        default_cnae: catalog.default_cnae.clone(),
    }
}

// ------------------------------------------------------------------ data model

#[derive(Debug, Deserialize)]
pub(crate) struct TaxData {
    schema_version: u32,
    coverage: Coverage,
    minimum_wage: Vec<MinimumWage>,
    inss: Vec<Inss>,
    irrf: Vec<Irrf>,
    irrf_reduction: Vec<IrrfReduction>,
    mei: Vec<Mei>,
    mei_das: Vec<MeiDas>,
    simples: Vec<Simples>,
    presumido: Vec<Presumido>,
    cnae_catalog: CnaeCatalog,
}

/// Months covered by the file; outside them the nearest tables are used, with a warning.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverage {
    valid_from: Date,
    valid_to: Date,
    /// pt-BR: what is missing before `valid_from`.
    before_note: String,
    /// pt-BR: what is missing after `valid_to`.
    after_note: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MinimumWage {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    #[serde(deserialize_with = "number")]
    pub monthly: Decimal,
}

/// INSS withheld from the pró-labore (contribuinte individual, 11% up to the ceiling).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Inss {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    #[serde(deserialize_with = "number")]
    pub ceiling: Decimal,
    #[serde(deserialize_with = "number")]
    pub pro_labore_rate: Decimal,
}

/// Monthly IRRF table on work income.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Irrf {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    /// Ascending; the last one has no upper limit.
    pub brackets: Vec<IrrfBracket>,
    #[serde(deserialize_with = "number")]
    pub dependent_deduction: Decimal,
    /// Used instead of the legal deductions when larger.
    #[serde(deserialize_with = "number")]
    pub simplified_discount: Decimal,
    /// Withholdings up to this amount are not made.
    #[serde(deserialize_with = "number")]
    pub min_withholding: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IrrfBracket {
    /// Inclusive upper limit of the base; `None` for the last bracket.
    #[serde(default, deserialize_with = "optional_number")]
    pub up_to: Option<Decimal>,
    #[serde(deserialize_with = "number")]
    pub rate: Decimal,
    #[serde(deserialize_with = "number")]
    pub deduction: Decimal,
}

/// Monthly IRRF reduction of Lei 15.270/2025, computed on the gross taxable income.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IrrfReduction {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    #[serde(deserialize_with = "number")]
    pub full_reduction_up_to: Decimal,
    #[serde(deserialize_with = "number")]
    pub max_reduction: Decimal,
    #[serde(deserialize_with = "number")]
    pub phase_out_up_to: Decimal,
    #[serde(deserialize_with = "number")]
    pub phase_out_constant: Decimal,
    #[serde(deserialize_with = "number")]
    pub phase_out_coefficient: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mei {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    #[serde(deserialize_with = "number")]
    pub annual_limit: Decimal,
    /// Limit per month of activity in the opening year (a started month counts in full).
    #[serde(deserialize_with = "number")]
    pub opening_year_monthly_limit: Decimal,
    /// Excess up to this ratio of the limit only takes effect the next year.
    #[serde(deserialize_with = "number")]
    pub tolerance_ratio: Decimal,
    pub it_services_allowed: bool,
    pub tax_free_distribution: Distribution,
}

/// Monthly DAS-MEI of a service provider: a share of the minimum wage (INSS) plus fixed ISS.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeiDas {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    #[serde(deserialize_with = "number")]
    pub inss_rate_on_minimum_wage: Decimal,
    #[serde(deserialize_with = "number")]
    pub iss: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Simples {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    /// Annual revenue limit of the ME; above it the company is an EPP.
    #[serde(deserialize_with = "number")]
    pub me_limit: Decimal,
    /// Annual revenue limit of the EPP (the ceiling of the Simples Nacional).
    #[serde(deserialize_with = "number")]
    pub epp_limit: Decimal,
    #[serde(deserialize_with = "number")]
    pub fator_r_threshold: Decimal,
    /// Fator R when there is payroll but no revenue in the window.
    #[serde(deserialize_with = "number")]
    pub fator_r_without_revenue: Decimal,
    /// Fator R when there is no payroll in the window.
    #[serde(deserialize_with = "number")]
    pub fator_r_without_payroll: Decimal,
    pub annex_iii: AnnexTable,
    pub annex_v: AnnexTable,
    pub tax_free_distribution: Distribution,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AnnexTable {
    /// Maximum effective ISS rate inside the DAS.
    #[serde(deserialize_with = "number")]
    pub iss_cap: Decimal,
    /// How the effective rate above the capped ISS is split among the other taxes.
    #[serde(deserialize_with = "shares")]
    pub iss_excess_split: Vec<(String, Option<Decimal>)>,
    /// Ascending by `rbt12_up_to`.
    pub brackets: Vec<SimplesBracket>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SimplesBracket {
    pub bracket: u32,
    /// Inclusive upper limit of the RBT12.
    #[serde(deserialize_with = "number")]
    pub rbt12_up_to: Decimal,
    #[serde(deserialize_with = "number")]
    pub nominal_rate: Decimal,
    #[serde(deserialize_with = "number")]
    pub deduction: Decimal,
    /// Share of each tax in the DAS; a `null` ISS means the ISS is paid outside the DAS.
    #[serde(deserialize_with = "shares")]
    pub split: Vec<(String, Option<Decimal>)>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Presumido {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    #[serde(default)]
    unverified: bool,
    #[serde(deserialize_with = "number")]
    pub irpj_rate: Decimal,
    #[serde(deserialize_with = "number")]
    pub irpj_presumption: Decimal,
    #[serde(deserialize_with = "number")]
    pub irpj_surcharge_rate: Decimal,
    /// The surcharge applies to the part of the monthly IRPJ base above this amount.
    #[serde(deserialize_with = "number")]
    pub irpj_surcharge_monthly_threshold: Decimal,
    #[serde(deserialize_with = "number")]
    pub csll_rate: Decimal,
    #[serde(deserialize_with = "number")]
    pub csll_presumption: Decimal,
    #[serde(deserialize_with = "number")]
    pub pis_rate: Decimal,
    #[serde(deserialize_with = "number")]
    pub cofins_rate: Decimal,
    /// Employer contribution on the pró-labore (outside the Simples).
    #[serde(deserialize_with = "number")]
    pub cpp_rate: Decimal,
    pub tax_free_distribution: Distribution,
}

/// Profit that can be distributed tax free without bookkeeping.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Distribution {
    source: String,
    /// Presumed profit ratio of services (Lei 9.249/1995, art. 15).
    #[serde(deserialize_with = "number")]
    pub presumption: Decimal,
    #[serde(default)]
    unverified: bool,
}

impl Distribution {
    /// "label: source" for the report sources.
    pub fn source_line(&self, label: &str) -> String {
        source_line(label, &self.source, self.unverified)
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// The rule was not rechecked in the primary source.
    pub fn is_unverified(&self) -> bool {
        self.unverified
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CnaeCatalog {
    source: String,
    valid_from: Date,
    valid_to: Option<Date>,
    default_cnae: String,
    cnaes: Vec<CnaeRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CnaeRow {
    code: String,
    description: String,
    activity: SimplesActivity,
    mei_allowed: bool,
    #[serde(default)]
    mei_occupation: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

// ------------------------------------------------------------------ versions

/// A block of the file valid for a period.
trait Versioned {
    fn valid_from(&self) -> Date;
    fn valid_to(&self) -> Option<Date>;
    fn source(&self) -> &str;
    fn unverified(&self) -> bool;

    fn contains(&self, date: Date) -> bool {
        self.valid_from() <= date && self.valid_to().is_none_or(|to| date <= to)
    }
}

macro_rules! versioned {
    ($($table:ty),+ $(,)?) => {$(
        impl Versioned for $table {
            fn valid_from(&self) -> Date {
                self.valid_from
            }
            fn valid_to(&self) -> Option<Date> {
                self.valid_to
            }
            fn source(&self) -> &str {
                &self.source
            }
            fn unverified(&self) -> bool {
                self.unverified
            }
        }
    )+};
}

versioned!(
    MinimumWage,
    Inss,
    Irrf,
    IrrfReduction,
    Mei,
    MeiDas,
    Simples,
    Presumido
);

/// The version in force on `date`, or else the nearest one (`true` = fallback): the latest that
/// started before `date`, or the first one when `date` precedes them all. `versions` is non-empty
/// and sorted (checked by [`TaxData::validate`]).
fn pick<T: Versioned>(versions: &[T], date: Date) -> (&T, bool) {
    if let Some(table) = versions.iter().find(|t| t.contains(date)) {
        return (table, false);
    }
    let nearest = versions
        .iter()
        .rev()
        .find(|t| t.valid_from() <= date)
        .unwrap_or(&versions[0]);
    (nearest, true)
}

fn source_line(label: &str, source: &str, unverified: bool) -> String {
    if unverified {
        format!("{label}: {source} ⚠ não verificado")
    } else {
        format!("{label}: {source}")
    }
}

/// Where a reference month stands against the months covered by the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoverageStatus {
    Within,
    /// Before the first covered day, with the pt-BR note of what is missing.
    Before(Date, &'static str),
    /// After the last covered day, with the pt-BR note of what is missing.
    After(Date, &'static str),
}

/// A table that is not in force in the reference month: its pt-BR name and the start of the
/// version used instead.
#[derive(Debug, Clone)]
pub(crate) struct Fallback {
    pub label: &'static str,
    pub used_from: Date,
}

/// The tables in force in a reference month.
pub(crate) struct Tables {
    pub minimum_wage: Decimal,
    pub inss: &'static Inss,
    pub irrf: &'static Irrf,
    /// `None` before the reduction of Lei 15.270/2025 exists.
    pub irrf_reduction: Option<&'static IrrfReduction>,
    pub mei: &'static Mei,
    pub mei_das: &'static MeiDas,
    pub simples: &'static Simples,
    pub presumido: &'static Presumido,
    pub coverage: CoverageStatus,
    /// Tables not in force in the reference month (the nearest version was used).
    pub fallbacks: Vec<Fallback>,
    /// Legal basis of the tables, pt-BR ("label: source").
    pub sources: Vec<String>,
}

struct Picker {
    date: Date,
    fallbacks: Vec<Fallback>,
    sources: Vec<String>,
}

impl Picker {
    fn pick<T: Versioned>(&mut self, label: &'static str, versions: &'static [T]) -> &'static T {
        let (table, fallback) = pick(versions, self.date);
        if fallback {
            self.fallbacks.push(Fallback {
                label,
                used_from: table.valid_from(),
            });
        }
        self.sources
            .push(source_line(label, table.source(), table.unverified()));
        table
    }
}

impl TaxData {
    /// The tables to use for `month`, chosen by its first day.
    pub(crate) fn tables(&'static self, month: Month) -> Tables {
        let date = month.first_day();
        let coverage = if date < self.coverage.valid_from {
            CoverageStatus::Before(self.coverage.valid_from, &self.coverage.before_note)
        } else if date > self.coverage.valid_to {
            CoverageStatus::After(self.coverage.valid_to, &self.coverage.after_note)
        } else {
            CoverageStatus::Within
        };
        let mut picker = Picker {
            date,
            fallbacks: Vec::new(),
            sources: Vec::new(),
        };
        let minimum_wage = picker.pick("Salário mínimo", &self.minimum_wage).monthly;
        let inss = picker.pick("INSS do pró-labore", &self.inss);
        let irrf = picker.pick("IRRF do pró-labore", &self.irrf);
        // The reduction did not exist before its first version: that is not a fallback.
        let irrf_reduction = (date >= self.irrf_reduction[0].valid_from)
            .then(|| picker.pick("Redução do IRRF", &self.irrf_reduction));
        let mei = picker.pick("MEI", &self.mei);
        let mei_das = picker.pick("DAS-MEI", &self.mei_das);
        let simples = picker.pick("Simples Nacional", &self.simples);
        let presumido = picker.pick("Lucro Presumido", &self.presumido);
        Tables {
            minimum_wage,
            inss,
            irrf,
            irrf_reduction,
            mei,
            mei_das,
            simples,
            presumido,
            coverage,
            fallbacks: picker.fallbacks,
            sources: picker.sources,
        }
    }

    /// Structural checks, so a bad edit of `br.json` fails the tests instead of skewing results.
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "schema_version {} não suportada (esperada {SCHEMA_VERSION})",
                self.schema_version
            ));
        }
        if self.coverage.valid_from > self.coverage.valid_to {
            return Err("coverage: valid_from depois de valid_to".into());
        }
        check_versions("minimum_wage", &self.minimum_wage)?;
        check_versions("inss", &self.inss)?;
        check_versions("irrf", &self.irrf)?;
        check_versions("irrf_reduction", &self.irrf_reduction)?;
        check_versions("mei", &self.mei)?;
        check_versions("mei_das", &self.mei_das)?;
        check_versions("simples", &self.simples)?;
        check_versions("presumido", &self.presumido)?;
        for irrf in &self.irrf {
            check_irrf(irrf)?;
        }
        for simples in &self.simples {
            check_annex("simples.annex_iii", &simples.annex_iii)?;
            check_annex("simples.annex_v", &simples.annex_v)?;
        }
        let distributions = self
            .mei
            .iter()
            .map(|m| &m.tax_free_distribution)
            .chain(self.simples.iter().map(|s| &s.tax_free_distribution))
            .chain(self.presumido.iter().map(|p| &p.tax_free_distribution));
        for distribution in distributions {
            if distribution.source.trim().is_empty() {
                return Err("tax_free_distribution sem source".into());
            }
        }
        self.check_catalog()
    }

    fn check_catalog(&self) -> Result<(), String> {
        let catalog = &self.cnae_catalog;
        if catalog.source.trim().is_empty() {
            return Err("cnae_catalog sem source".into());
        }
        if catalog.valid_to.is_some_and(|to| to < catalog.valid_from) {
            return Err("cnae_catalog: valid_from depois de valid_to".into());
        }
        let mut codes = HashSet::new();
        for row in &catalog.cnaes {
            if row.code.trim().is_empty() || row.description.trim().is_empty() {
                return Err("cnae_catalog: código ou descrição vazios".into());
            }
            if !codes.insert(row.code.as_str()) {
                return Err(format!("cnae_catalog: código repetido {}", row.code));
            }
        }
        if !codes.contains(catalog.default_cnae.as_str()) {
            return Err(format!(
                "cnae_catalog: default_cnae {} não está na lista",
                catalog.default_cnae
            ));
        }
        Ok(())
    }
}

fn check_versions<T: Versioned>(name: &str, versions: &[T]) -> Result<(), String> {
    if versions.is_empty() {
        return Err(format!("{name}: nenhuma versão"));
    }
    for (i, version) in versions.iter().enumerate() {
        if version.source().trim().is_empty() {
            return Err(format!("{name}[{i}]: source vazio"));
        }
        if version
            .valid_to()
            .is_some_and(|to| to < version.valid_from())
        {
            return Err(format!("{name}[{i}]: valid_from depois de valid_to"));
        }
        if let Some(next) = versions.get(i + 1) {
            match version.valid_to() {
                Some(to) if to < next.valid_from() => {}
                _ => {
                    return Err(format!(
                        "{name}[{i}]: as versões precisam estar em ordem e sem sobreposição"
                    ))
                }
            }
        }
    }
    Ok(())
}

fn check_irrf(irrf: &Irrf) -> Result<(), String> {
    let Some((last, others)) = irrf.brackets.split_last() else {
        return Err("irrf: tabela sem faixas".into());
    };
    if last.up_to.is_some() || others.iter().any(|b| b.up_to.is_none()) {
        return Err("irrf: só a última faixa fica sem limite (up_to null)".into());
    }
    let limits: Vec<Decimal> = others.iter().filter_map(|b| b.up_to).collect();
    if limits.windows(2).any(|w| w[0] >= w[1]) {
        return Err("irrf: faixas fora de ordem".into());
    }
    Ok(())
}

fn check_annex(name: &str, annex: &AnnexTable) -> Result<(), String> {
    let tolerance = Decimal::new(1, 4);
    if annex.brackets.is_empty() {
        return Err(format!("{name}: sem faixas"));
    }
    if annex
        .iss_excess_split
        .iter()
        .any(|(tax, share)| tax == "iss" || share.is_none())
    {
        return Err(format!(
            "{name}: iss_excess_split não pode ter ISS nem valores nulos"
        ));
    }
    if (share_sum(&annex.iss_excess_split) - Decimal::ONE).abs() > tolerance {
        return Err(format!("{name}: iss_excess_split não soma 100%"));
    }
    for (i, bracket) in annex.brackets.iter().enumerate() {
        if bracket.bracket as usize != i + 1 {
            return Err(format!(
                "{name}: faixas precisam ser numeradas a partir de 1"
            ));
        }
        if i > 0 && annex.brackets[i - 1].rbt12_up_to >= bracket.rbt12_up_to {
            return Err(format!("{name}: faixas fora de ordem"));
        }
        if (share_sum(&bracket.split) - Decimal::ONE).abs() > tolerance {
            return Err(format!(
                "{name}: a repartição da faixa {} não soma 100%",
                bracket.bracket
            ));
        }
    }
    Ok(())
}

fn share_sum(shares: &[(String, Option<Decimal>)]) -> Decimal {
    shares.iter().filter_map(|(_, share)| *share).sum()
}

// ------------------------------------------------------------------ numbers

fn decimal_from_number(number: &serde_json::Number) -> Result<Decimal, String> {
    let text = number.to_string();
    Decimal::from_str(&text)
        .or_else(|_| Decimal::from_scientific(&text))
        .map_err(|_| format!("número inválido: {text}"))
}

fn number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
    let number = serde_json::Number::deserialize(deserializer)?;
    decimal_from_number(&number).map_err(de::Error::custom)
}

fn optional_number<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error> {
    Option::<serde_json::Number>::deserialize(deserializer)?
        .map(|number| decimal_from_number(&number))
        .transpose()
        .map_err(de::Error::custom)
}

/// A `{ "tax": share }` object, in the canonical tax order.
fn shares<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<(String, Option<Decimal>)>, D::Error> {
    let map = BTreeMap::<String, Option<serde_json::Number>>::deserialize(deserializer)?;
    let mut shares = map
        .into_iter()
        .map(|(tax, share)| {
            let share = share.map(|n| decimal_from_number(&n)).transpose()?;
            Ok((tax, share))
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(de::Error::custom)?;
    // Stable: unknown taxes keep the alphabetical order of the map.
    shares.sort_by_key(|(tax, _)| {
        TAX_ORDER
            .iter()
            .position(|known| known == tax)
            .unwrap_or(TAX_ORDER.len())
    });
    Ok(shares)
}

/// The tables of a month known to be valid (tests).
#[cfg(test)]
pub(crate) fn tables_for(text: &str) -> Tables {
    data()
        .expect("valid embedded tables")
        .tables(super::month::month(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    use crate::tax::month::month;

    #[test]
    fn embedded_tables_are_valid() {
        let data = data().expect("br.json must parse and validate");
        assert_eq!(data.minimum_wage.len(), 2);
        assert!(!data.cnae_catalog.cnaes.is_empty());
    }

    #[test]
    fn numbers_are_exact() {
        let data = data().unwrap();
        assert_eq!(data.irrf_reduction[0].phase_out_coefficient, dec!(0.133145));
        assert_eq!(data.inss[1].ceiling, dec!(8475.55));
        let bracket = &data.simples[0].annex_iii.brackets[4];
        assert_eq!(bracket.nominal_rate, dec!(0.21));
        assert_eq!(bracket.deduction, dec!(125640));
        let taxes: Vec<&str> = bracket.split.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(taxes, ["irpj", "csll", "cofins", "pis", "cpp", "iss"]);
        assert_eq!(bracket.split[2].1, Some(dec!(0.1282)));
    }

    #[test]
    fn picks_versions_by_reference_month() {
        let data = data().unwrap();
        let t2026 = data.tables(month("2026-09"));
        assert_eq!(t2026.minimum_wage, dec!(1621));
        assert_eq!(t2026.inss.ceiling, dec!(8475.55));
        assert_eq!(t2026.irrf.simplified_discount, dec!(607.20));
        assert!(t2026.irrf_reduction.is_some());
        assert_eq!(t2026.coverage, CoverageStatus::Within);
        assert!(t2026.fallbacks.is_empty());

        let t2025 = data.tables(month("2025-05"));
        assert_eq!(t2025.minimum_wage, dec!(1518));
        assert_eq!(t2025.inss.ceiling, dec!(8157.41));
        assert_eq!(t2025.irrf.simplified_discount, dec!(607.20));
        assert!(t2025.irrf_reduction.is_none());
        assert!(t2025.fallbacks.is_empty());

        let t2025_april = data.tables(month("2025-04"));
        assert_eq!(t2025_april.irrf.simplified_discount, dec!(564.80));
        assert!(t2025_april.fallbacks.is_empty());
    }

    #[test]
    fn falls_back_to_the_nearest_version() {
        let data = data().unwrap();
        let later = data.tables(month("2027-03"));
        assert!(matches!(later.coverage, CoverageStatus::After(_, note) if note.contains("2027")));
        assert_eq!(later.minimum_wage, dec!(1621));
        let labels: Vec<&str> = later.fallbacks.iter().map(|f| f.label).collect();
        assert_eq!(
            labels,
            [
                "Salário mínimo",
                "INSS do pró-labore",
                "DAS-MEI",
                "Simples Nacional",
                "Lucro Presumido"
            ]
        );

        let earlier = data.tables(month("2023-06"));
        assert!(matches!(earlier.coverage, CoverageStatus::Before(..)));
        assert_eq!(earlier.minimum_wage, dec!(1518));
        assert_eq!(earlier.irrf.simplified_discount, dec!(564.80));
        assert!(earlier.irrf_reduction.is_none());
    }

    #[test]
    fn simples_effective_rate_is_continuous_up_to_bracket_5() {
        // (RBT12 × nominal − deduction) / RBT12 must be the same on both sides of each limit.
        let data = data().unwrap();
        for annex in [&data.simples[0].annex_iii, &data.simples[0].annex_v] {
            for pair in annex.brackets[..5].windows(2) {
                let rbt12 = pair[0].rbt12_up_to;
                let low = (rbt12 * pair[0].nominal_rate - pair[0].deduction) / rbt12;
                let high = (rbt12 * pair[1].nominal_rate - pair[1].deduction) / rbt12;
                assert_eq!(low, high, "bracket {}", pair[0].bracket);
            }
        }
    }

    #[test]
    fn irrf_tables_are_continuous() {
        // At each limit the tax of both brackets differs by less than one cent.
        let data = data().unwrap();
        for irrf in &data.irrf {
            for pair in irrf.brackets.windows(2) {
                let base = pair[0].up_to.unwrap();
                let low = base * pair[0].rate - pair[0].deduction;
                let high = base * pair[1].rate - pair[1].deduction;
                assert!((low - high).abs() < dec!(0.01), "{base}: {low} x {high}");
            }
        }
        // The reduction of Lei 15.270/2025 meets its maximum at 5.000 and zero at 7.350.
        let reduction = &data.irrf_reduction[0];
        let at =
            |gross: Decimal| reduction.phase_out_constant - reduction.phase_out_coefficient * gross;
        assert!((at(reduction.full_reduction_up_to) - reduction.max_reduction).abs() < dec!(0.01));
        assert!(at(reduction.phase_out_up_to).abs() < dec!(0.01));
    }

    #[test]
    fn rejects_broken_tables() {
        let broken = BR_JSON.replacen("\"schema_version\": 1", "\"schema_version\": 9", 1);
        assert!(parse(&broken).unwrap_err().contains("schema_version"));
        let broken = BR_JSON.replacen(
            "\"cpp\": 0.434, \"iss\": 0.335",
            "\"cpp\": 0.434, \"iss\": 0.3",
            1,
        );
        assert!(parse(&broken).unwrap_err().contains("não soma 100%"));
        let broken = BR_JSON.replacen(
            "\"monthly\": 1621.00",
            "\"monthly\": 1621.00, \"typo\": 1",
            1,
        );
        assert!(parse(&broken).is_err());
        let broken = BR_JSON.replacen(
            "\"valid_to\": \"2025-04-30\"",
            "\"valid_to\": \"2025-05-31\"",
            1,
        );
        assert!(parse(&broken).unwrap_err().contains("sobreposição"));
    }
}
