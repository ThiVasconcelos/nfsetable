//! Tax estimates through the public API. Every expectation was computed by hand; the arithmetic
//! is in the comments (amounts in reais, rounded half away from zero to the cent at each tax
//! step).

use nfsetable_core::tax::tax_catalog;
use nfsetable_core::{
    tax_report, CoreError, MeiStatus, MonthAmount, RegimeCost, SimplesActivity, TaxInput,
    TaxRegime, TaxReport, TaxShare,
};
use serde_json::json;
use std::collections::HashSet;

/// `count` consecutive months from `first` (`yyyy-mm`), each with `cents`.
fn months(first: &str, count: u32, cents: i64) -> Vec<MonthAmount> {
    let year: i32 = first[..4].parse().unwrap();
    let month: i32 = first[5..].parse().unwrap();
    (0..count as i32)
        .map(|i| {
            let index = year * 12 + month - 1 + i;
            MonthAmount {
                month: format!("{:04}-{:02}", index / 12, index % 12 + 1),
                cents,
            }
        })
        .collect()
}

fn input(reference: &str, revenue_by_month: Vec<MonthAmount>) -> TaxInput {
    TaxInput {
        reference_month: reference.to_string(),
        revenue_by_month,
        regime: TaxRegime::Simples,
        activity: SimplesActivity::FatorR,
        opening_month: None,
        pro_labore_cents: None,
        revenue_history: Vec::new(),
        payroll_cents: 0,
        dependents: 0,
        monthly_costs_cents: 0,
        costs_by_month: Vec::new(),
        fixed_costs_by_month: Vec::new(),
        leftover_revenue_cents: None,
        reserve_rate: 0.0,
        iss_rate: 0.05,
        desired_net_cents: None,
    }
}

fn regime<'a>(report: &'a TaxReport, key: &str) -> &'a RegimeCost {
    report
        .comparison
        .iter()
        .find(|cost| cost.regime == key)
        .unwrap_or_else(|| panic!("regime {key} missing"))
}

fn share(shares: &[TaxShare], tax: &str) -> i64 {
    shares
        .iter()
        .find(|share| share.tax == tax)
        .unwrap_or_else(|| panic!("share {tax} missing in {shares:?}"))
        .cents
}

fn shares_sum(shares: &[TaxShare]) -> i64 {
    shares.iter().map(|share| share.cents).sum()
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
}

fn has_warning(report: &TaxReport, text: &str) -> bool {
    report.warnings.iter().any(|warning| warning.contains(text))
}

#[test]
fn three_months_of_5000_in_2026() {
    let report = tax_report(&input("2026-03", months("2026-01", 3, 500_000))).unwrap();

    // Revenue: 3 × 5.000 = 15.000; average 5.000; RBT12 = 5.000 × 12 = 60.000 (annualized).
    let revenue = &report.revenue;
    assert_eq!(report.reference_month, "2026-03");
    assert_eq!(revenue.months.len(), 3);
    assert_eq!(revenue.months[0].month, "2026-01");
    assert_eq!(revenue.months_count, 3);
    assert_eq!(revenue.total_cents, 1_500_000);
    assert_eq!(revenue.average_monthly_cents, 500_000);
    assert_eq!(revenue.month_cents, 500_000);
    assert_eq!(revenue.year_to_date_cents, 1_500_000);
    assert_eq!(revenue.rbt12_cents, 6_000_000);
    assert!(revenue.rbt12_annualized);

    // Pró-labore: 28% × 5.000 = 1.400 < minimum wage → 1.621,00. INSS 11% × 1.621 = 178,31.
    // IRRF: deductions max(178,31; 607,20) = 607,20; base 1.013,80 ≤ 2.428,80 → 0.
    // Net 1.621,00 − 178,31 = 1.442,69.
    let pl = &report.pro_labore;
    assert_eq!(pl.gross_cents, 162_100);
    assert!(pl.automatic);
    assert_eq!((pl.basis_average_cents, pl.basis_months), (500_000, 3));
    assert_eq!(pl.inss_cents, 17_831);
    assert_eq!((pl.irrf_cents, pl.irrf_reduction_cents), (0, 0));
    assert_eq!(pl.net_cents, 144_269);

    // Simples: FS12 = 1.621 × 12 = 19.452; Fator R = 19.452 / 60.000 = 0,3242 → 0,32 → Anexo III,
    // bracket 1, 6% → DAS 5.000 × 6% = 300,00.
    let simples = &report.simples;
    assert_eq!(simples.fs12_cents, 1_945_200);
    assert_close(simples.fator_r, 0.32);
    assert_eq!((simples.annex.as_str(), simples.bracket), ("III", 1));
    assert_close(simples.nominal_rate, 0.06);
    assert_close(simples.effective_rate, 0.06);
    assert_eq!(simples.deduction_cents, 0);
    assert_eq!(simples.das_cents, 30_000);
    // Split of 300,00: IRPJ 4% = 12,00; CSLL 3,5% = 10,50; Cofins 12,82% = 38,46;
    // PIS 2,78% = 8,34; CPP 43,4% = 130,20; ISS 33,5% = 100,50.
    let split: Vec<(&str, i64)> = simples
        .split
        .iter()
        .map(|s| (s.tax.as_str(), s.cents))
        .collect();
    assert_eq!(
        split,
        [
            ("irpj", 1_200),
            ("csll", 1_050),
            ("cofins", 3_846),
            ("pis", 834),
            ("cpp", 13_020),
            ("iss", 10_050)
        ]
    );
    assert_eq!(simples.pro_labore_for_annex_iii_cents, 162_100);
    // ME limit: 360.000 − 15.000 = 345.000; the Simples ceiling (EPP) is 4.800.000.
    assert_eq!(simples.me_limit_remaining_cents, 34_500_000);
    assert_eq!(simples.me_limit_cents, 36_000_000);
    assert_eq!(simples.epp_limit_cents, 480_000_000);
    // The year's projection is the revenue's, whatever the regime (the MEI card shows the same).
    assert_eq!(report.revenue.year_projection_cents, 6_000_000);
    assert_eq!(
        report.revenue.year_projection_cents,
        report.mei.year_projection_cents
    );

    // MEI: YTD 15.000 of 81.000; projection 15.000 + 5.000 × 9 = 60.000; DAS 81,05 + 5,00.
    let mei = &report.mei;
    assert_eq!(mei.limit_cents, 8_100_000);
    assert_eq!(mei.year_to_date_cents, 1_500_000);
    assert_close(mei.used_ratio, 15_000.0 / 81_000.0);
    assert_eq!(mei.status, MeiStatus::Within);
    assert_eq!(mei.year_projection_cents, 6_000_000);
    assert_close(mei.projection_ratio, 60_000.0 / 81_000.0);
    assert_eq!(mei.projected_status, MeiStatus::Within);
    assert_eq!(mei.das_cents, 8_605);
    assert!(!mei.it_services_allowed);

    // Comparison, in order.
    let keys: Vec<&str> = report
        .comparison
        .iter()
        .map(|c| c.regime.as_str())
        .collect();
    assert_eq!(keys, ["mei", "simplesIii", "simplesV", "presumido"]);
    let labels: Vec<&str> = report.comparison.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "MEI",
            "Simples Nacional (ME) – Anexo III",
            "Simples Nacional (ME) – Anexo V",
            "Lucro Presumido"
        ]
    );
    // MEI: 86,05 (INSS 81,05 + ISS 5,00); not available for IT; net 5.000 − 86,05 = 4.913,95.
    let m = regime(&report, "mei");
    assert!(!m.available);
    assert!(m.note.as_deref().unwrap().contains("art. 18-A, § 4º, I"));
    assert_eq!(m.company_taxes_cents, 8_605);
    assert_eq!(
        (
            share(&m.company_taxes, "inss"),
            share(&m.company_taxes, "iss")
        ),
        (8_105, 500)
    );
    assert_eq!((m.pro_labore_cents, m.owner_taxes_cents), (0, 0));
    assert_eq!(m.owner_net_cents, 491_395);
    // Anexo III: DAS 300,00 + INSS 178,31 = 478,31 (9,5662%); net 4.521,69.
    let iii = regime(&report, "simplesIii");
    assert!(iii.available);
    assert_eq!(iii.pro_labore_cents, 162_100);
    assert_eq!(iii.company_taxes_cents, 30_000);
    assert_eq!(iii.owner_taxes_cents, 17_831);
    assert_eq!(iii.total_taxes_cents, 47_831);
    assert_close(iii.total_rate, 0.095662);
    assert_eq!(iii.owner_net_cents, 452_169);
    // Anexo V with one minimum wage: 5.000 × 15,5% = 775,00 + 178,31 = 953,31. The minimum wage
    // already reaches 28% of the revenue, so the note says Anexo V is only a reference.
    let v = regime(&report, "simplesV");
    assert!(v.available);
    assert_eq!(v.company_taxes_cents, 77_500);
    assert_eq!(v.total_taxes_cents, 95_331);
    assert!(v.note.as_deref().unwrap().contains("só como referência"));
    // Presumido: base 32% × 5.000 = 1.600; IRPJ 240,00; CSLL 144,00; PIS 32,50; Cofins 150,00;
    // ISS 250,00; CPP 20% × 1.621 = 324,20 → 1.140,70; + INSS 178,31 = 1.319,01.
    let p = regime(&report, "presumido");
    assert!(p.available);
    assert_eq!(p.company_taxes_cents, 114_070);
    assert_eq!(p.total_taxes_cents, 131_901);

    // Leftover (Simples): 5.000 − 300,00 − 178,31 = 4.521,69; tax free: 32% × 5.000 − IRPJ of
    // the DAS (12,00) = 1.588,00.
    let leftover = &report.leftover;
    assert_eq!(leftover.revenue_cents, 500_000);
    assert_eq!(leftover.company_taxes_cents, 30_000);
    assert_eq!(
        (
            leftover.pro_labore_inss_cents,
            leftover.pro_labore_irrf_cents
        ),
        (17_831, 0)
    );
    assert_eq!(leftover.leftover_cents, 452_169);
    assert_eq!(leftover.tax_free_distribution_limit_cents, 158_800);
    assert!(report.pricing.is_none());

    assert!(has_warning(
        &report,
        "média de 3 meses (R$ 5.000,00 por mês)"
    ));
    assert!(has_warning(
        &report,
        "RBT12 foi estimado como a média × 12 (R$ 60.000,00)"
    ));
    assert!(has_warning(&report, "Fator R supõe pró-labore constante"));
    assert_eq!(report.warnings.len(), 3, "{:?}", report.warnings);
    assert!(report
        .sources
        .iter()
        .any(|s| s.contains("Decreto 12.797/2025")));
    assert!(report.sources.iter().any(|s| s.contains("Lei 15.270/2025")));
    assert!(report
        .sources
        .iter()
        .any(|s| s.starts_with("Lucro isento (Simples Nacional)")));
}

