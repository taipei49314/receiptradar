//! ISO 4217-aware money: never assume 2 decimal places.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Three-letter ISO 4217 currency code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Iso4217(pub [u8; 3]);

impl Iso4217 {
    pub const TWD: Self = Self(*b"TWD");
    pub const USD: Self = Self(*b"USD");
    pub const JPY: Self = Self(*b"JPY");
    pub const EUR: Self = Self(*b"EUR");
    pub const KRW: Self = Self(*b"KRW");
    pub const CNY: Self = Self(*b"CNY");
    pub const HKD: Self = Self(*b"HKD");
    pub const GBP: Self = Self(*b"GBP");

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("???")
    }

    pub fn parse(s: &str) -> Option<Self> {
        let b = s.trim().as_bytes();
        if b.len() != 3 {
            return None;
        }
        let upper = [
            b[0].to_ascii_uppercase(),
            b[1].to_ascii_uppercase(),
            b[2].to_ascii_uppercase(),
        ];
        if upper.iter().all(|c| c.is_ascii_alphabetic()) {
            Some(Self(upper))
        } else {
            None
        }
    }

    /// ISO 4217 minor-unit exponent (not always 2).
    pub fn exponent(self) -> u8 {
        match self.as_str() {
            "JPY" | "KRW" | "VND" | "CLP" => 0,
            "BHD" | "IQD" | "JOD" | "KWD" | "OMR" | "TND" => 3,
            _ => 2,
        }
    }
}

impl fmt::Display for Iso4217 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for Iso4217 {
    fn default() -> Self {
        Self::TWD
    }
}

/// Amount in minor units + currency + exponent (redundant but migration-safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor: i64,
    pub currency: Iso4217,
    pub exponent: u8,
}

impl Default for Money {
    fn default() -> Self {
        Self::new(0, Iso4217::TWD)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("currency mismatch: {0} vs {1}")]
    CurrencyMismatch(String, String),
    #[error("invalid major amount: {0}")]
    InvalidAmount(String),
}

impl Money {
    pub fn new(amount_minor: i64, currency: Iso4217) -> Self {
        Self {
            amount_minor,
            currency,
            exponent: currency.exponent(),
        }
    }

    /// Parse a major-unit decimal string (e.g. "89", "89.5", "1,234.56") into minor units.
    pub fn from_major_str(s: &str, currency: Iso4217) -> Result<Self, MoneyError> {
        let cleaned: String = s.chars().filter(|c| *c != ',' && *c != ' ').collect();
        let exp = currency.exponent() as u32;
        let parts: Vec<&str> = cleaned.split('.').collect();
        if parts.len() > 2 {
            return Err(MoneyError::InvalidAmount(s.to_string()));
        }
        let whole: i64 = parts[0]
            .parse()
            .map_err(|_| MoneyError::InvalidAmount(s.to_string()))?;
        let mut frac_minor: i64 = 0;
        if parts.len() == 2 {
            let frac = parts[1];
            if frac.len() > exp as usize {
                return Err(MoneyError::InvalidAmount(s.to_string()));
            }
            let mut padded = frac.to_string();
            while padded.len() < exp as usize {
                padded.push('0');
            }
            if exp > 0 {
                frac_minor = padded
                    .parse()
                    .map_err(|_| MoneyError::InvalidAmount(s.to_string()))?;
            }
        }
        let sign = if whole < 0 { -1 } else { 1 };
        let whole_abs = whole.abs();
        let scale = 10i64.pow(exp);
        let amount_minor = sign * (whole_abs * scale + frac_minor);
        Ok(Self::new(amount_minor, currency))
    }

    /// Integer major units (e.g. TWD 89 from e-invoice QR) → minor.
    pub fn from_major_i64(major: i64, currency: Iso4217) -> Self {
        let scale = 10i64.pow(currency.exponent() as u32);
        Self::new(major * scale, currency)
    }

    /// Same-currency add only.
    pub fn checked_add(self, other: Money) -> Result<Money, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch(
                self.currency.to_string(),
                other.currency.to_string(),
            ));
        }
        Ok(Self::new(
            self.amount_minor.saturating_add(other.amount_minor),
            self.currency,
        ))
    }

    pub fn display_major(&self) -> String {
        let scale = 10i64.pow(self.exponent as u32);
        if self.exponent == 0 {
            return format!("{}", self.amount_minor);
        }
        let neg = self.amount_minor < 0;
        let abs = self.amount_minor.abs();
        let whole = abs / scale;
        let frac = abs % scale;
        let frac_s = format!("{:0width$}", frac, width = self.exponent as usize);
        if neg {
            format!("-{whole}.{frac_s}")
        } else {
            format!("{whole}.{frac_s}")
        }
    }
}

/// Sum moneys of the **same** currency. Mixed currencies → error (no FX).
pub fn sum_same_currency(items: &[Money]) -> Result<Option<Money>, MoneyError> {
    if items.is_empty() {
        return Ok(None);
    }
    let mut acc = items[0];
    for m in &items[1..] {
        acc = acc.checked_add(*m)?;
    }
    Ok(Some(acc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twd_exponent_is_2() {
        assert_eq!(Iso4217::TWD.exponent(), 2);
        let m = Money::from_major_str("89", Iso4217::TWD).unwrap();
        assert_eq!(m.amount_minor, 8900);
        assert_eq!(m.display_major(), "89.00");
    }

    #[test]
    fn jpy_exponent_is_0() {
        assert_eq!(Iso4217::JPY.exponent(), 0);
        let m = Money::from_major_i64(1200, Iso4217::JPY);
        assert_eq!(m.amount_minor, 1200);
        assert_eq!(m.display_major(), "1200");
    }

    #[test]
    fn reject_cross_currency_add() {
        let a = Money::new(100, Iso4217::TWD);
        let b = Money::new(100, Iso4217::USD);
        assert!(matches!(
            a.checked_add(b),
            Err(MoneyError::CurrencyMismatch(_, _))
        ));
    }

    #[test]
    fn sum_same_currency_ok() {
        let items = [Money::new(100, Iso4217::USD), Money::new(250, Iso4217::USD)];
        let s = sum_same_currency(&items).unwrap().unwrap();
        assert_eq!(s.amount_minor, 350);
    }

    #[test]
    fn sum_mixed_currency_err() {
        let items = [Money::new(100, Iso4217::TWD), Money::new(100, Iso4217::USD)];
        assert!(sum_same_currency(&items).is_err());
    }
}
