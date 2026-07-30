//! Lightweight image preprocess placeholders (bytes in / bytes out).
//!
//! Full decode + resize lands with real image crates post–OCR spike.
//! Here we track adaptive max-edge *intent* and content hashing.

use sha2::{Digest, Sha256};

/// Preprocess options (design: start 1280, retry 1600 on low conf).
#[derive(Debug, Clone, Copy)]
pub struct PreprocessConfig {
    pub max_edge: u32,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self { max_edge: 1280 }
    }
}

impl PreprocessConfig {
    pub fn retry_higher(self) -> Self {
        Self {
            max_edge: self.max_edge.max(1600),
        }
    }
}

/// Result of preprocess pass — for mock path, passthrough bytes.
#[derive(Debug, Clone)]
pub struct Preprocessed {
    pub bytes: Vec<u8>,
    pub max_edge: u32,
    pub content_hash_hex: String,
}

pub fn content_hash(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// v0.1 stub: no decode; record hash + intended max edge for explain.
pub fn preprocess(bytes: &[u8], cfg: PreprocessConfig) -> Preprocessed {
    Preprocessed {
        bytes: bytes.to_vec(),
        max_edge: cfg.max_edge,
        content_hash_hex: content_hash(bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stable() {
        let a = content_hash(b"abc");
        let b = content_hash(b"abc");
        assert_eq!(a, b);
        assert_ne!(content_hash(b"abc"), content_hash(b"abd"));
    }

    #[test]
    fn preprocess_passthrough() {
        let p = preprocess(b"jpeg-bytes", PreprocessConfig::default());
        assert_eq!(p.bytes, b"jpeg-bytes");
        assert_eq!(p.max_edge, 1280);
    }
}
