//! OCR engine abstraction for ReceiptRadar.
//!
//! Mock backend is the CI default. ONNX path: `onnx` module + optional `--features onnx`.

#![deny(unsafe_code)]

pub mod onnx;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub use onnx::{
    auto_ort_dylib, ensure_ort_dylib_env, onnx_feature_enabled, OnnxConfig, OnnxOcrEngine,
};

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
    #[error("onnx backend not enabled or models not ready (see models/README.md)")]
    OnnxUnavailable,
    #[error("{0}")]
    OnnxUnavailableWithHint(String),
    #[error("backend error: {0}")]
    Backend(String),
}

/// Deterministic mock engine for CI and parser unit tests.
///
/// If bytes start with `RRADAR_MOCK_OCR` + LF or CRLF, remaining UTF-8 lines are returned.
/// Otherwise returns a fixed FamilyMart-style receipt.
#[derive(Debug, Default, Clone, Copy)]
pub struct MockOcrEngine;

const MAGIC_PREFIX: &[u8] = b"RRADAR_MOCK_OCR";

/// Returns payload after mock OCR magic, accepting `\n` or `\r\n` terminators.
pub fn strip_mock_ocr_magic(image_bytes: &[u8]) -> Option<&[u8]> {
    if !image_bytes.starts_with(MAGIC_PREFIX) {
        return None;
    }
    let rest = &image_bytes[MAGIC_PREFIX.len()..];
    if rest.starts_with(b"\r\n") {
        Some(&rest[2..])
    } else if rest.starts_with(b"\n") {
        Some(&rest[1..])
    } else {
        None
    }
}

impl OcrEngine for MockOcrEngine {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn recognize(&self, image_bytes: &[u8]) -> Result<Vec<OcrLine>, OcrError> {
        if image_bytes.is_empty() {
            return Err(OcrError::EmptyInput);
        }
        if let Some(payload) = strip_mock_ocr_magic(image_bytes) {
            let text = String::from_utf8_lossy(payload);
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

/// Select engine by name.
///
/// - `mock` — always available
/// - `onnx` — loads `RRADAR_MODELS_DIR` or `./models` via [`OnnxOcrEngine`]
pub fn engine_by_name(name: &str) -> Result<Box<dyn OcrEngine>, OcrError> {
    match name.to_ascii_lowercase().as_str() {
        "mock" | "" => Ok(Box::new(MockOcrEngine)),
        "onnx" | "onnx-rapidocr" => {
            let dir = std::env::var("RRADAR_MODELS_DIR").unwrap_or_else(|_| "models".into());
            ensure_ort_dylib_env(Path::new(&dir));
            let cfg = OnnxConfig::from_models_dir(&dir);
            match OnnxOcrEngine::new(cfg.clone()) {
                Ok(eng) => {
                    // Only mark inference_enabled when feature is on; otherwise
                    // still surface a precise hint on first recognize().
                    if onnx_feature_enabled() {
                        Ok(Box::new(eng))
                    } else {
                        Ok(Box::new(OnnxOcrEngine::unvalidated(cfg)))
                    }
                }
                Err(_) => Ok(Box::new(OnnxOcrEngine::unvalidated(cfg))),
            }
        }
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
        let mut bytes = b"RRADAR_MOCK_OCR\n".to_vec();
        bytes.extend_from_slice("HELLO\n合計 12\n".as_bytes());
        let lines = MockOcrEngine.recognize(&bytes).unwrap();
        assert_eq!(lines[0].text, "HELLO");
    }

    #[test]
    fn mock_magic_crlf_payload() {
        let mut bytes = b"RRADAR_MOCK_OCR\r\n".to_vec();
        bytes.extend_from_slice(b"STARBUCKS\r\nTOTAL $5.45\r\n");
        let lines = MockOcrEngine.recognize(&bytes).unwrap();
        assert_eq!(lines[0].text, "STARBUCKS");
        assert!(lines.iter().any(|l| l.text.contains("5.45")));
    }

    #[test]
    fn onnx_engine_by_name_without_models() {
        let eng = engine_by_name("onnx").unwrap();
        assert_eq!(eng.name(), "onnx-rapidocr");
        let err = eng.recognize(b"x").unwrap_err();
        assert!(matches!(
            err,
            OcrError::OnnxUnavailable | OcrError::OnnxUnavailableWithHint(_) | OcrError::Backend(_)
        ));
    }
}