#[test]
fn twelve_months_of_10000() {
    let report = tax_report(&input("2026-09", months("2025-10", 12, 1_000_000))).unwrap();
    // RBT12 = 12 × 10.000 = 120.000, not annualized.
    assert_eq!(report.revenue.rbt12_cents, 12_000_000);
    assert!(!report.revenue.rbt12_annualized);
    // Automatic pró-labore 28% × 10.000 = 2.800,00; FS12 = 33.600; Fator R = 0,28 → Anexo III.
    assert_eq!(report.pro_labore.gross_cents, 280_000);
    assert_eq!(
        (
            report.pro_labore.basis_average_cents,
            report.pro_labore.basis_months
        ),
        (1_000_000, 12)
    );
    assert_close(report.simples.fator_r, 0.28);
    assert_eq!(report.simples.annex, "III");
    // Bracket 1, 6% × 10.000 = 600,00.
    assert_eq!(report.simples.das_cents, 60_000);
    // INSS 11% × 2.800 = 308,00; IRRF: base 2.800 − 607,20 = 2.192,80 → 0.
    assert_eq!(report.pro_labore.inss_cents, 30_800);
    assert_eq!(report.pro_labore.irrf_cents, 0);
    // Total Anexo III: 600 + 308 = 908,00.
    assert_eq!(regime(&report, "simplesIii").total_taxes_cents, 90_800);
    // MEI: January to September 2026 = 90.000 (≤ 97.200: up to the tolerance); projection
    // 90.000 + 10.000 × 3 = 120.000 (> 97.200).
    assert_eq!(report.mei.year_to_date_cents, 9_000_000);
    assert_eq!(report.mei.status, MeiStatus::UpToTolerance);
    assert_eq!(report.mei.year_projection_cents, 12_000_000);
    assert_eq!(report.mei.projected_status, MeiStatus::AboveTolerance);
    assert!(!has_warning(&report, "RBT12 foi estimado"));
}

