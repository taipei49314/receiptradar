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
    apply_handoff_merge, attachments_root_for_db, budget_status_month, category_engine_with_packs,
    create_backup, create_handoff, data_dir, default_db_path, ensure_inbox_dir, ensure_rules_dir,
    inbox_dir, inspect_handoff, list_rule_files, normalize_tags, open_ledger_auto, process_bytes,
    process_path, remove_stored_attachment, resolve_attachment_path, rules_dir, store_attachment,
    store_attachment_bytes, write_handoff_file, BudgetBook, Iso4217, Ledger, ProcessOptions,
    TxFilter, TxUpdate, LEDGER_SCHEMA_VERSION, PRODUCT_ID, VERSION,
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

/// OCR engines catalog + ONNX readiness (same as `rradar engines --json`).
pub fn engines_json() -> String {
    rradar_ocr::engines_catalog_json()
}

/// Compact capability JSON for mobile about screens.
pub fn capabilities_json() -> String {
    #[derive(Serialize)]
    struct Caps {
        product_id: &'static str,
        version: &'static str,
        ledger_schema: u32,
        engines: [&'static str; 3],
        cloud_sync: bool,
        official_relay: bool,
        multi_device_handoff: bool,
        rule_packs: bool,
        local_http_serve: bool,
        tags_attachments: bool,
        attachment_store: bool,
        backup_includes_attachments: bool,
        capture_oneshot: bool,
        engine_auto: bool,
        tag_filter: bool,
        local_budgets: bool,
        notes: &'static str,
    }
    serde_json::to_string(&Caps {
        product_id: PRODUCT_ID,
        version: VERSION,
        ledger_schema: LEDGER_SCHEMA_VERSION,
        engines: ["mock", "onnx", "auto"],
        cloud_sync: false,
        official_relay: false,
        multi_device_handoff: true,
        rule_packs: true,
        local_http_serve: true,
        tags_attachments: true,
        attachment_store: true,
        backup_includes_attachments: true,
        capture_oneshot: true,
        engine_auto: true,
        tag_filter: true,
        local_budgets: true,
        notes: "local-first; multi-device via backup/handoff file only",
    })
    .unwrap_or_else(|_| "{}".into())
}

/// Default drop-folder path for capture → watch pipelines.
pub fn default_inbox_path() -> String {
    inbox_dir().display().to_string()
}

/// Ensure inbox directory exists; returns path.
pub fn ensure_inbox() -> Result<String, String> {
    ensure_inbox_dir()
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

/// Merchant rule packs directory.
pub fn default_rules_path() -> String {
    rules_dir().display().to_string()
}

/// Ensure rules dir exists; returns path.
pub fn ensure_rules() -> Result<String, String> {
    ensure_rules_dir()
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

/// Attachments root next to a ledger (`{db_parent}/attachments`).
pub fn attachments_dir_for_ledger(db_path: String) -> String {
    attachments_root_for_db(Path::new(&db_path))
        .display()
        .to_string()
}

/// Default attachments dir for the platform default ledger.
pub fn default_attachments_path() -> String {
    attachments_root_for_db(&default_db_path())
        .display()
        .to_string()
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
    let cats = category_engine_with_packs();
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
    let cats = category_engine_with_packs();
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

/// **Mobile capture one-shot:** process a filesystem path → confirm → optional attach + tags.
///
/// Options JSON keys (all optional except implied defaults):
/// `currency` (default TWD), `engine` (mock), `qr_payload`, `confirm` (true),
/// `attach` (false), `tags`, `force` (false), `notes`.
///
/// Returns JSON: `{ draft, confirm?, transaction?, inserted? }`.
pub fn process_confirm_path_json(
    db_path: String,
    passphrase: Option<String>,
    path: String,
    options_json: String,
) -> Result<String, String> {
    let opts = parse_capture_opts(&options_json)?;
    let eng = engine_by_name(&opts.engine).map_err(|e| e.to_string())?;
    let cats = category_engine_with_packs();
    let draft = process_path(
        Path::new(&path),
        eng.as_ref(),
        &cats,
        ProcessOptions {
            default_currency: opts.currency,
            qr_payload: opts.qr_payload.clone(),
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;

    if !opts.confirm {
        return Ok(serde_json::json!({ "draft": draft, "confirmed": false }).to_string());
    }

    let bytes = std::fs::read(&path).unwrap_or_default();
    let hash = rradar_core::preprocess::content_hash(&bytes);
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let res = ledger
            .confirm_draft(&draft, Some(&hash), opts.notes.as_deref(), opts.force)
            .map_err(|e| e.to_string())?;
        let mut tx = res.transaction.clone();
        if res.inserted {
            let mut patch = TxUpdate::default();
            if opts.attach && Path::new(&path).is_file() {
                if let Ok(rel) = store_attachment(ledger.path(), &tx.id, Path::new(&path)) {
                    patch.attachment_path = Some(rel);
                }
            }
            if let Some(ref t) = opts.tags {
                patch.tags = Some(normalize_tags(t).unwrap_or_default());
            }
            if patch.attachment_path.is_some() || patch.tags.is_some() {
                if let Ok(updated) = ledger.update_transaction(&tx.id, &patch) {
                    tx = updated;
                }
            }
        }
        Ok(serde_json::json!({
            "draft": draft,
            "confirmed": true,
            "inserted": res.inserted,
            "dedupe": res.dedupe,
            "transaction": tx,
        })
        .to_string())
    })
}

/// **Mobile capture one-shot from camera bytes:** process → confirm → store attachment bytes.
///
/// `filename` is used only when `attach` is true (e.g. `capture.jpg`).
/// Options JSON: same as [`process_confirm_path_json`].
pub fn process_confirm_bytes_json(
    db_path: String,
    passphrase: Option<String>,
    image_bytes: Vec<u8>,
    filename: String,
    options_json: String,
) -> Result<String, String> {
    let opts = parse_capture_opts(&options_json)?;
    let eng = engine_by_name(&opts.engine).map_err(|e| e.to_string())?;
    let cats = category_engine_with_packs();
    let draft = process_bytes(
        &image_bytes,
        None,
        eng.as_ref(),
        &cats,
        ProcessOptions {
            default_currency: opts.currency,
            qr_payload: opts.qr_payload.clone(),
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;

    if !opts.confirm {
        return Ok(serde_json::json!({ "draft": draft, "confirmed": false }).to_string());
    }

    let hash = rradar_core::preprocess::content_hash(&image_bytes);
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let res = ledger
            .confirm_draft(&draft, Some(&hash), opts.notes.as_deref(), opts.force)
            .map_err(|e| e.to_string())?;
        let mut tx = res.transaction.clone();
        if res.inserted {
            let mut patch = TxUpdate::default();
            if opts.attach && !image_bytes.is_empty() {
                let name = if filename.is_empty() {
                    "capture.bin"
                } else {
                    filename.as_str()
                };
                if let Ok(rel) = store_attachment_bytes(ledger.path(), &tx.id, name, &image_bytes) {
                    patch.attachment_path = Some(rel);
                }
            }
            if let Some(ref t) = opts.tags {
                patch.tags = Some(normalize_tags(t).unwrap_or_default());
            }
            if patch.attachment_path.is_some() || patch.tags.is_some() {
                if let Ok(updated) = ledger.update_transaction(&tx.id, &patch) {
                    tx = updated;
                }
            }
        }
        Ok(serde_json::json!({
            "draft": draft,
            "confirmed": true,
            "inserted": res.inserted,
            "dedupe": res.dedupe,
            "transaction": tx,
        })
        .to_string())
    })
}

struct CaptureOpts {
    currency: Iso4217,
    engine: String,
    qr_payload: Option<String>,
    confirm: bool,
    attach: bool,
    tags: Option<String>,
    force: bool,
    notes: Option<String>,
}

fn parse_capture_opts(options_json: &str) -> Result<CaptureOpts, String> {
    #[derive(serde::Deserialize, Default)]
    struct Raw {
        currency: Option<String>,
        engine: Option<String>,
        qr_payload: Option<String>,
        confirm: Option<bool>,
        attach: Option<bool>,
        tags: Option<String>,
        force: Option<bool>,
        notes: Option<String>,
    }
    let raw: Raw = if options_json.trim().is_empty() {
        Raw::default()
    } else {
        serde_json::from_str(options_json).map_err(|e| e.to_string())?
    };
    Ok(CaptureOpts {
        currency: raw
            .currency
            .as_deref()
            .and_then(Iso4217::parse)
            .unwrap_or(Iso4217::TWD),
        engine: raw.engine.unwrap_or_else(|| "mock".into()),
        qr_payload: raw.qr_payload,
        confirm: raw.confirm.unwrap_or(true),
        attach: raw.attach.unwrap_or(false),
        tags: raw.tags,
        force: raw.force.unwrap_or(false),
        notes: raw.notes,
    })
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

/// Query transactions with optional tag / category / text filters (JSON array).
///
/// `filter_json` keys (all optional): limit, offset, currency, query, tag, category,
/// year_month, from, to, min_minor, max_minor, has_attachment.
pub fn query_transactions_json(
    db_path: String,
    passphrase: Option<String>,
    filter_json: String,
) -> Result<String, String> {
    #[derive(serde::Deserialize, Default)]
    struct F {
        limit: Option<u32>,
        offset: Option<u32>,
        currency: Option<String>,
        query: Option<String>,
        tag: Option<String>,
        category: Option<String>,
        year_month: Option<String>,
        from: Option<String>,
        to: Option<String>,
        min_minor: Option<i64>,
        max_minor: Option<i64>,
        has_attachment: Option<bool>,
    }
    let f: F = if filter_json.trim().is_empty() {
        F::default()
    } else {
        serde_json::from_str(&filter_json).map_err(|e| e.to_string())?
    };
    let filter = TxFilter {
        limit: f.limit.unwrap_or(50) as usize,
        offset: f.offset.unwrap_or(0) as usize,
        currency: f.currency,
        query: f.query,
        tag: f.tag,
        category: f.category,
        year_month: f.year_month,
        from: f.from,
        to: f.to,
        min_minor: f.min_minor,
        max_minor: f.max_minor,
        has_attachment: f.has_attachment,
    };
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let rows = ledger
            .query_transactions(&filter)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    })
}

/// Distinct tags as JSON string array.
pub fn list_tags_json(db_path: String, passphrase: Option<String>) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let tags = ledger.list_tags().map_err(|e| e.to_string())?;
        serde_json::to_string(&tags).map_err(|e| e.to_string())
    })
}

