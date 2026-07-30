//! OCR engine abstraction for ReceiptRadar.
//!
//! Mock backend is the CI default. ONNX RapidOCR is feature-gated for post-spike.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// One recognized text line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrLine {
    pub text: String,
    /// Confidence in \[0, 1\] when known; mock may use 1.0.
    pub confidence: f32,
}

/// Trait every OCR backend implements (mock, ONNX, later platform engines).
pub trait OcrEngine: Send + Sync {
    /// Engine name for explain traces and debugging.
    fn name(&self) -> &'static str;

    /// Recognize text lines from encoded image bytes (JPEG/PNG) or mock payloads.
    fn recognize(&self, image_bytes: &[u8]) -> Result<Vec<OcrLine>, OcrError>;
}

/// OCR failures.
#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("empty image input")]
    EmptyInput,
    #[error("onnx backend not enabled (build with real models after PR-A04/A05 spike)")]
    OnnxUnavailable,
    #[error("backend error: {0}")]
    Backend(String),
}

/// Deterministic mock engine for CI and parser unit tests.
///
/// If bytes start with `RRADAR_MOCK_OCR\n`, remaining UTF-8 lines are returned.
/// Otherwise returns a fixed FamilyMart-style receipt.
#[derive(Debug, Default, Clone, Copy)]
pub struct MockOcrEngine;

const MAGIC: &[u8] = b"RRADAR_MOCK_OCR\n";

impl OcrEngine for MockOcrEngine {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn recognize(&self, image_bytes: &[u8]) -> Result<Vec<OcrLine>, OcrError> {
        if image_bytes.is_empty() {
            return Err(OcrError::EmptyInput);
        }
        if image_bytes.starts_with(MAGIC) {
            let text = String::from_utf8_lossy(&image_bytes[MAGIC.len()..]);
            let lines: Vec<OcrLine> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| OcrLine {
                    text: l.to_string(),
                    confidence: 1.0,
                })
                .collect();
            if lines.is_empty() {
                return Err(OcrError::EmptyInput);
            }
            return Ok(lines);
        }
        Ok(vec![
            OcrLine {
                text: "FAMILYMART".into(),
                confidence: 1.0,
            },
            OcrLine {
                text: "合計 89".into(),
                confidence: 1.0,
            },
            OcrLine {
                text: "2024-06-01".into(),
                confidence: 1.0,
            },
        ])
    }
}

/// Placeholder ONNX engine — returns error until models are pinned (PR-A05).
#[derive(Debug, Default, Clone, Copy)]
pub struct OnnxOcrEngine;

impl OcrEngine for OnnxOcrEngine {
    fn name(&self) -> &'static str {
        "onnx-rapidocr"
    }

    fn recognize(&self, image_bytes: &[u8]) -> Result<Vec<OcrLine>, OcrError> {
        if image_bytes.is_empty() {
            return Err(OcrError::EmptyInput);
        }
        Err(OcrError::OnnxUnavailable)
    }
}

/// Select engine by name (`mock` default; `onnx` stubs unavailable).
pub fn engine_by_name(name: &str) -> Result<Box<dyn OcrEngine>, OcrError> {
    match name.to_ascii_lowercase().as_str() {
        "mock" | "" => Ok(Box::new(MockOcrEngine)),
        "onnx" | "onnx-rapidocr" => Ok(Box::new(OnnxOcrEngine)),
        other => Err(OcrError::Backend(format!("unknown engine: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_rejects_empty() {
        let eng = MockOcrEngine;
        assert!(matches!(eng.recognize(&[]), Err(OcrError::EmptyInput)));
    }

    #[test]
    fn mock_returns_lines() {
        let eng = MockOcrEngine;
        let lines = eng.recognize(b"fake-jpeg").expect("ok");
        assert_eq!(eng.name(), "mock");
        assert!(lines.iter().any(|l| l.text.contains("89")));
    }

    #[test]
    fn mock_magic_payload() {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice("HELLO\n合計 12\n".as_bytes());
        let lines = MockOcrEngine.recognize(&bytes).unwrap();
        assert_eq!(lines[0].text, "HELLO");
    }

    #[test]
    fn onnx_unavailable() {
        let err = OnnxOcrEngine.recognize(b"x").unwrap_err();
        assert!(matches!(err, OcrError::OnnxUnavailable));
    }
}
