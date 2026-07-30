//! OCR engine abstraction for ReceiptRadar.
//!
//! v0.1 ships a mock backend for scaffolding; ONNX RapidOCR lands after
//! the PR-A04 device spike pins models and measured budgets.

#![deny(unsafe_code)]

/// Placeholder OCR line for scaffolding tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrLine {
    pub text: String,
    /// Confidence in \[0, 1\] when known; mock may use 1.0.
    pub confidence: f32,
}

/// Trait every OCR backend implements (mock, ONNX, later platform engines).
pub trait OcrEngine: Send + Sync {
    /// Engine name for explain traces and debugging.
    fn name(&self) -> &'static str;

    /// Recognize text lines from encoded image bytes (JPEG/PNG).
    /// Path-based APIs will wrap this in `rradar-core` (prefer decode in-process).
    fn recognize(&self, image_bytes: &[u8]) -> Result<Vec<OcrLine>, OcrError>;
}

/// OCR failures.
#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("empty image input")]
    EmptyInput,
    #[error("backend error: {0}")]
    Backend(String),
}

/// Deterministic mock engine for CI and parser unit tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct MockOcrEngine;

impl OcrEngine for MockOcrEngine {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn recognize(&self, image_bytes: &[u8]) -> Result<Vec<OcrLine>, OcrError> {
        if image_bytes.is_empty() {
            return Err(OcrError::EmptyInput);
        }
        // Scaffolding: pretend we read a fixed TW convenience-store style total.
        Ok(vec![
            OcrLine {
                text: "FAMILYMART".into(),
                confidence: 1.0,
            },
            OcrLine {
                text: "合計 89".into(),
                confidence: 1.0,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_rejects_empty() {
        let eng = MockOcrEngine;
        assert!(matches!(
            eng.recognize(&[]),
            Err(OcrError::EmptyInput)
        ));
    }

    #[test]
    fn mock_returns_lines() {
        let eng = MockOcrEngine;
        let lines = eng.recognize(b"fake-jpeg").expect("ok");
        assert_eq!(eng.name(), "mock");
        assert!(lines.iter().any(|l| l.text.contains("89")));
    }
}
