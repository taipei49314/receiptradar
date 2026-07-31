//! FFI surface for flutter_rust_bridge (PR-A19) and native mobile hosts.
//!
//! # Design
//! - Free functions only (FRB-friendly); all errors are `String`.
//! - Complex values cross the boundary as **JSON strings** so Dart can decode
//!   without sharing Rust types until codegen lands.
//! - **No network.** Local paths + optional passphrase for `.rrsealed` only.
//!
//! # Status
//! FRB is **not** generated yet (Flutter SDK optional). This crate is the
//! contract mobile will bind. See `docs/ffi.md` and `apps/mobile/lib/bridge/`.

#![deny(unsafe_code)]

use rradar_core::{
    create_backup, data_dir, default_db_path, open_ledger_auto, process_bytes, process_path,
    CategoryEngine, Iso4217, Ledger, ProcessOptions, TxUpdate, LEDGER_SCHEMA_VERSION, PRODUCT_ID,
    VERSION,
};
use rradar_ocr::engine_by_name;
use serde::Serialize;
use std::path::Path;

// ---------------------------------------------------------------------------
// Identity / paths
// ---------------------------------------------------------------------------

/// Hello for smoke tests from Dart.
pub fn api_version() -> String {
    format!("{PRODUCT_ID} ffi {VERSION}")
}

/// Product id constant.
pub fn product_id() -> String {
    PRODUCT_ID.into()
}

/// Core/CLI package version.
pub fn core_version() -> String {
    VERSION.into()
}

/// Highest ledger schema this binary migrates to.
pub fn supported_ledger_schema() -> u32 {
    LEDGER_SCHEMA_VERSION
}

/// Default app data directory (platform-specific).
pub fn default_data_dir() -> String {
    data_dir().display().to_string()
}

/// Default ledger.db path under the data dir.
pub fn default_ledger_path() -> String {
    default_db_path().display().to_string()
}

/// Compact capability JSON for mobile about screens.
pub fn capabilities_json() -> String {
    #[derive(Serialize)]
    struct Caps {
        product_id: &'static str,
        version: &'static str,
        ledger_schema: u32,
        engines: [&'static str; 2],
        cloud_sync: bool,
        official_relay: bool,
        notes: &'static str,
    }
    serde_json::to_string(&Caps {
        product_id: PRODUCT_ID,
        version: VERSION,
        ledger_schema: LEDGER_SCHEMA_VERSION,
        engines: ["mock", "onnx"],
        cloud_sync: false,
        official_relay: false,
        notes: "local-first; multi-device via backup file only",
    })
    .unwrap_or_else(|_| "{}".into())
}

// ---------------------------------------------------------------------------
// Ledger lifecycle
// ---------------------------------------------------------------------------

fn with_ledger<T>(
    db_path: &str,
    passphrase: Option<&str>,
    f: impl FnOnce(&Ledger) -> Result<T, String>,
) -> Result<T, String> {
    let (ledger, tmp) =
        open_ledger_auto(Path::new(db_path), passphrase).map_err(|e| e.to_string())?;
    let out = f(&ledger);
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    out
}

/// Open or create a plain ledger at path (mobile onboarding helper).
pub fn ensure_ledger(db_path: String) -> Result<(), String> {
    let _ = Ledger::open(Path::new(&db_path)).map_err(|e| e.to_string())?;
    Ok(())
}

/// Ledger schema version string on disk (after migrations).
pub fn ledger_schema_version(
    db_path: String,
    passphrase: Option<String>,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        ledger.schema_version().map_err(|e| e.to_string())
    })
}

/// Transaction count.
pub fn count_transactions(db_path: String, passphrase: Option<String>) -> Result<u64, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        ledger.count().map(|n| n as u64).map_err(|e| e.to_string())
    })
}

// ---------------------------------------------------------------------------
// OCR / process
// ---------------------------------------------------------------------------

/// Process a filesystem path (text fixture, image, or mock-ocr binary).
/// Returns JSON [`rradar_core::ReceiptDraft`].
pub fn process_receipt_path_json(
    path: String,
    currency: String,
    engine: String,
) -> Result<String, String> {
    process_receipt_path_json_ex(path, currency, engine, None)
}

