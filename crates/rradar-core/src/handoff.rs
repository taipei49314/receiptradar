//! Local multi-device handoff packages (file-based; no cloud relay).

use crate::attachments::{collect_attachment_files, write_attachment_files};
use crate::budget::BudgetBook;
use crate::crypto::{seal_backup, unseal_backup, ARGON2_M_KIB};
use crate::export::{find_budgets_file, pack_archive, unpack_archive, ExportError};
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
    /// Receipt attachment blobs packed under `attachments/` (0 if none / legacy).
    #[serde(default)]
    pub attachment_count: u32,
    /// Soft budgets file packed as `budgets.toml`.
    #[serde(default)]
    pub has_budgets: bool,
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
    let att_files = collect_attachment_files(ledger.path())?;
    let budgets = find_budgets_file(ledger.path()).and_then(|p| std::fs::read(p).ok());
    let manifest = HandoffManifest {
        format: "rradar-handoff-v1".into(),
        schema_version: LEDGER_SCHEMA_VERSION,
        created_at: utc_now_iso(),
        app_version: VERSION.into(),
        device_label: device_label.into(),
        transaction_count: txs.len() as i64,
        note: format!("{PRODUCT_ID} multi-device handoff; decrypt offline only"),
        attachment_count: att_files.len() as u32,
        has_budgets: budgets.is_some(),
    };
    let man = serde_json::to_vec_pretty(&manifest)?;
    let mut owned: Vec<(String, Vec<u8>)> =
        Vec::with_capacity(4 + att_files.len() + budgets.is_some() as usize);
    owned.push(("handoff.json".into(), man));
    owned.push(("ledger.sqlite".into(), sqlite));
    owned.push(("transactions.json".into(), txs_json));
    if let Some(b) = budgets {
        owned.push(("budgets.toml".into(), b));
    }
    owned.extend(att_files);
    let refs: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    let archive = pack_archive(&refs);
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

/// Apply handoff: merge transactions and re-hydrate attachment blobs next to target ledger.
pub fn apply_handoff_merge(
    passphrase: &str,
    sealed: &[u8],
    target: &Ledger,
) -> Result<(usize, usize, HandoffManifest), ExportError> {
    let plain = unseal_backup(passphrase, sealed)?;
    let files = unpack_archive(&plain)?;
    let mut manifest: Option<HandoffManifest> = None;
    let mut txs_json: Option<Vec<u8>> = None;
    let mut budgets_toml: Option<Vec<u8>> = None;
    let mut attachments: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, data) in files {
        let norm = name.replace('\\', "/");
        match norm.as_str() {
            "handoff.json" => manifest = Some(serde_json::from_slice(&data)?),
            "transactions.json" => txs_json = Some(data),
            "budgets.toml" => budgets_toml = Some(data),
            n if n.starts_with("attachments/") => attachments.push((norm, data)),
            _ => {}
        }
    }
    let mut manifest =
        manifest.ok_or_else(|| ExportError::Format("missing handoff.json".into()))?;
    let raw = txs_json.ok_or_else(|| ExportError::Format("missing transactions.json".into()))?;
    let rows: Vec<crate::ledger::Transaction> = serde_json::from_slice(&raw)?;
    let (ins, skip) = target.import_transactions(&rows)?;
    let _ = write_attachment_files(target.path(), &attachments)?;
    if let Some(bytes) = budgets_toml {
        manifest.has_budgets = true;
        if let Some(parent) = target.path().parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
                let _ = std::fs::write(parent.join("budgets.toml"), &bytes);
            }
        }
        let global = BudgetBook::path();
        if let Some(p) = global.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(global, bytes);
    }
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
