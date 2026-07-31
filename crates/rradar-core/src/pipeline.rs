//! Capture → QR prefer → OCR → L1 extract → category orchestration.

use crate::category::CategoryEngine;
use crate::explain::ExplainTrace;
use crate::extract::extract_l1_fields;
use crate::money::{Iso4217, Money};
use crate::preprocess::{preprocess, PreprocessConfig};
use crate::qr::{date_field, invoice_id_field, parse_tw_einvoice_left_qr, total_field};
use crate::types::{Field, FieldSource, ReceiptDraft, SourcePath, TextBlock};
use rradar_ocr::{strip_mock_ocr_magic, OcrEngine, OcrError};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("ocr: {0}")]
    Ocr(#[from] OcrError),
    #[error("empty input")]
    Empty,
}

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub default_currency: Iso4217,
    /// Optional raw TW e-invoice left QR string (offline; no image decode required).
    pub qr_payload: Option<String>,
    pub preprocess: PreprocessConfig,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            default_currency: Iso4217::TWD,
            qr_payload: None,
            preprocess: PreprocessConfig::default(),
        }
    }
}

/// Process file path: `.txt` / mock magic / sibling `.ocr.txt` / image bytes via OCR engine.
pub fn process_path(
    path: &Path,
    engine: &dyn OcrEngine,
    categories: &CategoryEngine,
    opts: ProcessOptions,
) -> Result<ReceiptDraft, ProcessError> {
    let bytes = std::fs::read(path)?;
    let sidecar = path.with_extension("ocr.txt");
    let sidecar2 = {
        let mut s = path.as_os_str().to_os_string();
        s.push(".ocr.txt");
        Path::new(&s).to_path_buf()
    };

    let text_override = if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("txt"))
        .unwrap_or(false)
    {
        Some(String::from_utf8_lossy(&bytes).into_owned())
    } else if let Some(payload) = strip_mock_ocr_magic(&bytes) {
        // Accept LF or CRLF after magic so Windows autocrlf checkouts still work.
        Some(String::from_utf8_lossy(payload).into_owned())
    } else if sidecar.is_file() {
        Some(std::fs::read_to_string(&sidecar)?)
    } else if sidecar2.is_file() {
        Some(std::fs::read_to_string(&sidecar2)?)
    } else {
        None
    };

    process_bytes(&bytes, text_override.as_deref(), engine, categories, opts)
}

