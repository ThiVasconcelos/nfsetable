//! Decimal helpers: conversions to and from cents, the rounding rules of the estimates and pt-BR
//! formatting for messages.
//!
//! Internally every amount is a [`Decimal`] in reais; the contract carries integer cents.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use std::str::FromStr;

/// Reais from integer cents.
pub(crate) fn from_cents(cents: i64) -> Decimal {
    Decimal::new(cents, 2)
}

/// Integer cents from reais, rounding half away from zero. Saturates on overflow, which only
/// absurd inputs (near `i64::MAX` cents) could cause.
pub(crate) fn to_cents(value: Decimal) -> i64 {
    (round_money(value) * Decimal::ONE_HUNDRED)
        .to_i64()
        .unwrap_or(if value.is_sign_negative() {
            i64::MIN
        } else {
            i64::MAX
        })
}

/// Rounds money to cents, half away from zero: the rule applied at each tax step.
pub(crate) fn round_money(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// Rounds money up (toward +∞) to the cent.
pub(crate) fn ceil_money(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::ToPositiveInfinity)
}

/// Truncates to two decimals, like the PGDAS-D does with the Fator R.
pub(crate) fn trunc2(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::ToZero)
}

/// `numerator / denominator`, or zero when the denominator is zero.
pub(crate) fn ratio(numerator: Decimal, denominator: Decimal) -> Decimal {
    if denominator.is_zero() {
        Decimal::ZERO
    } else {
        numerator / denominator
    }
}

/// A rate or ratio for the contract: the `f64` nearest to the exact decimal value.
pub(crate) fn to_f64(value: Decimal) -> f64 {
    value.to_string().parse().unwrap_or(0.0)
}

/// The exact decimal of a user-provided `f64`, from its shortest representation (0.05 → 0.05).
pub(crate) fn from_f64(value: f64) -> Option<Decimal> {
    if !value.is_finite() {
        return None;
    }
    Decimal::from_str(&value.to_string()).ok()
}

/// Brazilian currency for messages: "R$ 1.234,56" (`parse::format_brl` of the rounded cents).
pub(crate) fn brl(value: Decimal) -> String {
    crate::parse::format_brl(to_cents(value))
}

/// A rate as a pt-BR percentage for messages: 0.05 → "5%", 0.0065 → "0,65%".
pub(crate) fn percent(rate: Decimal) -> String {
    let value = (rate * Decimal::ONE_HUNDRED).normalize();
    format!("{}%", value.to_string().replace('.', ","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn rounding_rules() {
        assert_eq!(round_money(dec!(932.3105)), dec!(932.31));
        assert_eq!(round_money(dec!(0.005)), dec!(0.01));
        assert_eq!(round_money(dec!(-0.005)), dec!(-0.01));
        assert_eq!(round_money(dec!(233.008)), dec!(233.01));
        assert_eq!(ceil_money(dec!(2333.241)), dec!(2333.25));
        assert_eq!(ceil_money(dec!(-1.019)), dec!(-1.01));
        assert_eq!(trunc2(dec!(0.2799999)), dec!(0.27));
        assert_eq!(trunc2(dec!(0.3242)), dec!(0.32));
    }

    #[test]
    fn cents_round_trip() {
        assert_eq!(to_cents(from_cents(8605)), 8605);
        assert_eq!(to_cents(dec!(81.045)), 8105);
        assert_eq!(to_cents(dec!(-10.005)), -1001);
        assert_eq!(from_cents(123), dec!(1.23));
    }

    #[test]
    fn floats_convert_exactly() {
        assert_eq!(from_f64(0.05), Some(dec!(0.05)));
        assert_eq!(from_f64(0.133145), Some(dec!(0.133145)));
        assert_eq!(from_f64(f64::NAN), None);
        assert_eq!(to_f64(dec!(0.073)), 0.073);
        assert_eq!(to_f64(dec!(0.16125)), 0.16125);
    }

    #[test]
    fn pt_br_formatting() {
        assert_eq!(brl(dec!(0)), "R$ 0,00");
        assert_eq!(brl(dec!(86.05)), "R$ 86,05");
        assert_eq!(brl(dec!(1234.5)), "R$ 1.234,50");
        assert_eq!(brl(dec!(4800000)), "R$ 4.800.000,00");
        assert_eq!(brl(dec!(-15.2)), "-R$ 15,20");
        assert_eq!(percent(dec!(0.05)), "5%");
        assert_eq!(percent(dec!(0.0065)), "0,65%");
        assert_eq!(percent(dec!(0.16125)), "16,125%");
    }
}