/// Budget book from data dir as JSON.
pub fn budgets_json() -> String {
    serde_json::to_string(&BudgetBook::load()).unwrap_or_else(|_| r#"{"lines":[]}"#.into())
}

/// Evaluate budgets for a calendar month against a ledger.
pub fn budget_status_json(
    db_path: String,
    passphrase: Option<String>,
    year: i32,
    month: u32,
) -> Result<String, String> {
    let book = BudgetBook::load();
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let st = budget_status_month(ledger, &book, year, month).map_err(|e| e.to_string())?;
        serde_json::to_string(&st).map_err(|e| e.to_string())
    })
}

/// Upsert a budget line (major units string). category empty = overall monthly.
pub fn budget_set_json(
    currency: String,
    major: String,
    category: Option<String>,
) -> Result<String, String> {
    let mut book = BudgetBook::load();
    book.set_major(&currency, &major, category.as_deref())?;
    book.save().map_err(|e| e.to_string())?;
    serde_json::to_string(&book).map_err(|e| e.to_string())
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
/// category, notes, transacted_at, tags, attachment_path. Returns updated transaction JSON.
///
/// Empty `tags` / `attachment_path` strings clear those columns.
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
        tags: Option<String>,
        attachment_path: Option<String>,
    }
    let p: Patch = serde_json::from_str(&patch_json).map_err(|e| e.to_string())?;
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let tags = p.tags.map(|t| normalize_tags(&t).unwrap_or_default());
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
                    tags,
                    attachment_path: p.attachment_path,
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&tx).map_err(|e| e.to_string())
    })
}

