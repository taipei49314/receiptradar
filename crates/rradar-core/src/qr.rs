//! Taiwan e-invoice QR offline structural decode (Appendix A shape).
//!
//! Not a claim of MoF endorsement. Parses common left-QR concatenated payloads
//! used on thermal e-invoice proof slips.

use crate::explain::ExplainTrace;
use crate::money::{Iso4217, Money};
use crate::types::{Field, FieldSource};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum QrParseError {
    #[error("payload too short for TW e-invoice left QR")]
    TooShort,
    #[error("invalid date in payload")]
    BadDate,
    #[error("invalid amount hex")]
    BadAmount,
}

/// Parsed fields from a TW e-invoice QR payload (primarily left QR).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TwEinvoiceQr {
    pub invoice_id: String,
    /// ISO date YYYY-MM-DD
    pub transacted_at: String,
    pub total: Money,
    pub sales_amount_major: Option<i64>,
    pub buyer_ban: Option<String>,
    pub seller_ban: Option<String>,
    pub random_code: Option<String>,
}

/// Parse a common left-QR style string:
/// `InvoiceNo(10) + ROC_date(7) + Random(4) + SalesHex(8) + TotalHex(8) + BuyerBAN(8) + SellerBAN(8) + …`
///
/// Amounts in the QR are integer TWD major units encoded as 8-char hex.
pub fn parse_tw_einvoice_left_qr(payload: &str) -> Result<TwEinvoiceQr, QrParseError> {
    let p = payload.trim();
    // Minimum: 10+7+4+8+8 = 37
    if p.len() < 37 {
        return Err(QrParseError::TooShort);
    }
    let invoice_id = p[0..10].to_string();
    let roc_date = &p[10..17];
    let random_code = Some(p[17..21].to_string());
    let sales_hex = &p[21..29];
    let total_hex = &p[29..37];

    let sales_major = i64::from_str_radix(sales_hex, 16).map_err(|_| QrParseError::BadAmount)?;
    let total_major = i64::from_str_radix(total_hex, 16).map_err(|_| QrParseError::BadAmount)?;
    let transacted_at = roc_yyyymmdd_to_iso(roc_date).ok_or(QrParseError::BadDate)?;

    let buyer_ban = if p.len() >= 45 {
        let b = &p[37..45];
        if b.chars().all(|c| c == '0') {
            None
        } else {
            Some(b.to_string())
        }
    } else {
        None
    };
    let seller_ban = if p.len() >= 53 {
        Some(p[45..53].to_string())
    } else {
        None
    };

    // Prefer total (價稅合計) over sales (未稅)
    let total = Money::from_major_i64(total_major, Iso4217::TWD);

    Ok(TwEinvoiceQr {
        invoice_id,
        transacted_at,
        total,
        sales_amount_major: Some(sales_major),
        buyer_ban,
        seller_ban,
        random_code,
    })
}

fn roc_yyyymmdd_to_iso(roc: &str) -> Option<String> {
    if roc.len() != 7 || !roc.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let yyy: i32 = roc[0..3].parse().ok()?;
    let mm = &roc[3..5];
    let dd = &roc[5..7];
    let year = 1911 + yyy;
    if !(1..=12).contains(&mm.parse::<u32>().ok()?) {
        return None;
    }
    if !(1..=31).contains(&dd.parse::<u32>().ok()?) {
        return None;
    }
    Some(format!("{year:04}-{mm}-{dd}"))
}

/// Merge left (+ optional right) payloads; v0.1 uses left for money/date/id.
pub fn parse_tw_einvoice_payloads(
    left: &str,
    _right: Option<&str>,
    explain: &mut ExplainTrace,
) -> Result<TwEinvoiceQr, QrParseError> {
    explain.step("qr", "parsing TW e-invoice left QR (Appendix A)");
    let parsed = parse_tw_einvoice_left_qr(left)?;
    explain.step(
        "qr",
        format!(
            "invoice_id={} date={} total_minor={}",
            parsed.invoice_id, parsed.transacted_at, parsed.total.amount_minor
        ),
    );
    Ok(parsed)
}

pub fn invoice_id_field(qr: &TwEinvoiceQr) -> Field<String> {
    Field::new(qr.invoice_id.clone(), 0.97, FieldSource::Qr)
}

pub fn total_field(qr: &TwEinvoiceQr) -> Field<Money> {
    Field::new(qr.total, 0.97, FieldSource::Qr)
}

pub fn date_field(qr: &TwEinvoiceQr) -> Field<String> {
    Field::new(qr.transacted_at.clone(), 0.95, FieldSource::Qr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain::ExplainTrace;

    /// Synthetic left QR: AB12345678 + 1130101 + 4242 + sales 0x59 (89) + total 0x59 + buyer 00000000 + seller 12345678
    fn sample_payload() -> String {
        let inv = "AB12345678";
        let date = "1130101"; // 2024-01-01
        let rand = "4242";
        let sales = format!("{:08X}", 89u32);
        let total = format!("{:08X}", 89u32);
        let buyer = "00000000";
        let seller = "12345678";
        format!("{inv}{date}{rand}{sales}{total}{buyer}{seller}")
    }

    #[test]
    fn parse_sample_left_qr() {
        let p = sample_payload();
        let q = parse_tw_einvoice_left_qr(&p).unwrap();
        assert_eq!(q.invoice_id, "AB12345678");
        assert_eq!(q.transacted_at, "2024-01-01");
        assert_eq!(q.total.amount_minor, 8900);
        assert_eq!(q.total.currency, Iso4217::TWD);
        assert_eq!(q.seller_ban.as_deref(), Some("12345678"));
    }

    #[test]
    fn prefer_total_over_sales() {
        let inv = "CD99999999";
        let date = "1121231";
        let rand = "1111";
        let sales = format!("{:08X}", 80u32);
        let total = format!("{:08X}", 88u32);
        let payload = format!("{inv}{date}{rand}{sales}{total}0000000012345678");
        let q = parse_tw_einvoice_left_qr(&payload).unwrap();
        assert_eq!(q.total.amount_minor, 8800);
        assert_eq!(q.transacted_at, "2023-12-31");
    }

    #[test]
    fn explain_merge() {
        let mut ex = ExplainTrace::new("test", "qr");
        let q = parse_tw_einvoice_payloads(&sample_payload(), None, &mut ex).unwrap();
        assert_eq!(q.invoice_id, "AB12345678");
        assert!(!ex.steps.is_empty());
    }
}