#[test]
fn twelve_months_of_20000() {
    let report = tax_report(&input("2026-09", months("2025-10", 12, 2_000_000))).unwrap();
    // RBT12 240.000 → bracket 2: (240.000 × 11,2% − 9.360) / 240.000 = 17.520 / 240.000 = 7,3%.
    let simples = &report.simples;
    assert_eq!((simples.annex.as_str(), simples.bracket), ("III", 2));
    assert_close(simples.nominal_rate, 0.112);
    assert_eq!(simples.deduction_cents, 936_000);
    assert_close(simples.effective_rate, 0.073);
    // DAS 20.000 × 7,3% = 1.460,00: IRPJ 58,40; CSLL 51,10; Cofins 205,13; PIS 44,53;
    // CPP 633,64; ISS 467,20.
    assert_eq!(simples.das_cents, 146_000);
    assert_eq!(shares_sum(&simples.split), 146_000);
    assert_eq!(share(&simples.split, "cofins"), 20_513);
    assert_eq!(share(&simples.split, "pis"), 4_453);
    assert_eq!(share(&simples.split, "cpp"), 63_364);
    assert_eq!(share(&simples.split, "iss"), 46_720);
    // Pró-labore 28% × 20.000 = 5.600,00; INSS 616,00. IRRF: deductions max(616,00; 607,20) =
    // 616,00; base 4.984,00; 4.984 × 27,5% − 908,73 = 1.370,60 − 908,73 = 461,87. Reduction:
    // 978,62 − 0,133145 × 5.600 = 978,62 − 745,612 = 233,008 → 233,01. IRRF 461,87 − 233,01 =
    // 228,86. Net 5.600 − 616 − 228,86 = 4.755,14.
    let pl = &report.pro_labore;
    assert_eq!(pl.gross_cents, 560_000);
    assert_eq!(pl.inss_cents, 61_600);
    assert_eq!(pl.irrf_reduction_cents, 23_301);
    assert_eq!(pl.irrf_cents, 22_886);
    assert_eq!(pl.net_cents, 475_514);
    // Comparison at R$ 20.000/mês: Anexo III 1.460 + 616 + 228,86 =
    // 2.304,86; Anexo V 3.225 + 178,31 = 3.403,31; Presumido 3.590,20 + 178,31 = 3.768,51.
    assert_eq!(regime(&report, "simplesIii").total_taxes_cents, 230_486);
    assert_eq!(regime(&report, "simplesV").total_taxes_cents, 340_331);
    assert_eq!(regime(&report, "presumido").total_taxes_cents, 376_851);
    // Leftover: 20.000 − 1.460 − 616 − 228,86 = 17.695,14; tax free 6.400 − 58,40 = 6.341,60.
    assert_eq!(report.leftover.leftover_cents, 1_769_514);
    assert_eq!(report.leftover.tax_free_distribution_limit_cents, 634_160);
}

#[test]
fn user_pro_labore_of_one_minimum_wage_falls_to_annex_v() {
    let mut data = input("2026-09", months("2025-10", 12, 2_000_000));
    data.pro_labore_cents = Some(162_100);
    let report = tax_report(&data).unwrap();
    // FS12 = 1.621 × 12 = 19.452; Fator R = 19.452 / 240.000 = 0,08105 → 0,08 → Anexo V,
    // bracket 2: (240.000 × 18% − 4.500) / 240.000 = 38.700 / 240.000 = 16,125%.
    let simples = &report.simples;
    assert_close(simples.fator_r, 0.08);
    assert_eq!((simples.annex.as_str(), simples.bracket), ("V", 2));
    assert_close(simples.effective_rate, 0.16125);
    // DAS 20.000 × 16,125% = 3.225,00: IRPJ 23% = 741,75; CSLL 15% = 483,75;
    // Cofins 14,1% = 454,725 → 454,73; PIS 3,05% = 98,3625 → 98,36; CPP 27,85% = 898,1625 →
    // 898,16; ISS 17% = 548,25; sum 3.225,00.
    assert_eq!(simples.das_cents, 322_500);
    let split: Vec<i64> = simples.split.iter().map(|s| s.cents).collect();
    assert_eq!(split, [74_175, 48_375, 45_473, 9_836, 89_816, 54_825]);
    // The pró-labore for Anexo III is still the automatic one: 28% × 20.000 = 5.600.
    assert_eq!(simples.pro_labore_for_annex_iii_cents, 560_000);
    assert!(!report.pro_labore.automatic);
    assert_eq!(report.pro_labore.gross_cents, 162_100);
    // Anexo III scenario uses the automatic 5.600 (the typed value would miss the Fator R);
    // Anexo V uses the typed 1.621.
    let iii = regime(&report, "simplesIii");
    assert_eq!(iii.pro_labore_cents, 560_000);
    assert!(iii
        .note
        .as_deref()
        .unwrap()
        .contains("O informado (R$ 1.621,00)"));
    let v = regime(&report, "simplesV");
    assert_eq!(v.pro_labore_cents, 162_100);
    assert_eq!(v.total_taxes_cents, 340_331);
    // Leftover in Anexo V: 20.000 − 3.225 − 178,31 = 16.596,69; tax free 6.400 − 741,75 = 5.658,25.
    assert_eq!(report.leftover.company_taxes_cents, 322_500);
    assert_eq!(report.leftover.leftover_cents, 1_659_669);
    assert_eq!(report.leftover.tax_free_distribution_limit_cents, 565_825);
    let v_note = v.note.as_deref().unwrap();
    assert!(v_note.contains("fica abaixo de 28% e a receita vai para o Anexo V"));
}

#[test]
fn typed_pro_labore_below_the_minimum_wage() {
    let mut data = input("2026-03", months("2026-01", 3, 300_000));
    data.pro_labore_cents = Some(100_000);
    let report = tax_report(&data).unwrap();
    // Typed 1.000,00: INSS 110,00, no IRRF. FS12 12.000 / RBT12 36.000 = 0,33 → Anexo III even
    // below the minimum wage; DAS 6% × 3.000 = 180,00.
    assert_eq!(report.pro_labore.gross_cents, 100_000);
    assert_eq!(report.pro_labore.inss_cents, 11_000);
    assert_close(report.simples.fator_r, 0.33);
    assert_eq!(report.simples.annex, "III");
    assert_eq!(report.simples.das_cents, 18_000);
    assert!(has_warning(
        &report,
        "Pró-labore informado (R$ 1.000,00) abaixo do salário mínimo (R$ 1.621,00)"
    ));
    // The automatic amount is the minimum wage (28% × 3.000 = 840 is below it), so the Anexo III
    // scenario uses 1.621,00 and says why the typed value was not used.
    let iii = regime(&report, "simplesIii");
    assert_eq!(iii.pro_labore_cents, 162_100);
    assert!(iii
        .note
        .as_deref()
        .unwrap()
        .contains("O informado (R$ 1.000,00) fica abaixo do salário mínimo."));
    // Anexo V with the typed value is only a reference: it already reaches the Fator R.
    let v = regime(&report, "simplesV");
    assert_eq!(v.pro_labore_cents, 100_000);
    assert!(v.note.as_deref().unwrap().contains("só como referência"));
    // Leftover: 3.000 − 180,00 − 110,00 = 2.710,00; tax free 960,00 − IRPJ 7,20 = 952,80.
    assert_eq!(report.leftover.leftover_cents, 271_000);
    assert_eq!(report.leftover.tax_free_distribution_limit_cents, 95_280);
}