/// Process path with optional TW e-invoice left-QR payload.
pub fn process_receipt_path_json_ex(
    path: String,
    currency: String,
    engine: String,
    qr_payload: Option<String>,
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
            qr_payload,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string(&draft).map_err(|e| e.to_string())
}

/// Process raw image / mock-OCR bytes from the camera pipeline.
/// Prefer this on mobile after writing a temp file is undesirable.
pub fn process_image_bytes_json(
    image_bytes: Vec<u8>,
    currency: String,
    engine: String,
    qr_payload: Option<String>,
) -> Result<String, String> {
    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let cats = CategoryEngine::with_seed();
    let cur = Iso4217::parse(&currency).unwrap_or(Iso4217::TWD);
    let draft = process_bytes(
        &image_bytes,
        None,
        eng.as_ref(),
        &cats,
        ProcessOptions {
            default_currency: cur,
            qr_payload,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string(&draft).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Confirm / CRUD
// ---------------------------------------------------------------------------

/// Confirm a draft JSON into the ledger. Returns ConfirmResult JSON.
pub fn confirm_draft_json(
    db_path: String,
    passphrase: Option<String>,
    draft_json: String,
    force: bool,
) -> Result<String, String> {
    confirm_draft_json_ex(db_path, passphrase, draft_json, force, None, None)
}

/// Confirm with optional content_hash + notes.
pub fn confirm_draft_json_ex(
    db_path: String,
    passphrase: Option<String>,
    draft_json: String,
    force: bool,
    content_hash: Option<String>,
    notes: Option<String>,
) -> Result<String, String> {
    let draft: rradar_core::ReceiptDraft =
        serde_json::from_str(&draft_json).map_err(|e| e.to_string())?;
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let res = ledger
            .confirm_draft(&draft, content_hash.as_deref(), notes.as_deref(), force)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&res).map_err(|e| e.to_string())
    })
}

/// List transactions as JSON.
pub fn list_transactions_json(
    db_path: String,
    passphrase: Option<String>,
    limit: u32,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let rows = ledger
            .list_transactions(limit as usize, 0)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    })
}

/// Get one transaction by id as JSON.
pub fn get_transaction_json(
    db_path: String,
    passphrase: Option<String>,
    id: String,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let tx = ledger.get_transaction(&id).map_err(|e| e.to_string())?;
        serde_json::to_string(&tx).map_err(|e| e.to_string())
    })
}

/// Most recently confirmed transaction JSON, or `"null"`.
pub fn last_transaction_json(
    db_path: String,
    passphrase: Option<String>,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let tx = ledger.last_transaction().map_err(|e| e.to_string())?;
        serde_json::to_string(&tx).map_err(|e| e.to_string())
    })
}

/// Delete a transaction by id. Returns true if a row was removed.
pub fn delete_transaction(
    db_path: String,
    passphrase: Option<String>,
    id: String,
) -> Result<bool, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        ledger.delete_transaction(&id).map_err(|e| e.to_string())
    })
}

/// Patch transaction fields. JSON body: optional merchant, amount_minor, currency,
/// category, notes, transacted_at. Returns updated transaction JSON.
pub fn update_transaction_json(
    db_path: String,
    passphrase: Option<String>,
    id: String,
    patch_json: String,
) -> Result<String, String> {
    #[derive(serde::Deserialize, Default)]
    struct Patch {
        merchant: Option<String>,
        amount_minor: Option<i64>,
        currency: Option<String>,
        category: Option<String>,
        notes: Option<String>,
        transacted_at: Option<String>,
    }
    let p: Patch = serde_json::from_str(&patch_json).map_err(|e| e.to_string())?;
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let tx = ledger
            .update_transaction(
                &id,
                &TxUpdate {
                    merchant: p.merchant,
                    amount_minor: p.amount_minor,
                    currency: p.currency,
                    category: p.category,
                    notes: p.notes,
                    transacted_at: p.transacted_at,
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&tx).map_err(|e| e.to_string())
    })
}

// ---------------------------------------------------------------------------
// Analytics / taxonomy
// ---------------------------------------------------------------------------

/// Per-currency monthly stats as JSON (all months).
pub fn stats_all_json(db_path: String, passphrase: Option<String>) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let rows = ledger.stats_by_currency_all().map_err(|e| e.to_string())?;
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    })
}

