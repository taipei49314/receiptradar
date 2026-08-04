//! P2 at-rest: seal entire SQLite file with Argon2id + XChaCha20-Poly1305.

use crate::crypto::{seal_bytes, unseal_bytes, ARGON2_M_KIB};
use crate::ledger::{Ledger, LedgerError};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SealedError {
    #[error("ledger: {0}")]
    Ledger(#[from] LedgerError),
    #[error("crypto: {0}")]
    Crypto(#[from] crate::crypto::CryptoError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

/// Open a plaintext `.db` or decrypt a `.rrsealed` file into a temp DB.
pub fn open_ledger_auto(
    path: &Path,
    passphrase: Option<&str>,
) -> Result<(Ledger, Option<PathBuf>), SealedError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "rrsealed" {
        let pass = passphrase
            .ok_or_else(|| SealedError::Msg("sealed database requires --passphrase".into()))?;
        let sealed = std::fs::read(path)?;
        let plain = unseal_bytes(pass, &sealed, ARGON2_M_KIB)?;
        let tmp = std::env::temp_dir().join(format!("rradar-open-{}.db", ulid::Ulid::new()));
        std::fs::write(&tmp, &plain)?;
        let mut ledger = Ledger::open(&tmp)?;
        ledger.configure_sealed_purge(path.to_path_buf(), pass.to_owned());
        return Ok((ledger, Some(tmp)));
    }

    Ok((Ledger::open(path)?, None))
}

/// Persist ledger to a sealed path (encrypts SQLite bytes).
pub fn save_sealed(
    ledger: &Ledger,
    sealed_path: &Path,
    passphrase: &str,
) -> Result<(), SealedError> {
    let bytes = ledger.export_sqlite_bytes()?;
    let sealed = seal_bytes(passphrase, &bytes, ARGON2_M_KIB)?;
    if let Some(parent) = sealed_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut output = atomic_write_file::AtomicWriteFile::options().open(sealed_path)?;
    output.write_all(&sealed)?;
    output.commit()?;
    Ok(())
}

/// Convert plaintext DB file → sealed file.
pub fn seal_db_file(
    db_path: &Path,
    sealed_path: &Path,
    passphrase: &str,
) -> Result<(), SealedError> {
    let ledger = Ledger::open(db_path)?;
    save_sealed(&ledger, sealed_path, passphrase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachments::{resolve_attachment_path, store_attachment_bytes};
    use crate::money::{Iso4217, Money};
    use crate::types::{Field, FieldSource, ReceiptDraft, SourcePath};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn case_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rradar-seal-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn sample_draft(id: &str) -> ReceiptDraft {
        ReceiptDraft {
            id: id.into(),
            captured_at: "2024-01-01T00:00:00Z".into(),
            merchant: Field::new("X".into(), 1.0, FieldSource::User),
            total: Field::new(Money::new(99, Iso4217::TWD), 1.0, FieldSource::User),
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
    fn seal_and_reopen() {
        let dir = case_dir("reopen");
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("t.db");
        let sealed_path = dir.join("t.rrsealed");

        {
            let ledger = Ledger::open(&db_path).unwrap();
            let draft = sample_draft("s1");
            ledger.confirm_draft(&draft, None, None, false).unwrap();
            save_sealed(&ledger, &sealed_path, "pw").unwrap();
        }

        let (ledger, tmp) = open_ledger_auto(&sealed_path, Some("pw")).unwrap();
        assert_eq!(ledger.count().unwrap(), 1);
        if let Some(t) = tmp {
            let _ = std::fs::remove_file(t);
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sealed_purge_persists_before_cleaning_the_logical_attachment_root() {
        let dir = case_dir("purge");
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("source.db");
        let sealed_path = dir.join("ledger.rrsealed");
        let attachment_path = {
            let ledger = Ledger::open(&db_path).unwrap();
            ledger
                .confirm_draft(&sample_draft("purge-me"), None, None, false)
                .unwrap();
            let stored =
                store_attachment_bytes(&sealed_path, "purge-me", "receipt.jpg", b"receipt")
                    .unwrap();
            ledger
                .connection()
                .execute(
                    "UPDATE transactions SET attachment_path = ?2 WHERE id = ?1",
                    rusqlite::params!["purge-me", stored],
                )
                .unwrap();
            save_sealed(&ledger, &sealed_path, "pw").unwrap();
            resolve_attachment_path(&sealed_path, &stored)
        };

        let (ledger, tmp) = open_ledger_auto(&sealed_path, Some("pw")).unwrap();
        let report = ledger.purge_transaction("purge-me").unwrap();
        assert_eq!(report.purged_transactions, 1);
        assert_eq!(report.attachments.deleted.len(), 1, "{report:?}");
        assert!(!attachment_path.exists());
        assert!(dir.join("attachments").is_dir());
        drop(ledger);
        if let Some(tmp) = tmp {
            std::fs::remove_file(tmp).unwrap();
        }

        let (reopened, tmp) = open_ledger_auto(&sealed_path, Some("pw")).unwrap();
        assert_eq!(reopened.count().unwrap(), 0);
        drop(reopened);
        if let Some(tmp) = tmp {
            std::fs::remove_file(tmp).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sealed_purge_persistence_failure_does_not_delete_attachment() {
        let dir = case_dir("persist-failure");
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("source.db");
        let sealed_path = dir.join("ledger.rrsealed");
        let original_path = dir.join("original.rrsealed");
        let attachment_path = {
            let ledger = Ledger::open(&db_path).unwrap();
            ledger
                .confirm_draft(&sample_draft("keep-me"), None, None, false)
                .unwrap();
            let stored =
                store_attachment_bytes(&sealed_path, "keep-me", "receipt.jpg", b"receipt").unwrap();
            ledger
                .connection()
                .execute(
                    "UPDATE transactions SET attachment_path = ?2 WHERE id = ?1",
                    rusqlite::params!["keep-me", stored],
                )
                .unwrap();
            save_sealed(&ledger, &sealed_path, "pw").unwrap();
            resolve_attachment_path(&sealed_path, &stored)
        };

        let (ledger, tmp) = open_ledger_auto(&sealed_path, Some("pw")).unwrap();
        std::fs::rename(&sealed_path, &original_path).unwrap();
        std::fs::create_dir(&sealed_path).unwrap();
        assert!(ledger.purge_transaction("keep-me").is_err());
        assert!(attachment_path.is_file());
        drop(ledger);
        if let Some(tmp) = tmp {
            std::fs::remove_file(tmp).unwrap();
        }

        let (reopened, tmp) = open_ledger_auto(&original_path, Some("pw")).unwrap();
        assert_eq!(reopened.count().unwrap(), 1);
        drop(reopened);
        if let Some(tmp) = tmp {
            std::fs::remove_file(tmp).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
