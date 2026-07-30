//! FFI surface sketch for flutter_rust_bridge (PR-A19).
//!
//! This crate is intentionally thin and does not depend on FRB codegen yet.
//! Mobile will call these free functions once bindings are generated.

#![deny(unsafe_code)]

use rradar_core::{
    open_ledger_auto, process_path, CategoryEngine, Iso4217, ProcessOptions, PRODUCT_ID, VERSION,
};
use rradar_ocr::engine_by_name;
use std::path::Path;

/// Hello for smoke tests from Dart.
pub fn api_version() -> String {
    format!("{PRODUCT_ID} ffi {VERSION}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_nonempty() {
        assert!(api_version().contains("ffi"));
    }
}