pub fn process_bytes(
    image_or_payload: &[u8],
    text_override: Option<&str>,
    engine: &dyn OcrEngine,
    categories: &CategoryEngine,
    opts: ProcessOptions,
) -> Result<ReceiptDraft, ProcessError> {
    if image_or_payload.is_empty() && text_override.is_none() && opts.qr_payload.is_none() {
        return Err(ProcessError::Empty);
    }

    let mut explain = ExplainTrace::new(engine.name(), "pending");
    let pre = preprocess(
        if image_or_payload.is_empty() {
            b"empty"
        } else {
            image_or_payload
        },
        opts.preprocess,
    );
    explain.step(
        "preprocess",
        format!(
            "max_edge={} sha256={}",
            pre.max_edge,
            &pre.content_hash_hex[..16.min(pre.content_hash_hex.len())]
        ),
    );

    // --- QR prefer path ---
    if let Some(ref qr) = opts.qr_payload {
        if let Ok(parsed) = parse_tw_einvoice_left_qr(qr) {
            explain.source_path = SourcePath::Qr.as_str().into();
            explain.engine_id = engine.name().into();
            explain.step("qr", "QR prefer path succeeded");
            let merchant_name = String::new();
            let cat = categories.categorize(&merchant_name, qr, &mut explain);
            let blocks = vec![TextBlock {
                text: format!("[qr] {qr}"),
                confidence: 1.0,
            }];
            let overall = 0.95f32;
            return Ok(ReceiptDraft {
                id: ReceiptDraft::new_id(),
                captured_at: now_iso(),
                merchant: Field::new(
                    parsed
                        .seller_ban
                        .clone()
                        .map(|b| format!("seller:{b}"))
                        .unwrap_or_else(|| "unknown".into()),
                    0.5,
                    FieldSource::Qr,
                ),
                total: total_field(&parsed),
                transacted_at: date_field(&parsed),
                tax: None,
                invoice_id: Some(invoice_id_field(&parsed)),
                category: cat,
                raw_text: qr.clone(),
                ocr_blocks: blocks,
                overall_confidence: overall,
                explain,
                source_path: SourcePath::Qr,
            });
        } else {
            explain.step("qr", "QR payload present but failed to parse; falling back");
        }
    }

    // --- OCR / text path ---
    let blocks: Vec<TextBlock> = if let Some(text) = text_override {
        explain.step("ocr", "using text fixture / sidecar (no pixel OCR)");
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| TextBlock {
                text: l.to_string(),
                confidence: 1.0,
            })
            .collect()
    } else {
        explain.step("ocr", format!("engine={}", engine.name()));
        let lines = engine.recognize(&pre.bytes)?;
        lines
            .into_iter()
            .map(|l| TextBlock {
                text: l.text,
                confidence: l.confidence,
            })
            .collect()
    };

    if blocks.is_empty() {
        return Err(ProcessError::Empty);
    }

    let raw_text = blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Inline QR detection: if a line looks like left-QR payload, prefer it
    for b in &blocks {
        let t = b.text.trim();
        if t.len() >= 37 && t.chars().take(10).all(|c| c.is_ascii_alphanumeric()) {
            if let Ok(parsed) = parse_tw_einvoice_left_qr(t) {
                explain.step("qr", "detected QR-like line inside OCR/text");
                let fields = extract_l1_fields(&blocks, opts.default_currency, &mut explain);
                let merchant = fields
                    .merchant
                    .unwrap_or_else(|| Field::new("unknown".into(), 0.3, FieldSource::Rule));
                let cat = categories.categorize(&merchant.value, &raw_text, &mut explain);
                explain.source_path = SourcePath::Mixed.as_str().into();
                return Ok(ReceiptDraft {
                    id: ReceiptDraft::new_id(),
                    captured_at: now_iso(),
                    merchant,
                    total: total_field(&parsed),
                    transacted_at: date_field(&parsed),
                    tax: None,
                    invoice_id: Some(invoice_id_field(&parsed)),
                    category: cat,
                    raw_text,
                    ocr_blocks: blocks,
                    overall_confidence: 0.92,
                    explain,
                    source_path: SourcePath::Mixed,
                });
            }
        }
    }

    let fields = extract_l1_fields(&blocks, opts.default_currency, &mut explain);
    let merchant = fields
        .merchant
        .unwrap_or_else(|| Field::new("unknown".into(), 0.2, FieldSource::Rule));
    let total = fields.total.unwrap_or_else(|| {
        Field::new(Money::new(0, opts.default_currency), 0.1, FieldSource::Rule)
    });
    let transacted_at = fields
        .transacted_at
        .unwrap_or_else(|| Field::new(now_iso()[..10].to_string(), 0.2, FieldSource::Rule));
    let cat = categories.categorize(&merchant.value, &raw_text, &mut explain);

    let overall = (merchant.confidence + total.confidence + cat.confidence) / 3.0;
    explain.source_path = SourcePath::Ocr.as_str().into();

    Ok(ReceiptDraft {
        id: ReceiptDraft::new_id(),
        captured_at: now_iso(),
        merchant,
        total,
        transacted_at,
        tax: None,
        invoice_id: fields.invoice_id,
        category: cat,
        raw_text,
        ocr_blocks: blocks,
        overall_confidence: overall,
        explain,
        source_path: SourcePath::Ocr,
    })
}

pub fn utc_now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let hh = (secs / 3600) % 24;
    let mm = (secs / 60) % 60;
    let ss = secs % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn now_iso() -> String {
    utc_now_iso()
}

/// Howard Hinnant civil_from_days (UTC).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rradar_ocr::MockOcrEngine;

    #[test]
    fn process_text_fixture_style() {
        let text = "全家便利商店\n合計 89\n2024-05-01\n";
        let eng = MockOcrEngine;
        let cat = CategoryEngine::with_seed();
        let draft = process_bytes(
            b"not-empty",
            Some(text),
            &eng,
            &cat,
            ProcessOptions::default(),
        )
        .unwrap();
        assert_eq!(draft.total.value.amount_minor, 8900);
        assert_eq!(draft.category.value, crate::category::CAT_GROCERY);
        assert_eq!(draft.source_path, SourcePath::Ocr);
    }

    #[test]
    fn process_qr_option() {
        let inv = "AB12345678";
        let date = "1130101";
        let rand = "4242";
        let sales = format!("{:08X}", 89u32);
        let total = format!("{:08X}", 89u32);
        let payload = format!("{inv}{date}{rand}{sales}{total}0000000012345678");
        let eng = MockOcrEngine;
        let cat = CategoryEngine::with_seed();
        let opts = ProcessOptions {
            qr_payload: Some(payload),
            ..Default::default()
        };
        let draft = process_bytes(b"x", None, &eng, &cat, opts).unwrap();
        assert_eq!(draft.source_path, SourcePath::Qr);
        assert_eq!(draft.total.value.amount_minor, 8900);
        assert_eq!(draft.invoice_id.unwrap().value, "AB12345678");
    }

    #[test]
    fn mock_engine_bytes() {
        let eng = MockOcrEngine;
        let cat = CategoryEngine::with_seed();
        let draft =
            process_bytes(b"fake-jpeg", None, &eng, &cat, ProcessOptions::default()).unwrap();
        // MockOcrEngine returns FAMILYMART + 合計 89
        assert_eq!(draft.total.value.amount_minor, 8900);
    }
}
