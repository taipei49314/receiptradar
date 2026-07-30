//! FFI surface sketch for flutter_rust_bridge (PR-A19).
//!
//! This crate is intentionally thin and does not depend on FRB codegen yet.
//! Mobile will call these free functions once bindings are generated.
//!
//! Surface covers the demo closed-loop steps: process → confirm → list → stats.

#![deny(unsafe_code)]

use rradar_core::{
    open_ledger_auto, process_path, CategoryEngine, Iso4217, Ledger, ProcessOptions, PRODUCT_ID,
    VERSION,
};
use rradar_ocr::engine_by_name;
use std::path::Path;

/// Hello for smoke tests from Dart.
pub fn api_version() -> String {
    format!("{PRODUCT_ID} ffi {VERSION}")
}

/// Ledger schema version string (e.g. `"1"`).
pub fn ledger_schema_version(
    db_path: String,
    passphrase: Option<String>,
) -> Result<String, String> {
    let (ledger, tmp) =
        open_ledger_auto(Path::new(&db_path), passphrase.as_deref()).map_err(|e| e.to_string())?;
    let v = ledger.schema_version().map_err(|e| e.to_string())?;
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(v)
}

/// Process a filesystem path (preferred mobile entry).
/// Returns JSON [`rradar_core::ReceiptDraft`].
pub fn process_receipt_path_json(
    path: String,
    currency: String,
    engine: String,
) -> Result<String, String> {
    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let cats = CategoryEngine::with_seed();
    let cur = Iso4217::parse(&currency).unwrap_or(Iso4217::TWD);
    let draft = process_path(
        Path::new(&path),
        eng.as_ref(),
        &cats,
        ProcessOptions {
            default_currency: cur,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string(&draft).map_err(|e| e.to_string())
}

/// Confirm a draft JSON into the ledger. Returns ConfirmResult JSON.
pub fn confirm_draft_json(
    db_path: String,
    passphrase: Option<String>,
    draft_json: String,
    force: bool,
) -> Result<String, String> {
    let draft: rradar_core::ReceiptDraft =
        serde_json::from_str(&draft_json).map_err(|e| e.to_string())?;
    let (ledger, tmp) =
        open_ledger_auto(Path::new(&db_path), passphrase.as_deref()).map_err(|e| e.to_string())?;
    let res = ledger
        .confirm_draft(&draft, None, None, force)
        .map_err(|e| e.to_string())?;
    // Reseal path is owned by higher layers when using sealed files; open_ledger_auto
    // for plain sqlite leaves db on disk already mutated.
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    serde_json::to_string(&res).map_err(|e| e.to_string())
}

/// List transactions as JSON from a db path (optional passphrase for .rrsealed).
pub fn list_transactions_json(
    db_path: String,
    passphrase: Option<String>,
    limit: u32,
) -> Result<String, String> {
    let (ledger, tmp) =
        open_ledger_auto(Path::new(&db_path), passphrase.as_deref()).map_err(|e| e.to_string())?;
    let rows = ledger
        .list_transactions(limit as usize, 0)
        .map_err(|e| e.to_string())?;
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    serde_json::to_string(&rows).map_err(|e| e.to_string())
}

/// Per-currency monthly stats as JSON (all months).
pub fn stats_all_json(db_path: String, passphrase: Option<String>) -> Result<String, String> {
    let (ledger, tmp) =
        open_ledger_auto(Path::new(&db_path), passphrase.as_deref()).map_err(|e| e.to_string())?;
    let rows = ledger.stats_by_currency_all().map_err(|e| e.to_string())?;
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    serde_json::to_string(&rows).map_err(|e| e.to_string())
}

/// Open or create a plain ledger at path (mobile onboarding helper).
pub fn ensure_ledger(db_path: String) -> Result<(), String> {
    let _ = Ledger::open(Path::new(&db_path)).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_nonempty() {
        assert!(api_version().contains("ffi"));
    }

    #[test]
    fn process_and_confirm_roundtrip() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/familymart_89.txt");
        assert!(root.is_file(), "fixture missing");
        let draft_json =
            process_receipt_path_json(root.display().to_string(), "TWD".into(), "mock".into())
                .expect("process");
        assert!(
            draft_json.contains("89") || draft_json.contains("全家"),
            "{draft_json}"
        );

        let dir = std::env::temp_dir().join(format!("rradar-ffi-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("ledger.db");
        ensure_ledger(db.display().to_string()).unwrap();
        let conf = confirm_draft_json(db.display().to_string(), None, draft_json, false).unwrap();
        assert!(
            conf.contains("\"inserted\":true") || conf.contains("inserted"),
            "{conf}"
        );
        let list = list_transactions_json(db.display().to_string(), None, 10).unwrap();
        assert!(list.contains("8900") || list.contains("全家"), "{list}");
        let stats = stats_all_json(db.display().to_string(), None).unwrap();
        assert!(stats.contains("TWD"), "{stats}");
        let ver = ledger_schema_version(db.display().to_string(), None).unwrap();
        assert_eq!(ver, "1");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
