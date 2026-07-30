//! P2 at-rest: seal entire SQLite file with Argon2id + XChaCha20-Poly1305.

use crate::crypto::{seal_bytes, unseal_bytes, ARGON2_M_KIB};
use crate::ledger::{Ledger, LedgerError};
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
        let ledger = Ledger::open(&tmp)?;
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
    std::fs::write(sealed_path, sealed)?;
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
    use crate::money::{Iso4217, Money};
    use crate::types::{Field, FieldSource, ReceiptDraft, SourcePath};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn seal_and_reopen() {
        let dir = std::env::temp_dir().join(format!(
            "rradar-seal-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("t.db");
        let sealed_path = dir.join("t.rrsealed");

        {
            let ledger = Ledger::open(&db_path).unwrap();
            let draft = ReceiptDraft {
                id: "s1".into(),
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
            };
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
}
