//! Image preprocess for the OCR pipeline (design: max-edge 1280, retry 1600).
//!
//! Non-image payloads (text fixtures, mock OCR bins) pass through unchanged.

use sha2::{Digest, Sha256};

/// Preprocess options (design: start 1280, retry 1600 on low conf).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreprocessConfig {
    /// Longest edge after downscale (px). Upscale is never applied.
    pub max_edge: u32,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self { max_edge: 1280 }
    }
}

impl PreprocessConfig {
    /// Higher-resolution retry pass (design Orange/Green gate).
    pub fn retry_higher(self) -> Self {
        Self {
            max_edge: self.max_edge.max(1600),
        }
    }
}

/// Result of a preprocess pass.
#[derive(Debug, Clone)]
pub struct Preprocessed {
    /// Bytes fed to OCR (original or re-encoded PNG after resize).
    pub bytes: Vec<u8>,
    pub max_edge: u32,
    pub content_hash_hex: String,
    /// True when JPEG/PNG/etc. was decoded.
    pub decoded: bool,
    /// True when a downscale was applied.
    pub resized: bool,
    pub original_width: Option<u32>,
    pub original_height: Option<u32>,
    pub output_width: Option<u32>,
    pub output_height: Option<u32>,
}

pub fn content_hash(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Decode image when possible; downscale so longest edge ≤ `cfg.max_edge`.
///
/// Text / mock payloads that are not valid images are returned as-is (no panic).
pub fn preprocess(bytes: &[u8], cfg: PreprocessConfig) -> Preprocessed {
    let hash = content_hash(bytes);
    if bytes.is_empty() {
        return Preprocessed {
            bytes: Vec::new(),
            max_edge: cfg.max_edge,
            content_hash_hex: hash,
            decoded: false,
            resized: false,
            original_width: None,
            original_height: None,
            output_width: None,
            output_height: None,
        };
    }

    match try_resize_image(bytes, cfg.max_edge) {
        Ok(Some(r)) => Preprocessed {
            bytes: r.bytes,
            max_edge: cfg.max_edge,
            content_hash_hex: hash,
            decoded: true,
            resized: r.resized,
            original_width: Some(r.original_width),
            original_height: Some(r.original_height),
            output_width: Some(r.output_width),
            output_height: Some(r.output_height),
        },
        Ok(None) | Err(_) => Preprocessed {
            bytes: bytes.to_vec(),
            max_edge: cfg.max_edge,
            content_hash_hex: hash,
            decoded: false,
            resized: false,
            original_width: None,
            original_height: None,
            output_width: None,
            output_height: None,
        },
    }
}

struct ResizedImage {
    bytes: Vec<u8>,
    original_width: u32,
    original_height: u32,
    output_width: u32,
    output_height: u32,
    resized: bool,
}

/// Returns `Ok(None)` when bytes are not a supported image format.
fn try_resize_image(bytes: &[u8], max_edge: u32) -> Result<Option<ResizedImage>, String> {
    let img = match image::load_from_memory(bytes) {
        Ok(i) => i,
        Err(_) => return Ok(None),
    };
    let (ow, oh) = (img.width(), img.height());
    if ow == 0 || oh == 0 {
        return Ok(None);
    }
    let long = ow.max(oh);
    let (nw, nh, resized) = if long > max_edge && max_edge > 0 {
        let scale = max_edge as f64 / long as f64;
        let nw = ((ow as f64) * scale).round().max(1.0) as u32;
        let nh = ((oh as f64) * scale).round().max(1.0) as u32;
        (nw, nh, true)
    } else {
        (ow, oh, false)
    };

    let out_img = if resized {
        img.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    // Always re-encode as PNG so OCR backends get a stable container after resize.
    // If not resized and input was already small, still PNG — size is fine for receipts.
    let mut buf = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut buf);
        out_img
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
    }
    Ok(Some(ResizedImage {
        bytes: buf,
        original_width: ow,
        original_height: oh,
        output_width: nw,
        output_height: nh,
        resized,
    }))
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
    fn preprocess_passthrough_non_image() {
        let p = preprocess(b"jpeg-bytes-not-really", PreprocessConfig::default());
        assert_eq!(p.bytes, b"jpeg-bytes-not-really");
        assert!(!p.decoded);
        assert!(!p.resized);
        assert_eq!(p.max_edge, 1280);
    }

    #[test]
    fn preprocess_resizes_large_png() {
        // 2000x1000 solid RGB → must shrink to max_edge 1280 (long edge).
        let mut img = image::RgbImage::new(2000, 1000);
        for p in img.pixels_mut() {
            *p = image::Rgb([40, 40, 40]);
        }
        // Draw a bright stripe so it's a valid image with structure.
        for x in 0..2000 {
            img.put_pixel(x, 500, image::Rgb([220, 220, 220]));
        }
        let dyn_img = image::DynamicImage::ImageRgb8(img);
        let mut png = Vec::new();
        {
            let mut c = std::io::Cursor::new(&mut png);
            dyn_img
                .write_to(&mut c, image::ImageFormat::Png)
                .expect("encode");
        }
        let p = preprocess(&png, PreprocessConfig { max_edge: 1280 });
        assert!(p.decoded);
        assert!(p.resized);
        assert_eq!(p.original_width, Some(2000));
        assert_eq!(p.original_height, Some(1000));
        assert_eq!(p.output_width, Some(1280));
        assert_eq!(p.output_height, Some(640));
        // Output must still decode.
        let again = image::load_from_memory(&p.bytes).expect("redecode");
        assert_eq!(again.width(), 1280);
        assert_eq!(again.height(), 640);
    }

    #[test]
    fn preprocess_no_upscale() {
        let mut img = image::RgbImage::new(100, 50);
        for p in img.pixels_mut() {
            *p = image::Rgb([10, 20, 30]);
        }
        let dyn_img = image::DynamicImage::ImageRgb8(img);
        let mut png = Vec::new();
        {
            let mut c = std::io::Cursor::new(&mut png);
            dyn_img
                .write_to(&mut c, image::ImageFormat::Png)
                .expect("encode");
        }
        let p = preprocess(&png, PreprocessConfig { max_edge: 1280 });
        assert!(p.decoded);
        assert!(!p.resized);
        assert_eq!(p.output_width, Some(100));
        assert_eq!(p.output_height, Some(50));
    }

    #[test]
    fn retry_higher_raises_edge() {
        let c = PreprocessConfig { max_edge: 1280 }.retry_higher();
        assert_eq!(c.max_edge, 1600);
    }
}