#[test]
fn mei_opened_in_july() {
    let mut data = input("2026-09", months("2026-07", 3, 1_000_000));
    data.regime = TaxRegime::Mei;
    data.opening_month = Some("2026-07".to_string());
    let report = tax_report(&data).unwrap();
    // Limit 6.750 × 6 months (July to December) = 40.500. YTD 30.000 ≤ 40.500.
    let mei = &report.mei;
    assert_eq!(mei.limit_cents, 4_050_000);
    assert_eq!(mei.year_to_date_cents, 3_000_000);
    assert_eq!(mei.status, MeiStatus::Within);
    assert_close(mei.used_ratio, 30_000.0 / 40_500.0);
    // Projection 30.000 + 10.000 × 3 = 60.000 > 40.500 × 1,2 = 48.600.
    assert_eq!(mei.year_projection_cents, 6_000_000);
    assert_eq!(mei.projected_status, MeiStatus::AboveTolerance);
    assert_close(mei.projection_ratio, 60_000.0 / 40_500.0);
    assert_eq!(mei.das_cents, 8_605);
    // Leftover in the MEI: 10.000 − 86,05 = 9.913,95; tax free 32% × 10.000 = 3.200,00.
    assert_eq!(report.leftover.company_taxes_cents, 8_605);
    assert_eq!(report.leftover.pro_labore_inss_cents, 0);
    assert_eq!(report.leftover.leftover_cents, 991_395);
    assert_eq!(report.leftover.tax_free_distribution_limit_cents, 320_000);
    assert!(has_warning(
        &report,
        "mais de 20% acima do limite do MEI (R$ 40.500,00)"
    ));
    assert!(has_warning(&report, "serviços de TI não podem ser MEI"));
    assert!(has_warning(&report, "início de atividade simplificadas"));
    // The MEI tax-free rule is not verified: estimate warning and marked source.
    assert!(has_warning(&report, "lucro isento do MEI é estimativa"));
    assert!(report
        .sources
        .iter()
        .any(|s| s.starts_with("Lucro isento (MEI)") && s.ends_with("⚠ não verificado")));
    let m = regime(&report, "mei");
    assert!(!m.available);
    assert!(m.note.as_deref().unwrap().contains("R$ 60.000,00"));

    // An occupation allowed to the MEI (Anexo III activity) within the limit is available.
    data.activity = SimplesActivity::AnnexIii;
    data.revenue_by_month = months("2026-07", 3, 500_000);
    let report = tax_report(&data).unwrap();
    // Projection 15.000 + 5.000 × 3 = 30.000 ≤ 40.500.
    assert_eq!(report.mei.projected_status, MeiStatus::Within);
    assert!(regime(&report, "mei").available);
    assert!(!has_warning(&report, "serviços de TI não podem ser MEI"));
    // Anexo V does not apply to an activity that is always taxed in Anexo III.
    let v = regime(&report, "simplesV");
    assert!(!v.available);
    assert_eq!(
        v.note.as_deref(),
        Some("A atividade é sempre do Anexo III.")
    );
}

#[test]
fn pro_labore_of_10000() {
    let mut data = input("2026-09", months("2025-10", 12, 5_000_000));
    data.pro_labore_cents = Some(1_000_000);
    let report = tax_report(&data).unwrap();
    // INSS: 11% × ceiling 8.475,55 = 932,3105 → 932,31. IRRF: base 10.000 − 932,31 = 9.067,69;
    // 9.067,69 × 27,5% − 908,73 = 2.493,61475 − 908,73 = 1.584,88475 → 1.584,88; no reduction
    // above 7.350. Net 10.000 − 932,31 − 1.584,88 = 7.482,81.
    let pl = &report.pro_labore;
    assert_eq!(pl.inss_cents, 93_231);
    assert_eq!(pl.irrf_cents, 158_488);
    assert_eq!(pl.irrf_reduction_cents, 0);
    assert_eq!(pl.net_cents, 748_281);
}

#[test]
fn lucro_presumido_for_20000() {
    let mut data = input("2026-09", months("2025-10", 12, 2_000_000));
    data.regime = TaxRegime::Presumido;
    let report = tax_report(&data).unwrap();
    // Pró-labore of one minimum wage. IRPJ 15% × 32% × 20.000 = 960,00 (base 6.400 ≤ 20.000, no
    // surcharge); CSLL 9% × 6.400 = 576,00; PIS 0,65% = 130,00; Cofins 3% = 600,00;
    // ISS 5% = 1.000,00; CPP 20% × 1.621 = 324,20. Company 3.590,20; owner INSS 178,31.
    let p = regime(&report, "presumido");
    assert_eq!(p.pro_labore_cents, 162_100);
    // The pró-labore of the current regime is that minimum wage too (no Fator R here), while
    // the Simples estimates keep 28% × 20.000 = 5.600,00.
    assert_eq!(report.pro_labore.gross_cents, 162_100);
    assert_eq!(report.pro_labore.inss_cents, 17_831);
    assert!(report.pro_labore.basis_month.is_none());
    assert_eq!(report.simples.pro_labore_for_annex_iii_cents, 560_000);
    let company: Vec<(&str, i64)> = p
        .company_taxes
        .iter()
        .map(|s| (s.tax.as_str(), s.cents))
        .collect();
    assert_eq!(
        company,
        [
            ("irpj", 96_000),
            ("csll", 57_600),
            ("pis", 13_000),
            ("cofins", 60_000),
            ("iss", 100_000),
            ("cpp", 32_420)
        ]
    );
    assert_eq!(p.company_taxes_cents, 359_020);
    assert_eq!(p.owner_taxes_cents, 17_831);
    assert_eq!(p.total_taxes_cents, 376_851);
    // 3.768,51 / 20.000 = 18,84255%.
    assert_close(p.total_rate, 0.1884255);
    assert_eq!(p.owner_net_cents, 1_623_149);
    assert!(p.note.as_deref().unwrap().contains("média mensal"));
    // Leftover: 20.000 − 3.590,20 − 178,31 = 16.231,49; tax free (estimate): 6.400 − 960 − 576 −
    // 130 − 600 = 4.134,00.
    let leftover = &report.leftover;
    assert_eq!(leftover.company_taxes_cents, 359_020);
    assert_eq!(leftover.leftover_cents, 1_623_149);
    assert_eq!(leftover.tax_free_distribution_limit_cents, 413_400);
    assert!(has_warning(
        &report,
        "lucro isento do Lucro Presumido é estimativa"
    ));
    assert!(report
        .sources
        .iter()
        .any(|s| s.contains("IN RFB 1.700/2017, art. 238") && s.ends_with("⚠ não verificado")));
}

#[test]
fn iss_cap_in_annex_iii_bracket_5() {
    let mut data = input("2026-09", months("2025-10", 12, 25_000_000));
    data.activity = SimplesActivity::AnnexIii;
    let report = tax_report(&data).unwrap();
    // RBT12 3.000.000 → bracket 5: (3.000.000 × 21% − 125.640) / 3.000.000 = 504.360 / 3.000.000
    // = 16,812%. DAS 250.000 × 16,812% = 42.030,00. ISS 16,812% × 33,5% = 5,632% > 5% → capped:
    // ISS 5% × 250.000 = 12.500,00; the rest (11,812% × 250.000 = 29.530,00) goes IRPJ 6,02% =
    // 1.777,71; CSLL 5,26% = 1.553,28; Cofins 19,28% = 5.693,38; PIS 4,18% = 1.234,35;
    // CPP 65,26% = 19.271,28.
    let simples = &report.simples;
    assert_eq!(simples.bracket, 5);
    assert_close(simples.effective_rate, 0.16812);
    assert_eq!(simples.das_cents, 4_203_000);
    assert_eq!(shares_sum(&simples.split), simples.das_cents);
    assert_eq!(share(&simples.split, "iss"), 1_250_000);
    assert_eq!(share(&simples.split, "irpj"), 177_771);
    assert_eq!(share(&simples.split, "csll"), 155_328);
    assert_eq!(share(&simples.split, "cofins"), 569_338);
    assert_eq!(share(&simples.split, "pis"), 123_435);
    assert_eq!(share(&simples.split, "cpp"), 1_927_128);
    // 9 × 250.000 in 2026 passed the ME limit: EPP warning and note.
    assert_eq!(simples.me_limit_remaining_cents, 0);
    assert!(has_warning(
        &report,
        "passou do limite da ME (R$ 360.000,00)"
    ));
    assert!(regime(&report, "simplesIii")
        .note
        .as_deref()
        .unwrap()
        .contains("a empresa é EPP"));
}