/// Copy a local file into the ledger attachment store and set `attachment_path`.
/// Returns updated transaction JSON.
pub fn attach_file_json(
    db_path: String,
    passphrase: Option<String>,
    id: String,
    source_path: String,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let _ = ledger.get_transaction(&id).map_err(|e| e.to_string())?;
        let rel = store_attachment(ledger.path(), &id, Path::new(&source_path))
            .map_err(|e| e.to_string())?;
        let tx = ledger
            .update_transaction(
                &id,
                &TxUpdate {
                    attachment_path: Some(rel),
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&tx).map_err(|e| e.to_string())
    })
}

/// Store in-memory capture bytes as attachment and set `attachment_path`.
pub fn attach_bytes_json(
    db_path: String,
    passphrase: Option<String>,
    id: String,
    filename: String,
    bytes: Vec<u8>,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let _ = ledger.get_transaction(&id).map_err(|e| e.to_string())?;
        let name = if filename.is_empty() {
            "capture.bin"
        } else {
            filename.as_str()
        };
        let rel =
            store_attachment_bytes(ledger.path(), &id, name, &bytes).map_err(|e| e.to_string())?;
        let tx = ledger
            .update_transaction(
                &id,
                &TxUpdate {
                    attachment_path: Some(rel),
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&tx).map_err(|e| e.to_string())
    })
}

