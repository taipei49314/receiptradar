//! Golden fixture runner (metric a: text → fields).

use rradar_core::{process_path, CategoryEngine, Iso4217, ProcessOptions, SourcePath};
use rradar_ocr::MockOcrEngine;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Manifest {
    text_fixtures: Vec<TextFx>,
    #[serde(default)]
    mock_ocr_fixtures: Vec<TextFx>,
    qr_fixtures: Vec<QrFx>,
}

#[derive(Debug, Deserialize)]
struct TextFx {
    path: String,
    expect_total_minor: i64,
    expect_currency: String,
}

#[derive(Debug, Deserialize)]
struct QrFx {
    path: String,
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

#[test]
fn golden_text_fixtures() {
    let root = fixtures_root();
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(root.join("manifest.json")).expect("manifest"),
    )
    .expect("parse manifest");
    let eng = MockOcrEngine;
    let cats = CategoryEngine::with_seed();

    for fx in manifest.text_fixtures {
        let path = root.join(&fx.path);
        let currency = Iso4217::parse(&fx.expect_currency).expect("currency");
        let draft = process_path(
            &path,
            &eng,
            &cats,
            ProcessOptions {
                default_currency: currency,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{}: {e}", fx.path));
        assert_eq!(
            draft.total.value.amount_minor, fx.expect_total_minor,
            "{} total",
            fx.path
        );
        assert_eq!(
            draft.total.value.currency.to_string(),
            fx.expect_currency,
            "{} currency",
            fx.path
        );
        assert_eq!(draft.source_path, SourcePath::Ocr);
    }
}

#[test]
fn golden_mock_ocr_binaries() {
    let root = fixtures_root();
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(root.join("manifest.json")).expect("manifest"),
    )
    .expect("parse manifest");
    let eng = MockOcrEngine;
    let cats = CategoryEngine::with_seed();

    for fx in manifest.mock_ocr_fixtures {
        let path = root.join(&fx.path);
        let currency = Iso4217::parse(&fx.expect_currency).expect("currency");
        let draft = process_path(
            &path,
            &eng,
            &cats,
            ProcessOptions {
                default_currency: currency,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{}: {e}", fx.path));
        assert_eq!(
            draft.total.value.amount_minor, fx.expect_total_minor,
            "{} total",
            fx.path
        );
        assert_eq!(
            draft.total.value.currency.to_string(),
            fx.expect_currency,
            "{} currency",
            fx.path
        );
    }
}

#[test]
fn golden_qr_payloads_parse() {
    let root = fixtures_root();
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(root.join("manifest.json")).expect("manifest"),
    )
    .expect("parse");
    let eng = MockOcrEngine;
    let cats = CategoryEngine::with_seed();

    for fx in manifest.qr_fixtures {
        let path = root.join(&fx.path);
        let payload = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}", fx.path))
            .trim()
            .to_string();
        let draft = process_path(
            &path,
            &eng,
            &cats,
            ProcessOptions {
                qr_payload: Some(payload),
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{}: {e}", fx.path));
        assert_eq!(draft.source_path, SourcePath::Qr, "{}", fx.path);
        assert!(draft.total.value.amount_minor > 0, "{}", fx.path);
        assert!(draft.invoice_id.is_some(), "{}", fx.path);
    }
}