#[test]
fn sixth_bracket_and_above_the_ceiling() {
    let report = tax_report(&input("2026-09", months("2025-10", 12, 35_000_000))).unwrap();
    // RBT12 4.200.000 → bracket 6: (4.200.000 × 33% − 648.000) / 4.200.000 = 17,5714…%;
    // DAS 350.000 / 12 of 738.000 = 61.500,00, without ISS: IRPJ 35% = 21.525,00; CSLL 15% =
    // 9.225,00; Cofins 16,03% = 9.858,45; PIS 3,47% = 2.134,05; CPP 30,5% = 18.757,50.
    let simples = &report.simples;
    assert_eq!((simples.annex.as_str(), simples.bracket), ("III", 6));
    assert_eq!(simples.das_cents, 6_150_000);
    assert!(simples.split.iter().all(|s| s.tax != "iss"));
    assert_eq!(shares_sum(&simples.split), 6_150_000);
    assert!(has_warning(&report, "6ª faixa"));
    // The comparison adds the ISS paid to the municipality: 5% × 350.000 = 17.500,00.
    let iii = regime(&report, "simplesIii");
    assert!(iii.available);
    assert_eq!(share(&iii.company_taxes, "iss"), 1_750_000);
    assert_eq!(iii.company_taxes_cents, 6_150_000 + 1_750_000);

    // RBT12 5.400.000: above the ceiling of the Simples Nacional.
    let report = tax_report(&input("2026-09", months("2025-10", 12, 45_000_000))).unwrap();
    assert_eq!(report.simples.bracket, 6);
    assert!(has_warning(&report, "acima do teto do Simples Nacional"));
    assert!(!regime(&report, "simplesIii").available);
    assert!(!regime(&report, "simplesV").available);
    assert!(regime(&report, "presumido").available);
    // The Simples section is always shown, so the warning does not depend on the regime.
    let mut data = input("2026-09", months("2025-10", 12, 45_000_000));
    data.regime = TaxRegime::Presumido;
    let report = tax_report(&data).unwrap();
    assert!(has_warning(&report, "acima do teto do Simples Nacional"));
    assert!(!has_warning(&report, "limite da ME"));
}

#[test]
fn tables_follow_the_reference_month() {
    // Pró-labore 5.600 (automatic for 20.000/month), INSS 616,00, base 4.984,00.
    let irrf = |reference: &str| {
        tax_report(&input(reference, months("2024-10", 12, 2_000_000)))
            .unwrap()
            .pro_labore
            .irrf_cents
    };
    // Until 2025-04 (Lei 14.848/2024): 1.370,60 − 896,00 = 474,60.
    assert_eq!(irrf("2025-03"), 47_460);
    // From 2025-05 (Lei 15.191/2025), without the reduction: 1.370,60 − 908,73 = 461,87.
    assert_eq!(irrf("2025-06"), 46_187);
    // 2026, with the reduction of Lei 15.270/2025: 461,87 − 233,01 = 228,86.
    assert_eq!(irrf("2026-06"), 22_886);
    // DAS-MEI 2025: 5% × 1.518 = 75,90 + 5,00 = 80,90; minimum wage 1.518 in the scenarios.
    let report = tax_report(&input("2025-06", months("2025-01", 6, 500_000))).unwrap();
    assert_eq!(report.mei.das_cents, 8_090);
    assert_eq!(report.pro_labore.gross_cents, 151_800);
    // INSS 11% × 1.518 = 166,98.
    assert_eq!(report.pro_labore.inss_cents, 16_698);
    assert!(report
        .sources
        .iter()
        .any(|s| s.contains("Decreto 12.342/2024")));
    assert!(!report.sources.iter().any(|s| s.contains("Lei 15.270/2025")));
}

#[test]
fn no_months_gives_a_report_of_zeros() {
    let report = tax_report(&input("2026-09", Vec::new())).unwrap();
    assert_eq!(report.revenue.months_count, 0);
    assert_eq!(report.revenue.average_monthly_cents, 0);
    assert_eq!(report.simples.das_cents, 0);
    assert_eq!(report.pro_labore.gross_cents, 0);
    assert_eq!(report.leftover.leftover_cents, 0);
    assert_eq!(report.comparison.len(), 4);
    assert!(report
        .comparison
        .iter()
        .all(|c| !c.available && c.total_taxes_cents == 0));
    assert!(report.pricing.is_none());
    assert!(has_warning(&report, "Nenhum mês selecionado"));

    // Only malformed months: same report, plus the warning about the month.
    let report = tax_report(&input(
        "2026-09",
        vec![MonthAmount {
            month: "set/2026".to_string(),
            cents: 100_000,
        }],
    ))
    .unwrap();
    assert_eq!(report.revenue.months_count, 0);
    assert!(has_warning(&report, "\"set/2026\""));
    assert!(has_warning(&report, "Nenhum mês selecionado"));
}

#[test]
fn malformed_and_duplicate_months() {
    let mut revenue = months("2026-01", 2, 400_000);
    revenue.push(MonthAmount {
        month: "2026-02".to_string(),
        cents: 250_000,
    });
    revenue.push(MonthAmount {
        month: "2026-13".to_string(),
        cents: 999_999,
    });
    let report = tax_report(&input("2026-02", revenue)).unwrap();
    // 2026-01: 4.000; 2026-02: 4.000 + 2.500 = 6.500; 2026-13 ignored. Average 5.250.
    assert_eq!(report.revenue.months_count, 2);
    assert_eq!(report.revenue.months[1].cents, 650_000);
    assert_eq!(report.revenue.average_monthly_cents, 525_000);
    assert!(has_warning(&report, "\"2026-13\""));
}

#[test]
fn reference_month_outside_the_tables() {
    let report = tax_report(&input("2027-03", months("2027-01", 3, 500_000))).unwrap();
    // Uses the 2026 tables: DAS-MEI 86,05 and one minimum wage of 1.621,00.
    assert_eq!(report.mei.das_cents, 8_605);
    assert_eq!(report.pro_labore.gross_cents, 162_100);
    assert!(has_warning(
        &report,
        "As regras de 2027 em diante ainda não estão incluídas"
    ));
    assert!(has_warning(&report, "IBS e CBS"));

    let report = tax_report(&input("2024-06", months("2024-04", 3, 500_000))).unwrap();
    // Uses the nearest (2025) tables: minimum wage 1.518,00.
    assert_eq!(report.pro_labore.gross_cents, 151_800);
    assert!(has_warning(&report, "anteriores a 2025"));
}

