//! Hash-pinned ONNX model manifest (`models/manifest.sha256`).
//!
//! Format (one pin per line, comments allowed):
//! ```text
//! # comment
//! <64-hex-sha256>  <filename>
//! ```

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// One expected artifact from the pin file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPin {
    pub sha256_hex: String,
    pub filename: String,
}

/// Result of checking one pin against the models directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinCheck {
    Ok {
        pin: ModelPin,
        bytes: u64,
    },
    Missing {
        pin: ModelPin,
    },
    Mismatch {
        pin: ModelPin,
        actual_hex: String,
        bytes: u64,
    },
}

impl PinCheck {
    pub fn is_ok(&self) -> bool {
        matches!(self, PinCheck::Ok { .. })
    }

    pub fn summary_line(&self) -> String {
        match self {
            PinCheck::Ok { pin, bytes } => {
                format!("  pin ok     | {} ({} bytes)", pin.filename, bytes)
            }
            PinCheck::Missing { pin } => {
                format!(
                    "  pin MISS   | {} (expected {})",
                    pin.filename,
                    &pin.sha256_hex[..12]
                )
            }
            PinCheck::Mismatch {
                pin,
                actual_hex,
                bytes,
            } => format!(
                "  pin BAD    | {} want {}… got {}… ({} bytes)",
                pin.filename,
                &pin.sha256_hex[..12],
                &actual_hex[..12.min(actual_hex.len())],
                bytes
            ),
        }
    }
}

/// Parse `manifest.sha256` content (not a path).
pub fn parse_manifest(text: &str) -> Result<Vec<ModelPin>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let hash = parts
            .next()
            .ok_or_else(|| format!("manifest line {}: missing hash", i + 1))?;
        let name = parts
            .next()
            .ok_or_else(|| format!("manifest line {}: missing filename", i + 1))?;
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "manifest line {}: invalid sha256 (len={})",
                i + 1,
                hash.len()
            ));
        }
        if parts.next().is_some() {
            return Err(format!(
                "manifest line {}: extra tokens (use one filename)",
                i + 1
            ));
        }
        out.push(ModelPin {
            sha256_hex: hash.to_ascii_lowercase(),
            filename: name.to_string(),
        });
    }
    Ok(out)
}

/// Load pins from `models_dir/manifest.sha256`.
pub fn load_pins(models_dir: impl AsRef<Path>) -> Result<Vec<ModelPin>, String> {
    let path = models_dir.as_ref().join("manifest.sha256");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_manifest(&text)
}

/// SHA-256 hex (lowercase) of a file.
pub fn file_sha256_hex(path: impl AsRef<Path>) -> Result<String, String> {
    let bytes = fs::read(path.as_ref()).map_err(|e| e.to_string())?;
    let digest = Sha256::digest(&bytes);
    Ok(hex::encode(digest))
}

/// Verify every pin under `models_dir`. Empty pin list is an error if require_pins.
pub fn verify_models_dir(
    models_dir: impl AsRef<Path>,
    require_pins: bool,
) -> Result<Vec<PinCheck>, String> {
    let dir = models_dir.as_ref();
    let pins = match load_pins(dir) {
        Ok(p) if p.is_empty() && require_pins => {
            return Err("manifest.sha256 has no pin lines".into());
        }
        Ok(p) => p,
        Err(e) if require_pins => return Err(e),
        Err(_) => return Ok(vec![]),
    };
    let mut out = Vec::with_capacity(pins.len());
    for pin in pins {
        let path = dir.join(&pin.filename);
        if !path.is_file() {
            out.push(PinCheck::Missing { pin });
            continue;
        }
        let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
        let actual = file_sha256_hex(&path)?;
        if actual == pin.sha256_hex {
            out.push(PinCheck::Ok {
                pin,
                bytes: meta.len(),
            });
        } else {
            out.push(PinCheck::Mismatch {
                pin,
                actual_hex: actual,
                bytes: meta.len(),
            });
        }
    }
    Ok(out)
}

/// All pins OK?
pub fn all_pins_ok(checks: &[PinCheck]) -> bool {
    !checks.is_empty() && checks.iter().all(PinCheck::is_ok)
}

/// Default models dir: `RRADAR_MODELS_DIR` or `./models`.
pub fn default_models_dir() -> PathBuf {
    std::env::var("RRADAR_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("models"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pins_and_comments() {
        let text = r#"
# comment
d2a7720d45a54257208b1e13e36a8479894cb74155a5efe29462512d42f49da9  ch_PP-OCRv4_det_infer.onnx
48fc40f24f6d2a207a2b1091d3437eb3cc3eb6b676dc3ef9c37384005483683b  ch_PP-OCRv4_rec_infer.onnx
"#;
        let pins = parse_manifest(text).unwrap();
        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].filename, "ch_PP-OCRv4_det_infer.onnx");
        assert_eq!(pins[0].sha256_hex.len(), 64);
    }

    #[test]
    fn parse_rejects_short_hash() {
        assert!(parse_manifest("abc  file.onnx\n").is_err());
    }

    #[test]
    fn verify_repo_manifest_when_models_present() {
        // Workspace layout: crates/rradar-ocr → ../../models
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../models");
        let manifest = root.join("manifest.sha256");
        if !manifest.is_file() {
            return;
        }
        let checks = verify_models_dir(&root, true).expect("load pins");
        assert!(!checks.is_empty(), "expected pins in committed manifest");
        // Weights are gitignored; missing is OK, mismatch is not when file exists.
        for c in &checks {
            if let PinCheck::Mismatch { pin, .. } = c {
                panic!("hash mismatch for {}", pin.filename);
            }
        }
    }

    #[test]
    fn verify_roundtrip_temp_file() {
        let dir = std::env::temp_dir().join(format!("rradar-pin-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let payload = b"hello-rradar-model-pin";
        let name = "toy.onnx";
        fs::write(dir.join(name), payload).unwrap();
        let hash = hex::encode(Sha256::digest(payload));
        fs::write(dir.join("manifest.sha256"), format!("{hash}  {name}\n")).unwrap();
        let checks = verify_models_dir(&dir, true).unwrap();
        assert!(all_pins_ok(&checks));
        // corrupt
        fs::write(dir.join(name), b"tampered").unwrap();
        let checks = verify_models_dir(&dir, true).unwrap();
        assert!(!all_pins_ok(&checks));
        let _ = fs::remove_dir_all(&dir);
    }
}
