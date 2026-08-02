//! Generate synthetic receipt PNGs for `fixtures/images/`.
//!
//! Usage (from repo root):
//!   cargo run -p gen-receipt-png -- fixtures/images
//!
//! Draws white paper + black 5×7 bitmap glyphs (ASCII). No personal data.
//! Sidecar `.ocr.txt` files are maintained separately for CI mock path.

use image::{Rgb, RgbImage};
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let out = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("fixtures/images"));
    std::fs::create_dir_all(&out).expect("mkdir");

    // Specs: (filename, lines, width, height, scale)
    let specs: &[(&str, &[&str], u32, u32, u32)] = &[
        (
            "receipt_en_total89.png",
            &[
                "FAMILYMART LINJIANG",
                "TOTAL 89",
                "2024-05-01",
                "AB12345678",
            ],
            480,
            640,
            3,
        ),
        (
            "familymart_photo.png",
            &[
                "FAMILYMART LINJIANG",
                "TOTAL 89",
                "2024-05-01",
                "AB12345678",
            ],
            480,
            640,
            3,
        ),
        (
            "seven_eleven_photo.png",
            &["7-ELEVEN XINYI", "TOTAL 45", "2024-08-12", "TX778899"],
            480,
            640,
            3,
        ),
        (
            "starbucks_photo.png",
            &["STARBUCKS COFFEE", "TOTAL $5.45", "2024-07-04"],
            420,
            560,
            3,
        ),
        (
            "mcdonalds_photo.png",
            &["MCDONALDS TAIPEI", "TOTAL 168", "2024-06-15"],
            420,
            560,
            3,
        ),
    ];

    for (name, lines, w, h, scale) in specs {
        let img = render_receipt(lines, *w, *h, *scale);
        let path = out.join(name);
        img.save(&path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        println!("wrote {} ({}x{})", path.display(), w, h);
    }
}

fn render_receipt(lines: &[&str], width: u32, height: u32, scale: u32) -> RgbImage {
    let mut img = RgbImage::from_pixel(width, height, Rgb([245, 245, 240]));
    // faint paper margin
    for x in 16..width.saturating_sub(16) {
        img.put_pixel(x, 24, Rgb([200, 200, 195]));
        img.put_pixel(x, height.saturating_sub(24), Rgb([200, 200, 195]));
    }
    let mut y = 48u32;
    let line_gap = 8u32 * scale + 12;
    for line in lines {
        draw_text(&mut img, 32, y, line, scale, Rgb([20, 20, 20]));
        y = y.saturating_add(line_gap);
    }
    // decorative "TOTAL" underline
    if let Some(total_line) = lines.iter().position(|l| l.contains("TOTAL")) {
        let uy = 48 + (total_line as u32 + 1) * line_gap - 6;
        for x in 32..width.saturating_sub(32) {
            if uy < height {
                img.put_pixel(x, uy, Rgb([80, 80, 80]));
            }
        }
    }
    img
}

fn draw_text(img: &mut RgbImage, x0: u32, y0: u32, text: &str, scale: u32, color: Rgb<u8>) {
    let mut x = x0;
    for ch in text.chars() {
        let glyph = glyph_for(ch);
        for row in 0..7u32 {
            let bits = glyph[row as usize];
            for col in 0..5u32 {
                if bits & (1 << (4 - col)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = x + col * scale + dx;
                            let py = y0 + row * scale + dy;
                            if px < img.width() && py < img.height() {
                                img.put_pixel(px, py, color);
                            }
                        }
                    }
                }
            }
        }
        x += 6 * scale; // 5px glyph + 1px space
    }
}

/// 5×7 packed rows (bit4 = left). Space / unknown → empty.
fn glyph_for(ch: char) -> [u8; 7] {
    let c = ch.to_ascii_uppercase();
    match c {
        ' ' => [0; 7],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100,
        ],
        '$' => [
            0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b00100, 0b01000, 0b10000, 0b10000,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        _ => [
            0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ], // ?
    }
}

#[allow(dead_code)]
fn _path_hint(_: &Path) {}