#[test]
fn invalid_input_is_an_error() {
    let bad_month = tax_report(&input("2026-13", months("2026-01", 1, 1)));
    assert!(matches!(bad_month, Err(CoreError::Tax(message)) if message.contains("2026-13")));
    let mut data = input("2026-03", months("2026-01", 1, 1));
    data.iss_rate = 5.0;
    assert!(matches!(tax_report(&data), Err(CoreError::Tax(_))));
    data.iss_rate = f64::NAN;
    assert!(matches!(tax_report(&data), Err(CoreError::Tax(_))));
}

/// The owner net of a constant monthly revenue in the current regime, as the pricing sees it:
/// 12 equal months (RBT12 = 12 × revenue) with the automatic pró-labore.
fn leftover_at(base: &TaxInput, revenue_cents: i64) -> i64 {
    let mut data = base.clone();
    data.revenue_by_month = months("2025-10", 12, revenue_cents);
    data.desired_net_cents = None;
    tax_report(&data).unwrap().leftover.leftover_cents
}

#[test]
fn pricing_finds_the_smallest_revenue() {
    let mut base = input("2026-09", months("2025-10", 12, 1_000_000));
    base.monthly_costs_cents = 200_000;
    base.desired_net_cents = Some(1_000_000);

    // MEI: 10.000 + 2.000 of costs + 86,05 of DAS-MEI = 12.086,05.
    base.regime = TaxRegime::Mei;
    let pricing = tax_report(&base).unwrap().pricing.unwrap();
    assert_eq!(pricing.desired_net_cents, 1_000_000);
    assert_eq!(pricing.required_revenue_cents, 1_208_605);
    // 12 × 12.086,05 = 145.032,60 per year, above the MEI limit.
    assert!(pricing.note.contains("acima do limite do MEI"));

    for (regime, activity) in [
        (TaxRegime::Mei, SimplesActivity::FatorR),
        (TaxRegime::Simples, SimplesActivity::FatorR),
        (TaxRegime::Simples, SimplesActivity::AnnexIii),
        (TaxRegime::Presumido, SimplesActivity::FatorR),
    ] {
        for desired in [0, 350_000, 1_000_000, 2_500_000, 12_345_678] {
            base.regime = regime;
            base.activity = activity;
            base.desired_net_cents = Some(desired);
            let pricing = tax_report(&base).unwrap().pricing.unwrap();
            let required = pricing.required_revenue_cents;
            assert!(
                leftover_at(&base, required) >= desired,
                "{regime:?} {desired}: {required} does not reach it"
            );
            assert!(
                leftover_at(&base, required - 1) < desired,
                "{regime:?} {desired}: {} already reaches it",
                required - 1
            );
        }
    }

    // Simples: the note states the assumptions (Anexo III with the automatic pró-labore).
    base.regime = TaxRegime::Simples;
    base.activity = SimplesActivity::FatorR;
    base.desired_net_cents = Some(1_000_000);
    let note = tax_report(&base).unwrap().pricing.unwrap().note;
    assert!(note.contains("Anexo III"), "{note}");
    assert!(note.contains("pró-labore automático"), "{note}");
}

#[test]
fn report_json_shape() {
    let mut data = input("2026-03", months("2026-01", 3, 500_000));
    data.desired_net_cents = Some(300_000);
    let value = serde_json::to_value(tax_report(&data).unwrap()).unwrap();
    assert_eq!(value["referenceMonth"], "2026-03");
    assert_eq!(value["revenue"]["averageMonthlyCents"], 500_000);
    assert_eq!(value["revenue"]["rbt12Cents"], 6_000_000);
    assert_eq!(value["revenue"]["rbt12Annualized"], true);
    assert_eq!(
        value["revenue"]["months"][0],
        json!({"month": "2026-01", "cents": 500_000})
    );
    assert_eq!(value["mei"]["status"], "within");
    assert_eq!(value["mei"]["projectedStatus"], "within");
    assert_eq!(value["mei"]["itServicesAllowed"], false);
    assert_eq!(value["simples"]["fatorR"], 0.32);
    assert_eq!(value["simples"]["proLaboreForAnnexIiiCents"], 162_100);
    assert_eq!(value["simples"]["meLimitRemainingCents"], 34_500_000);
    assert_eq!(value["simples"]["meLimitCents"], 36_000_000);
    assert_eq!(value["simples"]["eppLimitCents"], 480_000_000);
    assert_eq!(value["revenue"]["yearProjectionCents"], 6_000_000);
    assert_eq!(
        value["simples"]["split"][0],
        json!({"tax": "irpj", "cents": 1_200})
    );
    assert_eq!(value["proLabore"]["irrfReductionCents"], 0);
    assert_eq!(value["proLabore"]["basisMonths"], 3);
    assert_eq!(value["comparison"][1]["regime"], "simplesIii");
    assert!(value["comparison"][1]["companyTaxes"].is_array());
    assert!(value["comparison"][1]["ownerNetCents"].is_i64());
    assert!(value["comparison"][1]["totalRate"].is_f64());
    assert!(value["leftover"]["taxFreeDistributionLimitCents"].is_i64());
    assert!(value["pricing"]["requiredRevenueCents"].is_i64());
    assert!(value["warnings"].is_array() && value["sources"].is_array());

    // The frontend sends camelCase and may omit the optional keys.
    let parsed: TaxInput = serde_json::from_value(json!({
        "referenceMonth": "2026-03",
        "revenueByMonth": [{"month": "2026-01", "cents": 500000}],
        "regime": "presumido",
        "activity": "annexIii",
        "payrollCents": 0,
        "dependents": 1,
        "monthlyCostsCents": 0,
        "issRate": 0.05
    }))
    .unwrap();
    assert_eq!(parsed.regime, TaxRegime::Presumido);
    assert_eq!(parsed.activity, SimplesActivity::AnnexIii);
    assert_eq!(parsed.opening_month, None);
    assert_eq!(parsed.pro_labore_cents, None);
    assert_eq!(parsed.desired_net_cents, None);
    assert!(tax_report(&parsed).is_ok());
}

