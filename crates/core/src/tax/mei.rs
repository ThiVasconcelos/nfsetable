//! MEI: annual revenue limit (proportional in the opening year), where the revenue stands against
//! it, and the monthly DAS-MEI of a service provider.

use super::money::round_money;
use super::month::Month;
use super::tables::Tables;
use crate::model::{MeiStatus, SimplesActivity};
use rust_decimal::Decimal;

/// Limit of the reference year: the annual limit, or in the opening year the monthly limit times
/// the months from the opening month to December (a started month counts in full).
pub(crate) fn limit(tables: &Tables, reference: Month, opening: Option<Month>) -> Decimal {
    let mei = tables.mei;
    match opening {
        Some(opening) if opening.year == reference.year => {
            mei.opening_year_monthly_limit * Decimal::from(13 - opening.month)
        }
        _ => mei.annual_limit,
    }
}

/// Within the limit, above it by up to the tolerance (20%), or beyond the tolerance.
pub(crate) fn status(revenue: Decimal, limit: Decimal, tolerance: Decimal) -> MeiStatus {
    if revenue <= limit {
        MeiStatus::Within
    } else if revenue <= limit * (Decimal::ONE + tolerance) {
        MeiStatus::UpToTolerance
    } else {
        MeiStatus::AboveTolerance
    }
}

/// Software development and IT services (the activities subject to the Fator R) cannot be MEI
/// when the tables say so (LC 123/2006, art. 18-A, § 4º, I).
pub(crate) fn it_blocked(activity: SimplesActivity, tables: &Tables) -> bool {
    activity == SimplesActivity::FatorR && !tables.mei.it_services_allowed
}

/// Monthly DAS-MEI of a service provider by tax: the INSS is a share (5%) of the minimum wage of
/// the month, the ISS and, from 2027, the CBS and the IBS fixed amounts, each rounded to the cent
/// (0,994 and 0,006 become 0,99 and 0,01). Taxes without an amount in the tables are left out.
pub(crate) fn das(tables: &Tables) -> Vec<(String, Decimal)> {
    let das = tables.mei_das;
    let inss = round_money(tables.minimum_wage * das.inss_rate_on_minimum_wage);
    let fixed = [("iss", das.iss), ("cbs", das.cbs), ("ibs", das.ibs)]
        .into_iter()
        .filter(|(_, amount)| !amount.is_zero())
        .map(|(tax, amount)| (tax.to_string(), round_money(amount)));
    std::iter::once(("inss".to_string(), inss))
        .chain(fixed)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::data;
    use rust_decimal_macros::dec;

    use crate::tax::month::month;
    use crate::tax::tables::tables_for;

    fn das_of(text: &str) -> Vec<(String, Decimal)> {
        das(&data().unwrap().tables(month(text)))
    }

    fn parts(parts: &[(&str, Decimal)]) -> Vec<(String, Decimal)> {
        parts.iter().map(|(t, a)| (t.to_string(), *a)).collect()
    }

    #[test]
    fn das_follows_the_minimum_wage_of_the_year() {
        // 2026: 5% × 1.621,00 = 81,05 + ISS 5,00.
        assert_eq!(
            das_of("2026-01"),
            parts(&[("inss", dec!(81.05)), ("iss", dec!(5.00))])
        );
        // 2025: 5% × 1.518,00 = 75,90 + ISS 5,00.
        assert_eq!(
            das_of("2025-12"),
            parts(&[("inss", dec!(75.90)), ("iss", dec!(5.00))])
        );
    }

    #[test]
    fn das_has_cbs_and_ibs_from_2027() {
        // 2027–2028: ISS 5,00, CBS 0,994 → 0,99 and IBS 0,006 → 0,01 (R$ 1,00 together); the INSS
        // still uses the 2026 minimum wage, the latest in the tables.
        let expected = parts(&[
            ("inss", dec!(81.05)),
            ("iss", dec!(5.00)),
            ("cbs", dec!(0.99)),
            ("ibs", dec!(0.01)),
        ]);
        assert_eq!(das_of("2027-01"), expected);
        assert_eq!(das_of("2028-12"), expected);
        // 2030: ISS 4,00, CBS 1,00, IBS 0,40.
        assert_eq!(
            das_of("2030-06"),
            parts(&[
                ("inss", dec!(81.05)),
                ("iss", dec!(4.00)),
                ("cbs", dec!(1.00)),
                ("ibs", dec!(0.40))
            ])
        );
        // From 2033 the ISS is gone: CBS 1,00 and IBS 2,00.
        assert_eq!(
            das_of("2033-01"),
            parts(&[
                ("inss", dec!(81.05)),
                ("cbs", dec!(1.00)),
                ("ibs", dec!(2.00))
            ])
        );
    }

    #[test]
    fn opening_year_limit() {
        let tables = tables_for("2026-09");
        let reference = month("2026-09");
        // Opened in September: 4 months × 6.750 = 27.000 (the example of the research).
        assert_eq!(
            limit(&tables, reference, Some(month("2026-09"))),
            dec!(27000)
        );
        assert_eq!(
            limit(&tables, reference, Some(month("2026-01"))),
            dec!(81000)
        );
        assert_eq!(
            limit(&tables, reference, Some(month("2025-07"))),
            dec!(81000)
        );
        assert_eq!(limit(&tables, reference, None), dec!(81000));
    }

    #[test]
    fn limits_of_lc_238() {
        // LC 238/2026: R$ 110.000,00 in 2027 (9.166,67 a month in the opening year) and
        // R$ 140.000,00 from 2028 on (11.666,67 a month).
        let reference = month("2027-09");
        let tables = tables_for("2027-09");
        assert_eq!(limit(&tables, reference, None), dec!(110000));
        // Opened in September 2027: 4 months × 9.166,67 = 36.666,68.
        assert_eq!(
            limit(&tables, reference, Some(month("2027-09"))),
            dec!(36666.68)
        );
        let reference = month("2028-03");
        let tables = tables_for("2028-03");
        assert_eq!(
            limit(&tables, reference, Some(month("2027-05"))),
            dec!(140000)
        );
        // Opened in March 2028: 10 months × 11.666,67 = 116.666,70.
        assert_eq!(
            limit(&tables, reference, Some(month("2028-03"))),
            dec!(116666.70)
        );
        assert!(tables_for("2031-01")
            .fallbacks
            .iter()
            .all(|f| f.label != "MEI"));
    }

    #[test]
    fn status_bands() {
        let tolerance = dec!(0.2);
        assert_eq!(
            status(dec!(81000), dec!(81000), tolerance),
            MeiStatus::Within
        );
        assert_eq!(
            status(dec!(81000.01), dec!(81000), tolerance),
            MeiStatus::UpToTolerance
        );
        assert_eq!(
            status(dec!(97200), dec!(81000), tolerance),
            MeiStatus::UpToTolerance
        );
        assert_eq!(
            status(dec!(97200.01), dec!(81000), tolerance),
            MeiStatus::AboveTolerance
        );
    }
}