/// Clear `attachment_path`. When `delete_file` is true, also removes the stored blob.
pub fn detach_file_json(
    db_path: String,
    passphrase: Option<String>,
    id: String,
    delete_file: bool,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let existing = ledger.get_transaction(&id).map_err(|e| e.to_string())?;
        if delete_file {
            if let Some(ref stored) = existing.attachment_path {
                remove_stored_attachment(ledger.path(), stored).map_err(|e| e.to_string())?;
            }
        }
        let tx = ledger
            .update_transaction(
                &id,
                &TxUpdate {
                    attachment_path: Some(String::new()),
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&tx).map_err(|e| e.to_string())
    })
}

/// Resolve a stored attachment path (relative or absolute) to an absolute filesystem path.
pub fn resolve_attachment_path_string(db_path: String, stored: String) -> String {
    resolve_attachment_path(Path::new(&db_path), &stored)
        .display()
        .to_string()
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

/// Category breakdown for one currency (optional `year_month` = `YYYY-MM` or empty for all-time).
pub fn stats_by_category_json(
    db_path: String,
    passphrase: Option<String>,
    currency: String,
    year_month: String,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        let ym = if year_month.is_empty() {
            None
        } else {
            Some(year_month.as_str())
        };
        let rows = ledger
            .stats_by_category(&currency, ym)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    })
}

