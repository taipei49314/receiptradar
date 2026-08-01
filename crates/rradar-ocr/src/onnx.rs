//! ONNX RapidOCR backend (PR-A05).
//!
//! - Default (no `onnx` feature): validates model paths; clear "how to enable" errors.
//! - With `--features onnx`: loads det/cls/rec via `paddle-ocr-rs` + ORT (`load-dynamic`).
//!
//! Weights are **not** bundled. Fetch with `tools/fetch-models.ps1` / `tools/fetch-models.sh`.
//! ONNX Runtime shared library: set `ORT_DYLIB_PATH` or place under `models/ort/` (see models/README.md).

use crate::{OcrEngine, OcrError, OcrLine};
use std::path::{Path, PathBuf};

/// Configuration for on-disk ONNX det/cls/rec packs.
#[derive(Debug, Clone)]
pub struct OnnxConfig {
    pub det_model: PathBuf,
    pub rec_model: PathBuf,
    pub cls_model: PathBuf,
    pub keys_path: Option<PathBuf>,
    pub num_threads: usize,
}

impl OnnxConfig {
    /// Default layout under `models/` after `tools/fetch-models.*`.
    pub fn from_models_dir(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        Self {
            det_model: dir.join("ch_PP-OCRv4_det_infer.onnx"),
            rec_model: dir.join("ch_PP-OCRv4_rec_infer.onnx"),
            cls_model: dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx"),
            keys_path: Some(dir.join("ppocr_keys_v1.txt")),
            num_threads: 2,
        }
    }

    /// Models directory derived from det path parent.
    pub fn models_dir(&self) -> PathBuf {
        self.det_model
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("models"))
    }

    pub fn validate_paths(&self) -> Result<(), OcrError> {
        for p in [&self.det_model, &self.rec_model, &self.cls_model] {
            if !p.is_file() {
                return Err(OcrError::Backend(format!(
                    "missing model file: {} — run tools/fetch-models.ps1 (Windows) or tools/fetch-models.sh",
                    p.display()
                )));
            }
        }
        Ok(())
    }

    /// Human-readable readiness summary for `rradar doctor`.
    pub fn status_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (label, p) in [
            ("det", &self.det_model),
            ("rec", &self.rec_model),
            ("cls", &self.cls_model),
        ] {
            out.push(format!(
                "  onnx {label}: {} ({})",
                p.display(),
                if p.is_file() { "ok" } else { "MISSING" }
            ));
        }
        #[cfg(feature = "onnx")]
        out.push("  onnx feature: ENABLED (inference linked)".into());
        #[cfg(not(feature = "onnx"))]
        out.push(
            "  onnx feature: OFF — rebuild with `cargo build -p rradar-cli --features onnx`".into(),
        );
        if let Ok(p) = std::env::var("ORT_DYLIB_PATH") {
            out.push(format!("  ORT_DYLIB_PATH: {p}"));
        } else {
            let auto = auto_ort_dylib(&self.models_dir());
            if let Some(p) = auto {
                out.push(format!("  ORT dylib (auto): {}", p.display()));
            } else {
                out.push(
                    "  ORT dylib: not found — set ORT_DYLIB_PATH or install under models/ort/"
                        .into(),
                );
            }
        }
        // Hash pins from models/manifest.sha256 (committed; weights are not).
        match crate::manifest::verify_models_dir(self.models_dir(), false) {
            Ok(checks) if checks.is_empty() => {
                out.push("  model pins: none (add lines to models/manifest.sha256)".into());
            }
            Ok(checks) => {
                let ok = checks.iter().filter(|c| c.is_ok()).count();
                let n = checks.len();
                out.push(format!(
                    "  model pins: {ok}/{n} verified{}",
                    if ok == n {
                        ""
                    } else {
                        " — run tools/fetch-models.* then rradar models verify"
                    }
                ));
                for c in checks {
                    if !c.is_ok() {
                        out.push(c.summary_line());
                    }
                }
            }
            Err(e) => out.push(format!("  model pins: error ({e})")),
        }
        out
    }
}

