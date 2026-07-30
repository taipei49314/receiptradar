//! CSV / JSON export and backup.rradar packaging.

use crate::crypto::{seal_backup, unseal_backup, CryptoError, ARGON2_M_KIB};
use crate::ledger::{Ledger, LedgerError, Transaction};
use crate::pipeline::utc_now_iso;
use crate::VERSION;
use serde::{Deserialize, Serialize};
use std::io::Write;
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupManifest {
    pub schema_version: u32,
    pub created_at: String,
    pub app_version: String,
    pub transaction_count: i64,
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
    writeln!(
        w,
        "id,confirmed_at,transacted_at,merchant,amount_minor,currency,exponent,category,invoice_id,source_path,confidence,notes"
    )?;
    for t in rows {
        let inv = t.invoice_id.as_deref().unwrap_or("");
        let notes = t.notes.as_deref().unwrap_or("").replace('"', "\"\"");
        let merchant = t.merchant.replace('"', "\"\"");
        writeln!(
            w,
            "{},{},{},\"{}\",{},{},{},{},{},{},{:.4},\"{}\"",
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
            notes
        )?;
    }
    Ok(String::from_utf8(w)?)
}

pub fn transactions_to_json(rows: &[Transaction]) -> Result<String, ExportError> {
    Ok(serde_json::to_string_pretty(rows)?)
}

/// Create encrypted backup.rradar bytes from ledger.
pub fn create_backup(
    ledger: &Ledger,
    passphrase: &str,
    m_kib: u32,
) -> Result<Vec<u8>, ExportError> {
    let txs = ledger.export_all()?;
    let sqlite = ledger.export_sqlite_bytes()?;
    let manifest = BackupManifest {
        schema_version: 1,
        created_at: utc_now_iso(),
        app_version: VERSION.to_string(),
        transaction_count: txs.len() as i64,
    };
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    let txs_json = serde_json::to_vec(&txs)?;
    let archive = pack_archive(&[
        ("manifest.json", &manifest_json),
        ("ledger.sqlite", &sqlite),
        ("transactions.json", &txs_json),
    ]);
    Ok(seal_backup(passphrase, &archive, m_kib)?)
}

pub fn create_backup_default_params(ledger: &Ledger, passphrase: &str) -> Result<Vec<u8>, ExportError> {
    create_backup(ledger, passphrase, ARGON2_M_KIB)
}

#[derive(Debug)]
pub struct RestoredBackup {
    pub manifest: BackupManifest,
    pub transactions_json: Option<Vec<u8>>,
    pub sqlite_bytes: Option<Vec<u8>>,
}

pub fn restore_backup(passphrase: &str, sealed: &[u8]) -> Result<RestoredBackup, ExportError> {
    let plain = unseal_backup(passphrase, sealed)?;
    let files = unpack_archive(&plain)?;
    let mut manifest: Option<BackupManifest> = None;
    let mut transactions_json = None;
    let mut sqlite_bytes = None;
    for (name, data) in files {
        match name.as_str() {
            "manifest.json" => {
                manifest = Some(serde_json::from_slice(&data)?);
            }
            "transactions.json" => transactions_json = Some(data),
            "ledger.sqlite" => sqlite_bytes = Some(data),
            _ => {}
        }
    }
    let manifest = manifest.ok_or_else(|| ExportError::Format("missing manifest".into()))?;
    Ok(RestoredBackup {
        manifest,
        transactions_json,
        sqlite_bytes,
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

        let sealed = create_backup(&db, "secret", 8).unwrap();
        let restored = restore_backup("secret", &sealed).unwrap();
        assert_eq!(restored.manifest.transaction_count, 1);
        assert!(restored.sqlite_bytes.unwrap().len() > 100);
        assert!(restore_backup("wrong", &sealed).is_err());
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