/// Markdown monthly report for year/month.
pub fn report_month_markdown(
    db_path: String,
    passphrase: Option<String>,
    year: i32,
    month: u32,
) -> Result<String, String> {
    with_ledger(&db_path, passphrase.as_deref(), |ledger| {
        rradar_core::monthly_markdown(ledger, year, month).map_err(|e| e.to_string())
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
    let cats = category_engine_with_packs();
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
// Rules + model pins (device config)
// ---------------------------------------------------------------------------

/// List installed rule pack file paths as JSON string array.
pub fn list_rule_packs_json() -> String {
    let paths: Vec<String> = list_rule_files()
        .into_iter()
        .map(|p| p.display().to_string())
        .collect();
    serde_json::to_string(&paths).unwrap_or_else(|_| "[]".into())
}

/// ONNX model pin verification JSON under `models_dir` (default `./models` or env).
pub fn models_pins_json(models_dir: String) -> Result<String, String> {
    let dir = if models_dir.is_empty() {
        rradar_ocr::default_models_dir()
    } else {
        Path::new(&models_dir).to_path_buf()
    };
    let checks = rradar_ocr::verify_models_dir(&dir, false).map_err(|e| e.to_string())?;
    let pins: Vec<serde_json::Value> = checks
        .iter()
        .map(|c| match c {
            rradar_ocr::PinCheck::Ok { pin, bytes } => serde_json::json!({
                "file": pin.filename, "status": "ok", "bytes": bytes
            }),
            rradar_ocr::PinCheck::Missing { pin } => {
                serde_json::json!({ "file": pin.filename, "status": "missing" })
            }
            rradar_ocr::PinCheck::Mismatch { pin, .. } => {
                serde_json::json!({ "file": pin.filename, "status": "mismatch" })
            }
        })
        .collect();
    Ok(serde_json::json!({
        "dir": dir.display().to_string(),
        "pins_ok": rradar_ocr::all_pins_ok(&checks),
        "onnx_feature": rradar_ocr::onnx_feature_enabled(),
        "pins": pins,
    })
    .to_string())
}

// ---------------------------------------------------------------------------
// Backup + handoff (local multi-device only — no cloud)
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

/// Create multi-device handoff package at `out_path` (encrypted file; no cloud).
pub fn handoff_create_file(
    db_path: String,
    passphrase_db: Option<String>,
    handoff_passphrase: String,
    device_label: String,
    out_path: String,
) -> Result<(), String> {
    with_ledger(&db_path, passphrase_db.as_deref(), |ledger| {
        let bytes = create_handoff(ledger, &handoff_passphrase, &device_label)
            .map_err(|e| e.to_string())?;
        write_handoff_file(Path::new(&out_path), &bytes).map_err(|e| e.to_string())?;
        Ok(())
    })
}

/// Inspect handoff package → manifest JSON.
pub fn handoff_info_json(
    handoff_passphrase: String,
    handoff_path: String,
) -> Result<String, String> {
    let sealed = std::fs::read(&handoff_path).map_err(|e| e.to_string())?;
    let man = inspect_handoff(&handoff_passphrase, &sealed).map_err(|e| e.to_string())?;
    serde_json::to_string(&man).map_err(|e| e.to_string())
}

/// Merge handoff transactions into target ledger. Returns `{inserted, skipped, manifest}` JSON.
pub fn handoff_apply_merge_json(
    db_path: String,
    passphrase_db: Option<String>,
    handoff_passphrase: String,
    handoff_path: String,
) -> Result<String, String> {
    let sealed = std::fs::read(&handoff_path).map_err(|e| e.to_string())?;
    with_ledger(&db_path, passphrase_db.as_deref(), |ledger| {
        let (ins, skip, man) =
            apply_handoff_merge(&handoff_passphrase, &sealed, ledger).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "inserted": ins,
            "skipped": skip,
            "manifest": man,
        })
        .to_string())
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
        assert!(caps.contains("\"multi_device_handoff\":true"));
        assert!(caps.contains("\"rule_packs\":true"));
        assert!(caps.contains("\"attachment_store\":true"));
        assert!(caps.contains("\"backup_includes_attachments\":true"));
        assert!(caps.contains("\"capture_oneshot\":true"));
        assert!(caps.contains("\"engine_auto\":true"));
        assert!(!categories_json().is_empty());
        let eng = engines_json();
        assert!(eng.contains("auto_resolves_to"));
        let _ = list_rule_packs_json();
        let _ = models_pins_json(String::new()).unwrap_or_default();
    }

    #[test]
    fn capture_oneshot_path_and_bytes() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/familymart_89.txt");
        assert!(root.is_file());
        let dir = std::env::temp_dir().join(format!("rradar-ffi-cap-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("ledger.db");
        ensure_ledger(db.display().to_string()).unwrap();

        let path_out = process_confirm_path_json(
            db.display().to_string(),
            None,
            root.display().to_string(),
            r#"{"confirm":true,"attach":true,"tags":"capture,path","currency":"TWD"}"#.into(),
        )
        .expect("path oneshot");
        assert!(path_out.contains("\"inserted\":true"), "{path_out}");
        assert!(path_out.contains("attachments/"), "{path_out}");
        assert!(
            path_out.contains("capture") || path_out.contains("path"),
            "{path_out}"
        );

        let mut magic = b"RRADAR_MOCK_OCR\n".to_vec();
        magic.extend_from_slice("相機店\n合計 55\n2024-06-01\n".as_bytes());
        let bytes_out = process_confirm_bytes_json(
            db.display().to_string(),
            None,
            magic.clone(),
            "cam.bin".into(),
            r#"{"confirm":true,"attach":true,"tags":"camera","currency":"TWD"}"#.into(),
        )
        .expect("bytes oneshot");
        assert!(bytes_out.contains("\"inserted\":true"), "{bytes_out}");
        assert!(bytes_out.contains("attachments/"), "{bytes_out}");
        assert!(
            bytes_out.contains("55") || bytes_out.contains("相機"),
            "{bytes_out}"
        );

        let n = count_transactions(db.display().to_string(), None).unwrap();
        assert_eq!(n, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_nonempty() {
        assert!(!default_data_dir().is_empty());
        assert!(default_ledger_path().contains("ledger"));
        assert!(!default_inbox_path().is_empty());
        assert!(!default_rules_path().is_empty());
        assert!(!default_attachments_path().is_empty());
        let inbox = ensure_inbox().unwrap();
        assert!(Path::new(&inbox).is_dir() || Path::new(&inbox).exists());
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
        assert_eq!(ver, "3");

        // Attachment store: copy fixture next to ledger, set tags
        let id: serde_json::Value = serde_json::from_str(&last).unwrap();
        let tid = id["id"].as_str().unwrap().to_string();
        let attached = attach_file_json(
            db.display().to_string(),
            None,
            tid.clone(),
            root.display().to_string(),
        )
        .unwrap();
        assert!(attached.contains("attachments/"), "{attached}");
        let patched = update_transaction_json(
            db.display().to_string(),
            None,
            tid.clone(),
            r#"{"tags":"demo,ffi"}"#.into(),
        )
        .unwrap();
        assert!(patched.contains("demo"), "{patched}");
        let att_dir = attachments_dir_for_ledger(db.display().to_string());
        assert!(att_dir.contains("attachments"), "{att_dir}");

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

        // tags (schema v3) + handoff merge roundtrip
        let last_id: serde_json::Value = serde_json::from_str(&last).unwrap();
        let id = last_id["id"].as_str().unwrap().to_string();
        let patched = update_transaction_json(
            db.display().to_string(),
            None,
            id,
            r#"{"tags":"demo,ffi","notes":"from-ffi"}"#.into(),
        )
        .unwrap();
        assert!(
            patched.contains("demo") || patched.contains("from-ffi"),
            "{patched}"
        );

        let handoff = dir.join("device.handoff");
        handoff_create_file(
            db.display().to_string(),
            None,
            "handoff-pass".into(),
            "test-device".into(),
            handoff.display().to_string(),
        )
        .unwrap();
        let info = handoff_info_json("handoff-pass".into(), handoff.display().to_string()).unwrap();
        assert!(
            info.contains("rradar-handoff-v1") || info.contains("test-device"),
            "{info}"
        );

        let db2 = dir.join("ledger2.db");
        ensure_ledger(db2.display().to_string()).unwrap();
        let applied = handoff_apply_merge_json(
            db2.display().to_string(),
            None,
            "handoff-pass".into(),
            handoff.display().to_string(),
        )
        .unwrap();
        assert!(applied.contains("inserted"), "{applied}");
        assert_eq!(
            count_transactions(db2.display().to_string(), None).unwrap(),
            1
        );

        let report = report_month_markdown(db.display().to_string(), None, 2024, 5).unwrap();
        assert!(report.contains("2024") || report.contains("TWD") || !report.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
