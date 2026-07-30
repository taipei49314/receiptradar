//! L1 rule-based field extraction from OCR text lines.

use crate::explain::{AmountCandidate, ExplainTrace};
use crate::money::{Iso4217, Money};
use crate::types::{Field, FieldSource, TextBlock};
use regex::Regex;
use std::sync::OnceLock;

fn re_amount() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:NT\$|TWD|USD|\$|¥|円|€)?\s*([0-9]{1,3}(?:,[0-9]{3})*(?:\.[0-9]{1,2})?|[0-9]+(?:\.[0-9]{1,2})?)",
        )
        .expect("amount re")
    })
}

fn re_phone() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"0\d{1,3}-?\d{6,8}|\d{4}-\d{3}-\d{3}").expect("phone re"))
}

fn re_ban() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b\d{8}\b").expect("ban re"))
}

fn re_invoice() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)([A-Z]{2}\d{8})").expect("invoice re"))
}

fn re_roc_date() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(民國)?\s*(\d{2,3})[./年](\d{1,2})[./月](\d{1,2})").expect("roc")
    })
}

fn re_iso_date() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(20\d{2})[-/.](\d{1,2})[-/.](\d{1,2})").expect("iso"))
}

/// Ranked amount extraction result.
#[derive(Debug, Clone)]
pub struct ExtractedFields {
    pub merchant: Option<Field<String>>,
    pub total: Option<Field<Money>>,
    pub transacted_at: Option<Field<String>>,
    pub invoice_id: Option<Field<String>>,
}

pub fn extract_l1_fields(
    blocks: &[TextBlock],
    default_currency: Iso4217,
    explain: &mut ExplainTrace,
) -> ExtractedFields {
    explain.step("extract", "L1 rules: amount / date / merchant / invoice");
    let lines: Vec<&str> = blocks.iter().map(|b| b.text.as_str()).collect();
    let joined = lines.join("\n");

    let mut candidates: Vec<AmountCandidate> = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        // Skip phone / pure invoice-id lines unless they also look like totals.
        if !line_has_total_keyword(line) {
            if re_phone().is_match(line) {
                continue;
            }
            if line.contains("電話") || line.to_ascii_uppercase().contains("TEL") {
                continue;
            }
            if line.contains("發票號碼") || line.contains("統一編號") {
                continue;
            }
            // Date-only lines produce noisy digit matches
            if (re_iso_date().is_match(line) || re_roc_date().is_match(line))
                && (!re_amount().is_match(line)
                    || line.chars().filter(|c| c.is_ascii_digit()).count() <= 8)
                && !line_has_total_keyword(line)
            {
                continue;
            }
        }
        for caps in re_amount().captures_iter(line) {
            let raw = caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
            let num = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if num.len() == 8
                && num.chars().all(|c| c.is_ascii_digit())
                && !line_has_total_keyword(line)
            {
                continue;
            }
            // Skip fragments carved out of invoice ids like AB12345678
            if re_invoice().is_match(line) && !line_has_total_keyword(line) {
                continue;
            }
            // Tiny bare integers on non-total lines are usually noise (qty, codes)
            if !line_has_total_keyword(line)
                && !raw.contains('.')
                && !raw.contains('$')
                && !raw.contains('元')
                && num.len() <= 2
            {
                continue;
            }
            let currency = detect_currency(line, &raw, default_currency);
            if let Ok(money) = Money::from_major_str(num, currency) {
                let mut score = 10;
                if line_has_total_keyword(line) {
                    score += 50;
                }
                if line_has_subtotal_keyword(line) {
                    score += 20;
                }
                if raw.contains('$')
                    || raw.contains('元')
                    || raw.to_ascii_uppercase().contains("NT")
                {
                    score += 5;
                }
                if money.amount_minor > 0 && money.amount_minor < 10_000_000 * 100 {
                    score += 5;
                }
                if line_idx == 0 && money.amount_minor < 100 {
                    score -= 5;
                }
                candidates.push(AmountCandidate {
                    raw: raw.clone(),
                    amount_minor: money.amount_minor,
                    currency: currency.to_string(),
                    rank_score: score,
                    reason: format!("line {line_idx}: {line}"),
                });
            }
        }
    }
    candidates.sort_by_key(|b| std::cmp::Reverse(b.rank_score));
    explain.amount_candidates = candidates.clone();

    let total = candidates.first().map(|c| {
        let cur = Iso4217::parse(&c.currency).unwrap_or(default_currency);
        Field::new(
            Money::new(c.amount_minor, cur),
            (0.5 + (c.rank_score as f32) / 200.0).min(0.95),
            FieldSource::Rule,
        )
    });
    if let Some(ref t) = total {
        explain.step(
            "extract",
            format!(
                "picked total {} {} minor={}",
                t.value.currency,
                t.value.display_major(),
                t.value.amount_minor
            ),
        );
    }

    let merchant = guess_merchant(&lines, explain);
    let transacted_at = guess_date(&joined, explain);
    let invoice_id = re_invoice()
        .captures(&joined)
        .map(|c| Field::new(c[1].to_string(), 0.85, FieldSource::Rule));

    if invoice_id.is_some() {
        explain.step("extract", "invoice id pattern matched");
    }

    ExtractedFields {
        merchant,
        total,
        transacted_at,
        invoice_id,
    }
}