#[test]
fn cnae_catalog() {
    let catalog = tax_catalog();
    assert_eq!(catalog.default_cnae, "6201-5/01");
    assert!(catalog.cnaes.iter().any(|c| c.code == catalog.default_cnae));
    let codes: HashSet<&str> = catalog.cnaes.iter().map(|c| c.code.as_str()).collect();
    assert_eq!(codes.len(), catalog.cnaes.len(), "codes must be unique");
    assert_eq!(catalog.cnaes.len(), 13);
    // IT services follow the Fator R and are not allowed to the MEI.
    let dev = &catalog.cnaes[0];
    assert_eq!(dev.code, "6201-5/01");
    assert_eq!(dev.activity, SimplesActivity::FatorR);
    assert!(!dev.mei_allowed && dev.mei_occupation.is_none());
    let data = catalog
        .cnaes
        .iter()
        .find(|c| c.code == "6311-9/00")
        .unwrap();
    assert!(data
        .note
        .as_deref()
        .unwrap()
        .contains("confirmar com contador"));
    // Peripheral activities allowed to the MEI are always taxed in Anexo III.
    let training = catalog
        .cnaes
        .iter()
        .find(|c| c.code == "8599-6/03")
        .unwrap();
    assert_eq!(training.activity, SimplesActivity::AnnexIii);
    assert!(training.mei_allowed);
    assert_eq!(
        training.mei_occupation.as_deref(),
        Some("Instrutor de informática")
    );
    let value = serde_json::to_value(&catalog).unwrap();
    assert_eq!(value["defaultCnae"], "6201-5/01");
    assert_eq!(value["cnaes"][0]["activity"], "fatorR");
    assert_eq!(value["cnaes"][0]["meiAllowed"], false);
    assert!(value["cnaes"][0].get("meiOccupation").is_some());
}

#[test]
fn leftover_with_a_fixed_revenue_and_a_reserve() {
    // Three months of 10.000,00: RBT12 120.000 (annualized), automatic pró-labore 2.800,00,
    // Fator R 0,28 -> Anexo III, 1st bracket, 6%.
    let mut tax = input("2026-09", months("2026-07", 3, 1_000_000));
    tax.monthly_costs_cents = 50_000;
    tax.reserve_rate = 0.10;
    tax.leftover_revenue_cents = Some(800_000);
    let report = tax_report(&tax).unwrap();

    // Sobra on 8.000,00 (e.g. without bonuses), same 6% effective rate and same pró-labore:
    // DAS 480,00; INSS 308,00; IRRF 0; costs 500,00; reserve 10% = 800,00.
    // 8.000,00 − 480,00 − 308,00 − 500,00 − 800,00 = 5.912,00.
    let left = &report.leftover;
    assert!(left.custom_revenue);
    assert_eq!(left.revenue_cents, 800_000);
    assert_eq!(left.company_taxes_cents, 48_000);
    assert_eq!(left.pro_labore_inss_cents, 30_800);
    assert_eq!(left.reserve_cents, 80_000);
    assert_eq!(left.leftover_cents, 591_200);
    // Tax free: 32% × 8.000,00 − IRPJ inside the DAS (4% × 480,00 = 19,20) = 2.540,80.
    assert_eq!(left.tax_free_distribution_limit_cents, 254_080);

    // The comparison stays on the average (10.000,00) but also keeps the reserve:
    // Anexo III total taxes 600,00 + 308,00; reserve 1.000,00; sobra 10.000 − 908 − 500 − 1.000.
    let iii = regime(&report, "simplesIii");
    assert_eq!(iii.reserve_cents, 100_000);
    assert_eq!(iii.owner_net_cents, 759_200);
}

#[test]
fn reserve_raises_the_price() {
    let mut tax = input("2026-09", months("2026-07", 3, 1_000_000));
    tax.monthly_costs_cents = 50_000;
    tax.desired_net_cents = Some(700_000);
    let without = tax_report(&tax)
        .unwrap()
        .pricing
        .unwrap()
        .required_revenue_cents;
    tax.reserve_rate = 0.10;
    let report = tax_report(&tax).unwrap();
    let with = report.pricing.as_ref().unwrap().required_revenue_cents;
    // net(R) = R − 6% R − 11% × 28% R − 500,00 − 10% R = 0,8092 R − 500,00 >= 7.000,00
    // -> R ≈ 9.268,66 (cents rounding may move it by a cent or two).
    assert!((926_800..=926_900).contains(&with), "{with}");
    assert!(with > without);
    assert!(report.pricing.unwrap().note.contains("reserva"));
}

#[test]
fn invalid_reserve_is_rejected() {
    let mut tax = input("2026-09", months("2026-07", 3, 1_000_000));
    tax.reserve_rate = 0.95;
    assert!(tax_report(&tax).is_err());
    tax.reserve_rate = -0.1;
    assert!(tax_report(&tax).is_err());
}

#[test]
fn variable_costs_and_the_monthly_cash_flow() {
    let revenue = vec![
        MonthAmount {
            month: "2026-07".into(),
            cents: 1_000_000,
        },
        MonthAmount {
            month: "2026-08".into(),
            cents: 1_200_000,
        },
        MonthAmount {
            month: "2026-09".into(),
            cents: 800_000,
        },
    ];
    let mut tax = input("2026-09", revenue);
    tax.monthly_costs_cents = 50_000;
    tax.costs_by_month = vec![
        MonthAmount {
            month: "2026-08".into(),
            cents: 120_000,
        },
        MonthAmount {
            month: "2026-09".into(),
            cents: 15_000,
        },
        // Outside the months considered: not in the typical month, not in the cash flow.
        MonthAmount {
            month: "2026-10".into(),
            cents: 99_900,
        },
        MonthAmount {
            month: "agosto".into(),
            cents: 1,
        },
    ];
    let report = tax_report(&tax).unwrap();
    assert!(has_warning(&report, "\"agosto\""));

    // Average 10.000,00, RBT12 120.000 -> Anexo III 6%. Each month's RBT12 (the months known
    // before it): July has none (10.000 × 12 = 120.000), August has July (120.000), September has
    // July and August (22.000 × 12 / 2 = 132.000). The automatic pró-labore covers the most
    // demanding month: 28% × 132.000 / 12 = 3.080,00 (the average alone would give 2.800,00 and
    // leave September at a Fator R of 0,25, Anexo V). INSS 338,80; IRRF zeroed by the reduction.
    let pl = &report.pro_labore;
    assert_eq!(pl.gross_cents, 308_000);
    assert_eq!(pl.basis_month.as_deref(), Some("2026-09"));
    assert_eq!((pl.basis_average_cents, pl.basis_months), (1_100_000, 2));
    assert_eq!((pl.inss_cents, pl.irrf_cents), (33_880, 0));

    // Variable costs of the typical month: the reference month's (September, 150,00), not an
    // average; August's 1.200,00 counts only in August.
    // Sobra: 10.000 − 600 − 338,80 − 500 (fixed) − 150 (variable) = 8.411,20.
    let left = &report.leftover;
    assert_eq!(left.costs_cents, 50_000);
    assert_eq!(left.variable_costs_cents, 15_000);
    assert_eq!(left.leftover_cents, 841_120);
    assert_eq!(regime(&report, "simplesIii").owner_net_cents, 841_120);

    // Month by month: each month's revenue, costs and RBT12 (bracket 1, 6%, in all three).
    let flow: Vec<(&str, i64, i64, i64, i64, i64)> = report
        .cash_flow
        .iter()
        .map(|m| {
            (
                m.month.as_str(),
                m.revenue_cents,
                m.company_taxes_cents,
                m.variable_costs_cents,
                m.leftover_cents,
                m.profit_cents,
            )
        })
        .collect();
    assert_eq!(
        flow,
        [
            // Sobra 10.000 − 600 − 338,80 − 500 = 8.561,20; lucro 10.000 − 600 − 500 − 3.080.
            ("2026-07", 1_000_000, 60_000, 0, 856_120, 582_000),
            // 12.000 − 720 − 338,80 − 500 − 1.200 = 9.241,20; 12.000 − 720 − 500 − 1.200 − 3.080.
            ("2026-08", 1_200_000, 72_000, 120_000, 924_120, 650_000),
            // 8.000 − 480 − 338,80 − 500 − 150 = 6.531,20; 8.000 − 480 − 500 − 150 − 3.080.
            ("2026-09", 800_000, 48_000, 15_000, 653_120, 379_000),
        ]
    );
    assert!(report
        .cash_flow
        .iter()
        .all(|m| m.pro_labore_cents == 308_000
            && m.owner_taxes_cents == 33_880
            && m.inss_cents == 33_880
            && m.irrf_cents == 0
            && m.annex.as_deref() == Some("III")));
    let rbt12: Vec<Option<i64>> = report.cash_flow.iter().map(|m| m.rbt12_cents).collect();
    assert_eq!(
        rbt12,
        [Some(12_000_000), Some(12_000_000), Some(13_200_000)]
    );
    // Fator R 36.960 / 120.000 = 0,308 → 0,30; in September 36.960 / 132.000 = 0,28.
    let fator: Vec<Option<f64>> = report.cash_flow.iter().map(|m| m.fator_r).collect();
    assert_eq!(fator, [Some(0.3), Some(0.3), Some(0.28)]);
    let minimum: Vec<Option<i64>> = report
        .cash_flow
        .iter()
        .map(|m| m.pro_labore_for_annex_iii_cents)
        .collect();
    assert_eq!(minimum, [Some(280_000), Some(280_000), Some(308_000)]);
}

