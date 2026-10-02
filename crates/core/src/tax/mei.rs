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

/// Monthly DAS-MEI of a service provider as (INSS, ISS): the INSS is a share (5%) of the minimum
/// wage of the reference month, the ISS a fixed amount.
pub(crate) fn das(tables: &Tables) -> (Decimal, Decimal) {
    let das = tables.mei_das;
    (
        round_money(tables.minimum_wage * das.inss_rate_on_minimum_wage),
        round_money(das.iss),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::data;
    use rust_decimal_macros::dec;

    use crate::tax::month::month;
    use crate::tax::tables::tables_for;

    #[test]
    fn das_follows_the_minimum_wage_of_the_year() {
        let data = data().unwrap();
        // 2026: 5% × 1.621,00 = 81,05 + ISS 5,00.
        assert_eq!(
            das(&data.tables(month("2026-01"))),
            (dec!(81.05), dec!(5.00))
        );
        // 2025: 5% × 1.518,00 = 75,90 + ISS 5,00.
        assert_eq!(
            das(&data.tables(month("2025-12"))),
            (dec!(75.90), dec!(5.00))
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