fn line_has_total_keyword(line: &str) -> bool {
    let u = line.to_uppercase();
    ["合計", "總計", "總額", "應收", "AMOUNT", "TOTAL", "應稅"]
        .iter()
        .any(|k| line.contains(k) || u.contains(k))
}

fn line_has_subtotal_keyword(line: &str) -> bool {
    ["小計", "銷售額", "未稅"].iter().any(|k| line.contains(k))
}

fn detect_currency(line: &str, raw: &str, default: Iso4217) -> Iso4217 {
    let u = format!("{line}{raw}").to_uppercase();
    if u.contains("TWD") || u.contains("NT$") || u.contains("NTD") || line.contains('元') {
        return Iso4217::TWD;
    }
    if u.contains("USD") {
        return Iso4217::USD;
    }
    if u.contains("JPY") || u.contains('円') || line.contains('¥') {
        return Iso4217::JPY;
    }
    if u.contains("EUR") || raw.contains('€') {
        return Iso4217::EUR;
    }
    default
}

fn guess_merchant(lines: &[&str], explain: &mut ExplainTrace) -> Option<Field<String>> {
    for line in lines.iter().take(6) {
        let t = line.trim();
        if t.is_empty() || t.len() < 2 {
            continue;
        }
        if re_amount().is_match(t) && t.len() < 12 {
            continue;
        }
        if re_phone().is_match(t) {
            continue;
        }
        if re_ban().is_match(t) && t.len() == 8 {
            continue;
        }
        if line_has_total_keyword(t) {
            continue;
        }
        if re_iso_date().is_match(t) || re_roc_date().is_match(t) {
            continue;
        }
        explain.step("extract", format!("merchant candidate: {t}"));
        return Some(Field::new(t.to_string(), 0.6, FieldSource::Rule));
    }
    None
}

fn guess_date(text: &str, explain: &mut ExplainTrace) -> Option<Field<String>> {
    if let Some(c) = re_iso_date().captures(text) {
        let y = &c[1];
        let m: u32 = c[2].parse().ok()?;
        let d: u32 = c[3].parse().ok()?;
        let s = format!("{y}-{m:02}-{d:02}");
        explain.step("extract", format!("iso date {s}"));
        return Some(Field::new(s, 0.8, FieldSource::Rule));
    }
    if let Some(c) = re_roc_date().captures(text) {
        let yyy: i32 = c[2].parse().ok()?;
        let m: u32 = c[3].parse().ok()?;
        let d: u32 = c[4].parse().ok()?;
        let year = 1911 + yyy;
        let s = format!("{year:04}-{m:02}-{d:02}");
        explain.step("extract", format!("roc date {s}"));
        return Some(Field::new(s, 0.75, FieldSource::Rule));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TextBlock;

    fn blocks(lines: &[&str]) -> Vec<TextBlock> {
        lines
            .iter()
            .map(|t| TextBlock {
                text: (*t).into(),
                confidence: 1.0,
            })
            .collect()
    }

    #[test]
    fn extract_familymart_style() {
        let mut ex = ExplainTrace::new("mock", "ocr");
        let b = blocks(&[
            "全家便利商店 臨江店",
            "電話 02-1234-5678",
            "合計 89",
            "2024/03/15",
        ]);
        let f = extract_l1_fields(&b, Iso4217::TWD, &mut ex);
        let total = f.total.expect("total");
        assert_eq!(total.value.amount_minor, 8900);
        assert!(f.merchant.unwrap().value.contains("全家"));
        assert_eq!(f.transacted_at.unwrap().value, "2024-03-15");
        assert!(!ex.amount_candidates.is_empty());
    }

    #[test]
    fn reject_ban_as_amount() {
        let mut ex = ExplainTrace::new("mock", "ocr");
        let b = blocks(&["統一編號 12345678", "總計 120"]);
        let f = extract_l1_fields(&b, Iso4217::TWD, &mut ex);
        assert_eq!(f.total.unwrap().value.amount_minor, 12000);
    }

    #[test]
    fn invoice_id() {
        let mut ex = ExplainTrace::new("mock", "ocr");
        let b = blocks(&["發票號碼 AB12345678", "合計 50"]);
        let f = extract_l1_fields(&b, Iso4217::TWD, &mut ex);
        assert_eq!(f.invoice_id.unwrap().value, "AB12345678");
    }
}
