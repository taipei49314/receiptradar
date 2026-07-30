//! ONNX RapidOCR backend (PR-A05).
//!
//! Enabled with `--features onnx`. Without models on disk this module still
//! compiles a load path that fails with a clear error (spike must pin artifacts).

use crate::{OcrEngine, OcrError, OcrLine};
use std::path::{Path, PathBuf};

/// Configuration for on-disk ONNX det/rec packs.
#[derive(Debug, Clone)]
pub struct OnnxConfig {
    pub det_model: PathBuf,
    pub rec_model: PathBuf,
    pub keys_path: Option<PathBuf>,
    pub num_threads: usize,
}

impl OnnxConfig {
    /// Default layout under `models/` after `tools/fetch-models.sh`.
    pub fn from_models_dir(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        Self {
            det_model: dir.join("ch_PP-OCRv4_det_infer.onnx"),
            rec_model: dir.join("ch_PP-OCRv4_rec_infer.onnx"),
            keys_path: Some(dir.join("ppocr_keys_v1.txt")),
            num_threads: 2,
        }
    }

    pub fn validate_paths(&self) -> Result<(), OcrError> {
        for p in [&self.det_model, &self.rec_model] {
            if !p.is_file() {
                return Err(OcrError::Backend(format!(
                    "missing model file: {} (run spike A04, pin hash, tools/fetch-models.sh)",
                    p.display()
                )));
            }
        }
        Ok(())
    }
}

/// ONNX engine. Runtime inference is wired when `ort` feature stack is complete;
/// for now validates model presence and returns a structured unavailable path
/// so CLI/mobile can surface "download models" UX.
#[derive(Debug, Clone)]
pub struct OnnxOcrEngine {
    pub config: OnnxConfig,
    /// When false, only validate paths (CI without weights).
    pub inference_enabled: bool,
}

impl OnnxOcrEngine {
    pub fn new(config: OnnxConfig) -> Result<Self, OcrError> {
        config.validate_paths()?;
        Ok(Self {
            config,
            inference_enabled: true,
        })
    }

    /// Construct without requiring files (unit tests / feature probe).
    pub fn unvalidated(config: OnnxConfig) -> Self {
        Self {
            config,
            inference_enabled: false,
        }
    }
}

impl OcrEngine for OnnxOcrEngine {
    fn name(&self) -> &'static str {
        "onnx-rapidocr"
    }

    fn recognize(&self, image_bytes: &[u8]) -> Result<Vec<OcrLine>, OcrError> {
        if image_bytes.is_empty() {
            return Err(OcrError::EmptyInput);
        }
        if !self.inference_enabled {
            return Err(OcrError::OnnxUnavailable);
        }
        // Path check every call so missing models after delete are obvious.
        self.config.validate_paths()?;

        // Placeholder for ORT session run:
        // 1) decode image (image crate)
        // 2) det → boxes
        // 3) rec → strings + conf
        // Wired in a follow-up once A04 pins pack + we enable `ort` dep on msvc/android.
        Err(OcrError::Backend(
            "ONNX models present but inference runtime not linked yet \
             (enable full ort integration after device spike Green)"
                .into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_models_error() {
        let cfg = OnnxConfig::from_models_dir("/nonexistent/models-rradar");
        assert!(OnnxOcrEngine::new(cfg).is_err());
    }

    #[test]
    fn unvalidated_name() {
        let eng = OnnxOcrEngine::unvalidated(OnnxConfig::from_models_dir("."));
        assert_eq!(eng.name(), "onnx-rapidocr");
        assert!(matches!(
            eng.recognize(b"x"),
            Err(OcrError::OnnxUnavailable)
        ));
    }
}
