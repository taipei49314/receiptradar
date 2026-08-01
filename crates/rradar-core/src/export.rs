//! CSV / JSON export and backup.rradar packaging.

use crate::aliases::{find_aliases_file, AliasBook};
use crate::attachments::{collect_attachment_files, write_attachment_files, AttachmentError};
use crate::budget::BudgetBook;
use crate::crypto::{seal_backup, unseal_backup, CryptoError, ARGON2_M_KIB};
use crate::ledger::{Ledger, LedgerError, Transaction};
use crate::pipeline::utc_now_iso;
use crate::VERSION;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("ledger: {0}")]
    Ledger(#[from] LedgerError),
    #[error("crypto: {0}")]
    Crypto(#[from] CryptoError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("utf8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("backup format: {0}")]
    Format(String),
    #[error("attachment: {0}")]
    Attachment(#[from] AttachmentError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Backup package format version (wire/archive layout).
    pub schema_version: u32,
    pub created_at: String,
    pub app_version: String,
    pub transaction_count: i64,
    /// Ledger SQLite schema version at backup time (0 = legacy / unknown).
    #[serde(default)]
    pub ledger_schema_version: u32,
    /// Number of receipt attachment blobs packed under `attachments/` (0 if none / legacy).
    #[serde(default)]
    pub attachment_count: u32,
    /// True when `budgets.toml` was packed (local soft budgets; not SQLite).
    #[serde(default)]
    pub has_budgets: bool,
    /// True when `merchant_aliases.toml` was packed.
    #[serde(default)]
    pub has_aliases: bool,
}

/// Opened backup with archive inventory (for `backup info` / verify).
#[derive(Debug, Clone, Serialize)]
pub struct BackupInspect {
    pub manifest: BackupManifest,
    pub files: Vec<BackupFileInfo>,
    pub has_sqlite: bool,
    pub has_transactions_json: bool,
    pub attachment_file_count: usize,
    pub has_budgets: bool,
    pub has_aliases: bool,
}

/// Locate a budgets.toml near the ledger or under the default data dir.
pub fn find_budgets_file(ledger_path: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(parent) = ledger_path.parent() {
        if !parent.as_os_str().is_empty() {
            candidates.push(parent.join("budgets.toml"));
        }
    }
    candidates.push(BudgetBook::path());
    candidates.into_iter().find(|p| p.is_file())
}

fn collect_budgets_bytes(ledger_path: &Path) -> Option<Vec<u8>> {
    let path = find_budgets_file(ledger_path)?;
    std::fs::read(path).ok()
}

fn collect_aliases_bytes(ledger_path: &Path) -> Option<Vec<u8>> {
    let path = find_aliases_file(ledger_path)?;
    std::fs::read(path).ok()
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupFileInfo {
    pub name: String,
    pub bytes: usize,
}

/// Simple multi-file archive (not tar — portable, length-prefixed).
/// Layout: u32le count, then for each: u32le name_len, name utf8, u64le data_len, data.
pub fn pack_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for (name, data) in files {
        let nb = name.as_bytes();
        out.extend_from_slice(&(nb.len() as u32).to_le_bytes());
        out.extend_from_slice(nb);
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

pub fn unpack_archive(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, ExportError> {
    if bytes.len() < 4 {
        return Err(ExportError::Format("short archive".into()));
    }
    let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let mut off = 4usize;
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        if off + 4 > bytes.len() {
            return Err(ExportError::Format("truncated name len".into()));
        }
        let nlen = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize;
        off += 4;
        if off + nlen + 8 > bytes.len() {
            return Err(ExportError::Format("truncated name/data".into()));
        }
        let name = String::from_utf8(bytes[off..off + nlen].to_vec())?;
        off += nlen;
        let dlen = u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap()) as usize;
        off += 8;
        if off + dlen > bytes.len() {
            return Err(ExportError::Format("truncated data".into()));
        }
        files.push((name, bytes[off..off + dlen].to_vec()));
        off += dlen;
    }
    Ok(files)
}

pub fn transactions_to_csv(rows: &[Transaction]) -> Result<String, ExportError> {
    let mut w = Vec::new();
    // UTF-8 BOM helps Excel on Windows open CJK correctly
    w.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    writeln!(
        w,
        "id,confirmed_at,transacted_at,merchant,amount_minor,currency,exponent,category,invoice_id,source_path,confidence,notes,tags,attachment_path"
    )?;
    for t in rows {
        let inv = t.invoice_id.as_deref().unwrap_or("");
        let notes = t.notes.as_deref().unwrap_or("").replace('"', "\"\"");
        let merchant = t.merchant.replace('"', "\"\"");
        let tags = t.tags.as_deref().unwrap_or("").replace('"', "\"\"");
        let att = t
            .attachment_path
            .as_deref()
            .unwrap_or("")
            .replace('"', "\"\"");
        writeln!(
            w,
            "{},{},{},\"{}\",{},{},{},{},{},{},{:.4},\"{}\",\"{}\",\"{}\"",
            t.id,
            t.confirmed_at,
            t.transacted_at,
            merchant,
            t.amount_minor,
            t.currency,
            t.exponent,
            t.category,
            inv,
            t.source_path,
            t.overall_confidence,
            notes,
            tags,
            att
        )?;
    }
    Ok(String::from_utf8(w)?)
}

pub fn transactions_to_json(rows: &[Transaction]) -> Result<String, ExportError> {
    Ok(serde_json::to_string_pretty(rows)?)
}

/// Create encrypted backup.rradar bytes from ledger.
///
/// Includes optional receipt blobs from `{db_parent}/attachments/**` and
/// optional local soft budgets (`budgets.toml`) when present.
pub fn create_backup(
    ledger: &Ledger,
    passphrase: &str,
    m_kib: u32,
) -> Result<Vec<u8>, ExportError> {
    let txs = ledger.export_all()?;
    let sqlite = ledger.export_sqlite_bytes()?;
    let ledger_schema = ledger.schema_version_u32().unwrap_or(0);
    let att_files = collect_attachment_files(ledger.path())?;
    let budgets = collect_budgets_bytes(ledger.path());
    let aliases = collect_aliases_bytes(ledger.path());
    let manifest = BackupManifest {
        schema_version: 1,
        created_at: utc_now_iso(),
        app_version: VERSION.to_string(),
        transaction_count: txs.len() as i64,
        ledger_schema_version: ledger_schema,
        attachment_count: att_files.len() as u32,
        has_budgets: budgets.is_some(),
        has_aliases: aliases.is_some(),
    };
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    let txs_json = serde_json::to_vec(&txs)?;

    // Own all payloads so we can pack a dynamic file list (core + attachments + local files).
    let mut owned: Vec<(String, Vec<u8>)> = Vec::with_capacity(
        5 + att_files.len() + budgets.is_some() as usize + aliases.is_some() as usize,
    );
    owned.push(("manifest.json".into(), manifest_json));
    owned.push(("ledger.sqlite".into(), sqlite));
    owned.push(("transactions.json".into(), txs_json));
    if let Some(b) = budgets {
        owned.push(("budgets.toml".into(), b));
    }
    if let Some(a) = aliases {
        owned.push(("merchant_aliases.toml".into(), a));
    }
    owned.extend(att_files);

    let refs: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    let archive = pack_archive(&refs);
    Ok(seal_backup(passphrase, &archive, m_kib)?)
}

/// Decrypt and inventory a backup without writing a database.
pub fn inspect_backup(passphrase: &str, sealed: &[u8]) -> Result<BackupInspect, ExportError> {
    let plain = unseal_backup(passphrase, sealed)?;
    let files_raw = unpack_archive(&plain)?;
    let mut manifest: Option<BackupManifest> = None;
    let mut has_sqlite = false;
    let mut has_transactions_json = false;
    let mut has_budgets = false;
    let mut has_aliases = false;
    let mut attachment_file_count = 0usize;
    let mut files = Vec::with_capacity(files_raw.len());
    for (name, data) in &files_raw {
        let norm = name.replace('\\', "/");
        match norm.as_str() {
            "manifest.json" => manifest = Some(serde_json::from_slice(data)?),
            "ledger.sqlite" => has_sqlite = true,
            "transactions.json" => has_transactions_json = true,
            "budgets.toml" => has_budgets = true,
            "merchant_aliases.toml" => has_aliases = true,
            n if n.starts_with("attachments/") => attachment_file_count += 1,
            _ => {}
        }
        files.push(BackupFileInfo {
            name: name.clone(),
            bytes: data.len(),
        });
    }
    let mut manifest = manifest.ok_or_else(|| ExportError::Format("missing manifest".into()))?;
    if has_budgets {
        manifest.has_budgets = true;
    }
    if has_aliases {
        manifest.has_aliases = true;
    }
    Ok(BackupInspect {
        manifest,
        files,
        has_sqlite,
        has_transactions_json,
        attachment_file_count,
        has_budgets,
        has_aliases,
    })
}

/// Decrypt + structural check (required entries present). Returns manifest on success.
pub fn verify_backup(passphrase: &str, sealed: &[u8]) -> Result<BackupManifest, ExportError> {
    let info = inspect_backup(passphrase, sealed)?;
    if !info.has_sqlite {
        return Err(ExportError::Format("missing ledger.sqlite".into()));
    }
    if !info.has_transactions_json {
        return Err(ExportError::Format("missing transactions.json".into()));
    }
    if info.manifest.transaction_count < 0 {
        return Err(ExportError::Format("invalid transaction_count".into()));
    }
    Ok(info.manifest)
}

/// Parse transactions array from a restored backup (for merge import).
pub fn transactions_from_backup(
    restored: &RestoredBackup,
) -> Result<Vec<Transaction>, ExportError> {
    let raw = restored
        .transactions_json
        .as_ref()
        .ok_or_else(|| ExportError::Format("missing transactions.json".into()))?;
    Ok(serde_json::from_slice(raw)?)
}

pub fn create_backup_default_params(
    ledger: &Ledger,
    passphrase: &str,
) -> Result<Vec<u8>, ExportError> {
    create_backup(ledger, passphrase, ARGON2_M_KIB)
}

#[derive(Debug)]
pub struct RestoredBackup {
    pub manifest: BackupManifest,
    pub transactions_json: Option<Vec<u8>>,
    pub sqlite_bytes: Option<Vec<u8>>,
    /// Attachment archive entries (`attachments/...` → bytes).
    pub attachments: Vec<(String, Vec<u8>)>,
    /// Optional local soft budgets file.
    pub budgets_toml: Option<Vec<u8>>,
    /// Optional merchant aliases file.
    pub aliases_toml: Option<Vec<u8>>,
}

pub fn restore_backup(passphrase: &str, sealed: &[u8]) -> Result<RestoredBackup, ExportError> {
    let plain = unseal_backup(passphrase, sealed)?;
    let files = unpack_archive(&plain)?;
    let mut manifest: Option<BackupManifest> = None;
    let mut transactions_json = None;
    let mut sqlite_bytes = None;
    let mut budgets_toml = None;
    let mut aliases_toml = None;
    let mut attachments = Vec::new();
    for (name, data) in files {
        let norm = name.replace('\\', "/");
        match norm.as_str() {
            "manifest.json" => {
                manifest = Some(serde_json::from_slice(&data)?);
            }
            "transactions.json" => transactions_json = Some(data),
            "ledger.sqlite" => sqlite_bytes = Some(data),
            "budgets.toml" => budgets_toml = Some(data),
            "merchant_aliases.toml" => aliases_toml = Some(data),
            n if n.starts_with("attachments/") => attachments.push((norm, data)),
            _ => {}
        }
    }
    let mut manifest = manifest.ok_or_else(|| ExportError::Format("missing manifest".into()))?;
    if budgets_toml.is_some() {
        manifest.has_budgets = true;
    }
    if aliases_toml.is_some() {
        manifest.has_aliases = true;
    }
    Ok(RestoredBackup {
        manifest,
        transactions_json,
        sqlite_bytes,
        attachments,
        budgets_toml,
        aliases_toml,
    })
}

/// Write sqlite bytes to path (after restore).
pub fn write_restored_db(path: &std::path::Path, sqlite_bytes: &[u8]) -> Result<(), ExportError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, sqlite_bytes)?;
    Ok(())
}

/// Write attachment blobs from a restored backup next to the target ledger path.
pub fn write_restored_attachments(
    db_path: &std::path::Path,
    restored: &RestoredBackup,
) -> Result<usize, ExportError> {
    Ok(write_attachment_files(db_path, &restored.attachments)?)
}

/// Write restored `budgets.toml` next to the ledger and into the default data dir.
///
/// Returns whether a budgets file was written.
pub fn write_restored_budgets(
    db_path: &Path,
    restored: &RestoredBackup,
) -> Result<bool, ExportError> {
    let Some(bytes) = restored.budgets_toml.as_ref() else {
        return Ok(false);
    };
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
            std::fs::write(parent.join("budgets.toml"), bytes)?;
        }
    }
    // Also hydrate the process default data dir so `rradar budget status` sees it.
    let global = BudgetBook::path();
    if let Some(parent) = global.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(global, bytes)?;
    Ok(true)
}