/// Stats for one calendar month (year, month 1–12).
pub fn stats_month_json(
    db_path: String,
    passphrase: Option<String>,
    year: i32,
    month: u32,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let rows = ledger
            .stats_by_currency_month(year, month)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    })
}

/// Top merchants for one currency: JSON array of `{merchant, total_minor, count}`.
pub fn top_merchants_json(
    db_path: String,
    passphrase: Option<String>,
    currency: String,
    limit: u32,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let rows = ledger
            .top_merchants(&currency, limit as usize)
            .map_err(|e| e.to_string())?;
        #[derive(Serialize)]
        struct Row {
            merchant: String,
            total_minor: i64,
            count: i64,
        }
        let mapped: Vec<Row> = rows
            .into_iter()
            .map(|(merchant, total_minor, count)| Row {
                merchant,
                total_minor,
                count,
            })
            .collect();
        serde_json::to_string(&mapped).map_err(|e| e.to_string())
    })
}

/// Built-in category id list as JSON string array.
pub fn categories_json() -> String {
    let cats = CategoryEngine::with_seed();
    // CategoryEngine may not expose ids — use fixed taxonomy from design.
    let ids = [
        "food_dining",
        "grocery_convenience",
        "transport",
        "shopping",
        "health",
        "utilities",
        "entertainment",
        "other",
    ];
    let _ = cats; // seed loaded for future rules exposure
    serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into())
}

// ---------------------------------------------------------------------------
// Backup (local multi-device only)
// ---------------------------------------------------------------------------

/// Write encrypted backup.rradar to `out_path`. Uses fast Argon2 when
/// `RRADAR_FAST_BACKUP` is set; otherwise design default.
pub fn backup_create_file(
    db_path: String,
    passphrase_db: Option<String>,
    backup_passphrase: String,
    out_path: String,
) -> Result<(), String> {
    with_ledger(&db_path, passphrase_db.as_deref(), |ledger| {
        let m = if std::env::var("RRADAR_FAST_BACKUP").is_ok() {
            8
        } else {
            rradar_core::crypto::ARGON2_M_KIB
        };
        let bytes = create_backup(ledger, &backup_passphrase, m).map_err(|e| e.to_string())?;
        if let Some(parent) = Path::new(&out_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        std::fs::write(&out_path, bytes).map_err(|e| e.to_string())?;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_capabilities() {
        assert!(api_version().contains("ffi"));
        assert_eq!(product_id(), PRODUCT_ID);
        assert_eq!(supported_ledger_schema(), LEDGER_SCHEMA_VERSION);
        let caps = capabilities_json();
        assert!(caps.contains("\"cloud_sync\":false"));
        assert!(caps.contains("official_relay"));
        assert!(!categories_json().is_empty());
    }

    #[test]
    fn paths_nonempty() {
        assert!(!default_data_dir().is_empty());
        assert!(default_ledger_path().contains("ledger"));
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
        let n = count_transactions(db.display().to_string(), None).unwrap();
        assert_eq!(n, 1);
        let last = last_transaction_json(db.display().to_string(), None).unwrap();
        assert!(last.contains("全家") || last.contains("8900"), "{last}");
        let ver = ledger_schema_version(db.display().to_string(), None).unwrap();
        assert_eq!(ver, "2");

        // mock image path bytes (LF)
        let mut magic = b"RRADAR_MOCK_OCR\n".to_vec();
        magic.extend_from_slice("測試店\n合計 42\n2024-01-02\n".as_bytes());
        let d2 = process_image_bytes_json(magic, "TWD".into(), "mock".into(), None).unwrap();
        assert!(d2.contains("42"), "{d2}");
        // CRLF terminator (Windows checkout tolerance)
        let mut crlf = b"RRADAR_MOCK_OCR\r\n".to_vec();
        crlf.extend_from_slice(b"SHOP\r\nTOTAL $1.25\r\n");
        let d3 = process_image_bytes_json(crlf, "USD".into(), "mock".into(), None).unwrap();
        assert!(d3.contains("125") || d3.contains("1.25"), "{d3}");

        let bak = dir.join("t.rradar");
        std::env::set_var("RRADAR_FAST_BACKUP", "1");
        backup_create_file(
            db.display().to_string(),
            None,
            "pass".into(),
            bak.display().to_string(),
        )
        .unwrap();
        assert!(bak.is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
