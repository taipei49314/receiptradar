//! Local multi-device handoff packages (file-based; no cloud relay).

use crate::crypto::{seal_backup, unseal_backup, ARGON2_M_KIB};
use crate::export::{pack_archive, unpack_archive, ExportError};
use crate::ledger::Ledger;
use crate::pipeline::utc_now_iso;
use crate::{LEDGER_SCHEMA_VERSION, PRODUCT_ID, VERSION};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct HandoffManifest {
    pub format: String,
    pub schema_version: u32,
    pub created_at: String,
    pub app_version: String,
    pub device_label: String,
    pub transaction_count: i64,
    pub note: String,
}

/// Create encrypted handoff blob (same crypto family as backup.rradar).
pub fn create_handoff(
    ledger: &Ledger,
    passphrase: &str,
    device_label: &str,
) -> Result<Vec<u8>, ExportError> {
    let sqlite = ledger.export_sqlite_bytes()?;
    let txs = ledger.export_all()?;
    let txs_json = serde_json::to_vec_pretty(&txs)?;
    let manifest = HandoffManifest {
        format: "rradar-handoff-v1".into(),
        schema_version: LEDGER_SCHEMA_VERSION,
        created_at: utc_now_iso(),
        app_version: VERSION.into(),
        device_label: device_label.into(),
        transaction_count: txs.len() as i64,
        note: format!("{PRODUCT_ID} multi-device handoff; decrypt offline only"),
    };
    let man = serde_json::to_vec_pretty(&manifest)?;
    let archive = pack_archive(&[
        ("handoff.json", &man),
        ("ledger.sqlite", &sqlite),
        ("transactions.json", &txs_json),
    ]);
    let m = if std::env::var("RRADAR_FAST_BACKUP").is_ok() {
        8
    } else {
        ARGON2_M_KIB
    };
    Ok(seal_backup(passphrase, &archive, m)?)
}

pub fn inspect_handoff(passphrase: &str, sealed: &[u8]) -> Result<HandoffManifest, ExportError> {
    let plain = unseal_backup(passphrase, sealed)?;
    let files = unpack_archive(&plain)?;
    for (name, data) in files {
        if name == "handoff.json" {
            return Ok(serde_json::from_slice(&data)?);
        }
    }
    Err(ExportError::Format("missing handoff.json".into()))
}

/// Apply handoff: write sqlite path and/or merge transactions.
pub fn apply_handoff_merge(
    passphrase: &str,
    sealed: &[u8],
    target: &Ledger,
) -> Result<(usize, usize, HandoffManifest), ExportError> {
    let plain = unseal_backup(passphrase, sealed)?;
    let files = unpack_archive(&plain)?;
    let mut manifest: Option<HandoffManifest> = None;
    let mut txs_json: Option<Vec<u8>> = None;
    for (name, data) in files {
        match name.as_str() {
            "handoff.json" => manifest = Some(serde_json::from_slice(&data)?),
            "transactions.json" => txs_json = Some(data),
            _ => {}
        }
    }
    let manifest = manifest.ok_or_else(|| ExportError::Format("missing handoff.json".into()))?;
    let raw = txs_json.ok_or_else(|| ExportError::Format("missing transactions.json".into()))?;
    let rows: Vec<crate::ledger::Transaction> = serde_json::from_slice(&raw)?;
    let (ins, skip) = target.import_transactions(&rows)?;
    Ok((ins, skip, manifest))
}

pub fn write_handoff_file(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)?;
    Ok(())
}