/// Write restored merchant aliases next to ledger and into the default data dir.
pub fn write_restored_aliases(
    db_path: &Path,
    restored: &RestoredBackup,
) -> Result<bool, ExportError> {
    let Some(bytes) = restored.aliases_toml.as_ref() else {
        return Ok(false);
    };
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
            std::fs::write(parent.join("merchant_aliases.toml"), bytes)?;
        }
    }
    let global = AliasBook::path();
    if let Some(parent) = global.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(global, bytes)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Ledger;
    use crate::money::{Iso4217, Money};
    use crate::types::{Field, FieldSource, ReceiptDraft, SourcePath};

    fn draft() -> ReceiptDraft {
        ReceiptDraft {
            id: "x1".into(),
            captured_at: "2024-01-01T00:00:00Z".into(),
            merchant: Field::new("Test".into(), 1.0, FieldSource::User),
            total: Field::new(Money::new(500, Iso4217::USD), 1.0, FieldSource::User),
            transacted_at: Field::new("2024-01-01".into(), 1.0, FieldSource::User),
            tax: None,
            invoice_id: None,
            category: Field::new("other".into(), 1.0, FieldSource::User),
            raw_text: "".into(),
            ocr_blocks: vec![],
            overall_confidence: 1.0,
            explain: crate::ExplainTrace::new("t", "ocr"),
            source_path: SourcePath::Manual,
        }
    }

    #[test]
    fn csv_and_backup_roundtrip() {
        let db = Ledger::open_in_memory().unwrap();
        db.confirm_draft(&draft(), Some("h"), Some("note"), false)
            .unwrap();
        let rows = db.export_all().unwrap();
        let csv = transactions_to_csv(&rows).unwrap();
        assert!(csv.contains("amount_minor"));
        assert!(csv.contains("500"));
        assert!(csv.contains("tags"));
        assert!(csv.contains("attachment_path"));

        let sealed = create_backup(&db, "secret", 8).unwrap();
        let restored = restore_backup("secret", &sealed).unwrap();
        assert_eq!(restored.manifest.transaction_count, 1);
        assert!(restored.manifest.ledger_schema_version >= 1);
        assert!(restored.sqlite_bytes.unwrap().len() > 100);
        assert!(restore_backup("wrong", &sealed).is_err());

        let info = inspect_backup("secret", &sealed).unwrap();
        assert!(info.has_sqlite && info.has_transactions_json);
        let m = verify_backup("secret", &sealed).unwrap();
        assert_eq!(m.transaction_count, 1);
        let txs = transactions_from_backup(&restore_backup("secret", &sealed).unwrap()).unwrap();
        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn backup_includes_budgets_toml() {
        let tmp = std::env::temp_dir().join(format!("rradar-bak-bud-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let db_path = tmp.join("ledger.db");
        let ledger = Ledger::open(&db_path).unwrap();
        ledger
            .confirm_draft(&draft(), Some("h"), Some("note"), false)
            .unwrap();
        let budgets = tmp.join("budgets.toml");
        std::fs::write(
            &budgets,
            "# test\nmonthly.TWD = 1000\ncategory.TWD.food_dining = 500\n",
        )
        .unwrap();

        let sealed = create_backup(&ledger, "secret", 8).unwrap();
        let info = inspect_backup("secret", &sealed).unwrap();
        assert!(info.has_budgets, "expected budgets.toml in archive");
        assert!(info.manifest.has_budgets);

        let dest = tmp.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        let dest_db = dest.join("ledger.db");
        let restored = restore_backup("secret", &sealed).unwrap();
        write_restored_db(&dest_db, restored.sqlite_bytes.as_ref().unwrap()).unwrap();
        assert!(write_restored_budgets(&dest_db, &restored).unwrap());
        let got = std::fs::read_to_string(dest.join("budgets.toml")).unwrap();
        assert!(got.contains("monthly.TWD"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn backup_includes_attachment_blobs() {
        let tmp = std::env::temp_dir().join(format!("rradar-bak-att-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let db_path = tmp.join("ledger.db");
        let ledger = Ledger::open(&db_path).unwrap();
        let res = ledger
            .confirm_draft(&draft(), Some("h"), Some("note"), false)
            .unwrap();
        let tx_id = res.transaction.id.clone();
        let src = tmp.join("shot.png");
        std::fs::write(&src, b"PNGDATA").unwrap();
        let rel = crate::attachments::store_attachment(&db_path, &tx_id, &src).unwrap();
        ledger
            .update_transaction(
                &tx_id,
                &crate::ledger::TxUpdate {
                    attachment_path: Some(rel.clone()),
                    tags: Some("demo,receipt".into()),
                    ..Default::default()
                },
            )
            .unwrap();

        let sealed = create_backup(&ledger, "secret", 8).unwrap();
        let info = inspect_backup("secret", &sealed).unwrap();
        assert_eq!(info.attachment_file_count, 1);
        assert_eq!(info.manifest.attachment_count, 1);
        assert!(info
            .files
            .iter()
            .any(|f| f.name.starts_with("attachments/")));

        let dest = tmp.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        let dest_db = dest.join("ledger.db");
        let restored = restore_backup("secret", &sealed).unwrap();
        write_restored_db(&dest_db, restored.sqlite_bytes.as_ref().unwrap()).unwrap();
        let n = write_restored_attachments(&dest_db, &restored).unwrap();
        assert_eq!(n, 1);
        let abs = crate::attachments::resolve_attachment_path(&dest_db, &rel);
        assert_eq!(std::fs::read(abs).unwrap(), b"PNGDATA");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn pack_unpack() {
        let a = pack_archive(&[("a.txt", b"hi"), ("b.bin", &[1, 2, 3])]);
        let files = unpack_archive(&a).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "a.txt");
        assert_eq!(files[0].1, b"hi");
    }
}