/// Prefer `models/ort/onnxruntime.{dll,so,dylib}` when env is unset.
pub fn auto_ort_dylib(models_dir: &Path) -> Option<PathBuf> {
    let names = [
        "onnxruntime.dll",
        "libonnxruntime.so",
        "libonnxruntime.dylib",
        "libonnxruntime.so.1",
    ];
    let candidates = [
        models_dir.join("ort"),
        models_dir.to_path_buf(),
        models_dir.join("..").join("ort"),
    ];
    for dir in candidates {
        for name in names {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Apply `ORT_DYLIB_PATH` from models dir if the env var is unset.
pub fn ensure_ort_dylib_env(models_dir: &Path) {
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return;
    }
    if let Some(p) = auto_ort_dylib(models_dir) {
        // SAFETY: only sets process env before first ORT load; single-threaded CLI path.
        std::env::set_var("ORT_DYLIB_PATH", &p);
    }
}

/// Whether this binary was compiled with the `onnx` Cargo feature.
pub fn onnx_feature_enabled() -> bool {
    cfg!(feature = "onnx")
}

/// Snapshot of whether desktop ONNX inference can run *right now*.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct OnnxReadiness {
    pub feature_enabled: bool,
    pub models_dir: String,
    pub det_present: bool,
    pub rec_present: bool,
    pub cls_present: bool,
    pub models_present: bool,
    pub pins_ok: bool,
    pub pin_ok_count: u32,
    pub pin_total: u32,
    pub ort_found: bool,
    pub ort_path: Option<String>,
    /// True only when feature + all three models exist (ORT optional until first infer).
    pub ready_for_inference: bool,
    pub hint: String,
}

/// Probe ONNX readiness under `models_dir` (does not load weights into memory).
pub fn probe_onnx_readiness(models_dir: impl AsRef<Path>) -> OnnxReadiness {
    let dir = models_dir.as_ref();
    let cfg = OnnxConfig::from_models_dir(dir);
    let det = cfg.det_model.is_file();
    let rec = cfg.rec_model.is_file();
    let cls = cfg.cls_model.is_file();
    let models_present = det && rec && cls;
    let feature = onnx_feature_enabled();
    let ort = std::env::var_os("ORT_DYLIB_PATH")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| auto_ort_dylib(dir));
    let ort_found = ort.is_some();
    let (pin_ok_count, pin_total, pins_ok) = match crate::manifest::verify_models_dir(dir, false) {
        Ok(checks) if checks.is_empty() => (0, 0, false),
        Ok(checks) => {
            let ok = checks.iter().filter(|c| c.is_ok()).count() as u32;
            let n = checks.len() as u32;
            (ok, n, ok == n && n > 0)
        }
        Err(_) => (0, 0, false),
    };
    let ready = feature && models_present;
    let hint = if ready && ort_found {
        "onnx ready — process --engine onnx|auto".into()
    } else if ready && !ort_found {
        "models present; set ORT_DYLIB_PATH or models/ort/ for load-dynamic ORT".into()
    } else if !feature && models_present {
        "models on disk; rebuild CLI with --features onnx".into()
    } else if feature && !models_present {
        "feature ON; run tools/fetch-models.ps1 (or .sh) then rradar models verify".into()
    } else {
        "mock default — fetch models + --features onnx for real OCR (models/README.md)".into()
    };
    OnnxReadiness {
        feature_enabled: feature,
        models_dir: dir.display().to_string(),
        det_present: det,
        rec_present: rec,
        cls_present: cls,
        models_present,
        pins_ok,
        pin_ok_count,
        pin_total,
        ort_found,
        ort_path: ort.map(|p| p.display().to_string()),
        ready_for_inference: ready,
        hint,
    }
}

/// ONNX engine. With feature `onnx` and models on disk, runs RapidOCR inference.
#[derive(Debug)]
pub struct OnnxOcrEngine {
    pub config: OnnxConfig,
    /// When false, only validate paths / feature (CI without weights).
    pub inference_enabled: bool,
    #[cfg(feature = "onnx")]
    runtime: std::sync::Mutex<Option<paddle_ocr_rs::ocr_lite::OcrLite>>,
}

impl Clone for OnnxOcrEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            inference_enabled: self.inference_enabled,
            #[cfg(feature = "onnx")]
            runtime: std::sync::Mutex::new(None),
        }
    }
}

impl OnnxOcrEngine {
    pub fn new(config: OnnxConfig) -> Result<Self, OcrError> {
        config.validate_paths()?;
        Ok(Self {
            config,
            inference_enabled: true,
            #[cfg(feature = "onnx")]
            runtime: std::sync::Mutex::new(None),
        })
    }

    /// Construct without requiring files (unit tests / feature probe).
    pub fn unvalidated(config: OnnxConfig) -> Self {
        Self {
            config,
            inference_enabled: false,
            #[cfg(feature = "onnx")]
            runtime: std::sync::Mutex::new(None),
        }
    }