#[test]
fn a_fixed_pro_labore_too_low_puts_a_month_in_annex_v() {
    // Same months, with the pró-labore fixed at 2.800,00 (28% of the average only).
    let mut tax = input(
        "2026-09",
        vec![
            MonthAmount {
                month: "2026-07".into(),
                cents: 1_000_000,
            },
            MonthAmount {
                month: "2026-08".into(),
                cents: 1_200_000,
            },
            MonthAmount {
                month: "2026-09".into(),
                cents: 800_000,
            },
        ],
    );
    tax.pro_labore_cents = Some(280_000);
    let report = tax_report(&tax).unwrap();
    assert!(!report.pro_labore.automatic);

    // September: Fator R 33.600 / 132.000 = 0,2545 → 0,25 → Anexo V, 15,5% of 8.000 = 1.240,00;
    // it needs 3.080,00 of pró-labore.
    let sep = &report.cash_flow[2];
    assert_eq!(sep.annex.as_deref(), Some("V"));
    assert_eq!(sep.fator_r, Some(0.25));
    assert_eq!(sep.company_taxes_cents, 124_000);
    assert_eq!(sep.pro_labore_for_annex_iii_cents, Some(308_000));
    assert!(report.cash_flow[..2]
        .iter()
        .all(|m| m.annex.as_deref() == Some("III")));
}

#[test]
fn the_mei_has_no_pro_labore_in_the_cash_flow() {
    let mut tax = input("2026-03", months("2026-01", 3, 500_000));
    tax.regime = TaxRegime::Mei;
    tax.monthly_costs_cents = 20_000;
    let report = tax_report(&tax).unwrap();
    // DAS-MEI 86,05; lucro 5.000 − 86,05 − 200 = 4.713,95; no pró-labore nor Simples details.
    for month in &report.cash_flow {
        assert_eq!(month.company_taxes_cents, 8_605);
        assert_eq!(
            (month.pro_labore_cents, month.inss_cents, month.irrf_cents),
            (0, 0, 0)
        );
        assert_eq!(month.profit_cents, 471_395);
        assert_eq!(month.leftover_cents, 471_395);
        assert!(month.rbt12_cents.is_none() && month.fator_r.is_none() && month.annex.is_none());
        assert!(month.pro_labore_for_annex_iii_cents.is_none());
    }
}

#[test]
fn fixed_costs_that_start_during_the_period() {
    let revenue = months("2026-07", 3, 1_000_000);
    let mut tax = input("2026-09", revenue);
    // R$ 500,00 every month, plus the accountant (R$ 300,00) only from August on.
    tax.monthly_costs_cents = 50_000;
    tax.fixed_costs_by_month = vec![
        MonthAmount {
            month: "2026-08".into(),
            cents: 30_000,
        },
        MonthAmount {
            month: "2026-09".into(),
            cents: 30_000,
        },
    ];
    let report = tax_report(&tax).unwrap();

    // Typical month: the costs in effect in the reference month (September), in full, not
    // averaged: 500,00 + 300,00 = 800,00.
    // Sobra: 10.000 − 600 (DAS 6%) − 308 (INSS) − 800 = 8.292,00.
    assert_eq!(report.leftover.costs_cents, 80_000);
    assert_eq!(report.leftover.leftover_cents, 829_200);

    // Month by month: July without the accountant, August and September with it.
    let fixed: Vec<i64> = report
        .cash_flow
        .iter()
        .map(|m| m.fixed_costs_cents)
        .collect();
    assert_eq!(fixed, [50_000, 80_000, 80_000]);
    let leftover: Vec<i64> = report.cash_flow.iter().map(|m| m.leftover_cents).collect();
    // 10.000 − 600 − 308 − 500 = 8.592; with the accountant 8.292.
    assert_eq!(leftover, [859_200, 829_200, 829_200]);
}

#[test]
fn fixed_costs_that_ended_before_the_reference_month() {
    let revenue = months("2026-07", 3, 1_000_000);
    let mut tax = input("2026-09", revenue);
    // R$ 500,00 every month, plus a course (R$ 300,00) that lasted only July and August.
    tax.monthly_costs_cents = 50_000;
    tax.fixed_costs_by_month = vec![
        MonthAmount {
            month: "2026-07".into(),
            cents: 30_000,
        },
        MonthAmount {
            month: "2026-08".into(),
            cents: 30_000,
        },
    ];
    let report = tax_report(&tax).unwrap();

    // Typical month (September): the course is over, so it is not spread over the months either.
    // Sobra: 10.000 − 600 − 308 − 500 = 8.592,00.
    assert_eq!(report.leftover.costs_cents, 50_000);
    assert_eq!(report.leftover.leftover_cents, 859_200);

    // Month by month: the course in full in July and August only.
    let fixed: Vec<i64> = report
        .cash_flow
        .iter()
        .map(|m| m.fixed_costs_cents)
        .collect();
    assert_eq!(fixed, [80_000, 80_000, 50_000]);
    let leftover: Vec<i64> = report.cash_flow.iter().map(|m| m.leftover_cents).collect();
    assert_eq!(leftover, [829_200, 829_200, 859_200]);
}