    #[cfg(feature = "onnx")]
    fn ensure_runtime(&self) -> Result<(), OcrError> {
        ensure_ort_dylib_env(&self.config.models_dir());
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| OcrError::Backend("onnx runtime mutex poisoned".into()))?;
        if guard.is_some() {
            return Ok(());
        }
        self.config.validate_paths()?;
        let mut ocr = paddle_ocr_rs::ocr_lite::OcrLite::new();
        let det = self.config.det_model.to_string_lossy();
        let cls = self.config.cls_model.to_string_lossy();
        let rec = self.config.rec_model.to_string_lossy();
        let threads = self.config.num_threads.max(1);
        let init = if let Some(keys) = &self.config.keys_path {
            if keys.is_file() {
                ocr.init_models_with_dict(
                    det.as_ref(),
                    cls.as_ref(),
                    rec.as_ref(),
                    keys.to_string_lossy().as_ref(),
                    threads,
                )
            } else {
                ocr.init_models(det.as_ref(), cls.as_ref(), rec.as_ref(), threads)
            }
        } else {
            ocr.init_models(det.as_ref(), cls.as_ref(), rec.as_ref(), threads)
        };
        init.map_err(|e| {
            OcrError::Backend(format!(
                "failed to init ONNX models (is ORT_DYLIB_PATH set? models corrupt?): {e}"
            ))
        })?;
        *guard = Some(ocr);
        Ok(())
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
            return Err(hint_unavailable(&self.config));
        }
        self.config.validate_paths()?;

        #[cfg(not(feature = "onnx"))]
        {
            let _ = image_bytes;
            Err(hint_unavailable(&self.config))
        }

        #[cfg(feature = "onnx")]
        {
            self.ensure_runtime()?;
            let img = image::load_from_memory(image_bytes)
                .map_err(|e| OcrError::Backend(format!("image decode failed: {e}")))?
                .to_rgb8();

            let mut guard = self
                .runtime
                .lock()
                .map_err(|_| OcrError::Backend("onnx runtime mutex poisoned".into()))?;
            let ocr = guard
                .as_mut()
                .ok_or_else(|| OcrError::Backend("onnx runtime not initialized".into()))?;

            // RapidOCR defaults used by paddle-ocr-rs examples.
            let result = ocr
                .detect(
                    &img, 50,    // padding
                    1024,  // max_side_len
                    0.5,   // box_score_thresh
                    0.3,   // box_thresh
                    1.6,   // un_clip_ratio
                    true,  // do_angle
                    false, // most_angle
                )
                .map_err(|e| OcrError::Backend(format!("onnx detect failed: {e}")))?;

            let lines: Vec<OcrLine> = result
                .text_blocks
                .into_iter()
                .filter(|b| !b.text.trim().is_empty())
                .map(|b| OcrLine {
                    text: b.text,
                    confidence: b.text_score.clamp(0.0, 1.0),
                })
                .collect();

            if lines.is_empty() {
                return Err(OcrError::Backend(
                    "onnx produced no text lines (blank image or det threshold)".into(),
                ));
            }
            // Detector order is kept as-is (usually top-to-bottom).
            Ok(lines)
        }
    }
}

fn hint_unavailable(cfg: &OnnxConfig) -> OcrError {
    let mut parts = vec![
        "ONNX OCR is not ready.".to_string(),
        if onnx_feature_enabled() {
            "Binary has --features onnx.".into()
        } else {
            "Rebuild: cargo build -p rradar-cli --features onnx".into()
        },
    ];
    if let Err(e) = cfg.validate_paths() {
        parts.push(e.to_string());
    } else {
        parts.push("Models present.".into());
        if std::env::var_os("ORT_DYLIB_PATH").is_none()
            && auto_ort_dylib(&cfg.models_dir()).is_none()
        {
            parts.push(
                "Set ORT_DYLIB_PATH to onnxruntime shared library, or place it under models/ort/."
                    .into(),
            );
        }
        if onnx_feature_enabled() {
            parts.push(
                "Call failed before inference enable — use OnnxOcrEngine::new after models exist."
                    .into(),
            );
        }
    }
    parts.push("Docs: models/README.md".into());
    OcrError::OnnxUnavailableWithHint(parts.join(" "))
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
        let err = eng.recognize(b"x").unwrap_err();
        match err {
            OcrError::OnnxUnavailable | OcrError::OnnxUnavailableWithHint(_) => {}
            OcrError::Backend(_) => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn status_mentions_feature() {
        let cfg = OnnxConfig::from_models_dir("models");
        let s = cfg.status_lines().join("\n");
        assert!(s.contains("onnx feature"));
    }
}
