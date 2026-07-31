//! Local receipt attachment store (schema v3 `attachment_path`).
//!
//! Files live next to the ledger: `{db_parent}/attachments/{tx_id}/{filename}`.
//! The DB stores a **relative** path (`attachments/…`) so a whole data dir can
//! move, and so encrypted backups can re-hydrate blobs on another device.
//!
//! Multi-device = user copies `backup.rradar` (or the folder). **No** cloud relay.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AttachmentError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

/// `{parent of ledger.db}/attachments`.
pub fn attachments_root_for_db(db: &Path) -> PathBuf {
    db_parent(db).join("attachments")
}

/// Ensure the attachments root exists; returns it.
pub fn ensure_attachments_root(db: &Path) -> Result<PathBuf, AttachmentError> {
    let root = attachments_root_for_db(db);
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

fn db_parent(db: &Path) -> PathBuf {
    db.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Keep a portable file name (ASCII alnum, `.`, `-`, `_`).
pub fn safe_filename(name: &str) -> String {
    let base = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment.bin");
    let mut out = String::with_capacity(base.len());
    for c in base.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out == "." || out == ".." {
        "attachment.bin".into()
    } else {
        out
    }
}

/// Copy `source` into the local store for `tx_id`.
///
/// Returns the **relative** path to store in `transactions.attachment_path`
/// (always uses `/` separators).
pub fn store_attachment(
    db_path: &Path,
    tx_id: &str,
    source: &Path,
) -> Result<String, AttachmentError> {
    if tx_id.is_empty() || tx_id.contains('/') || tx_id.contains('\\') || tx_id.contains("..") {
        return Err(AttachmentError::Msg("invalid transaction id".into()));
    }
    if !source.is_file() {
        return Err(AttachmentError::Msg(format!(
            "not a file: {}",
            source.display()
        )));
    }
    let root = ensure_attachments_root(db_path)?;
    let dest_dir = root.join(tx_id);
    std::fs::create_dir_all(&dest_dir)?;
    let fname = safe_filename(
        source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment.bin"),
    );
    let dest = dest_dir.join(&fname);
    std::fs::copy(source, &dest)?;
    Ok(format!("attachments/{tx_id}/{fname}"))
}

/// Resolve a stored attachment path (relative or absolute) against the ledger.
pub fn resolve_attachment_path(db_path: &Path, stored: &str) -> PathBuf {
    let p = Path::new(stored);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    // Normalize accidental backslashes from older Windows absolute-ish entries.
    let norm = stored.replace('\\', "/");
    db_parent(db_path).join(norm)
}

/// Delete a stored attachment file (best-effort empty-dir cleanup).
pub fn remove_stored_attachment(db_path: &Path, stored: &str) -> Result<(), AttachmentError> {
    let path = resolve_attachment_path(db_path, stored);
    if path.is_file() {
        std::fs::remove_file(&path)?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent); // only if empty
        }
    }
    Ok(())
}

/// Collect attachment blobs for backup packaging: `(archive_name, bytes)`.
///
/// Archive names always start with `attachments/` and use `/`.
pub fn collect_attachment_files(db_path: &Path) -> Result<Vec<(String, Vec<u8>)>, AttachmentError> {
    let root = attachments_root_for_db(db_path);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    walk_collect(&root, "attachments", &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn walk_collect(
    dir: &Path,
    prefix: &str,
    out: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), AttachmentError> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for ent in entries {
        let path = ent.path();
        let name = ent.file_name().to_string_lossy().to_string();
        // Skip path traversal junk
        if name == "." || name == ".." || name.contains("..") {
            continue;
        }
        let archive_name = format!("{prefix}/{name}");
        if path.is_dir() {
            walk_collect(&path, &archive_name, out)?;
        } else if path.is_file() {
            out.push((archive_name, std::fs::read(&path)?));
        }
    }
    Ok(())
}

/// Write attachment entries from a backup archive next to `db_path`.
///
/// Only names under `attachments/` are accepted.
pub fn write_attachment_files(
    db_path: &Path,
    files: &[(String, Vec<u8>)],
) -> Result<usize, AttachmentError> {
    let base = db_parent(db_path);
    let mut n = 0usize;
    for (name, data) in files {
        let norm = name.replace('\\', "/");
        if !norm.starts_with("attachments/") {
            continue;
        }
        if norm.contains("..") {
            return Err(AttachmentError::Msg(format!(
                "refusing path traversal in backup entry: {norm}"
            )));
        }
        let dest = base.join(&norm);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, data)?;
        n += 1;
    }
    Ok(n)
}

/// Normalize free-form tags: trim, drop empties, join with `,`.
pub fn normalize_tags(raw: &str) -> Option<String> {
    let parts: Vec<String> = raw
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn store_resolve_and_collect() {
        let tmp = std::env::temp_dir().join(format!("rradar-att-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let db = tmp.join("ledger.db");
        std::fs::write(&db, b"sqlite-placeholder").unwrap();

        let src = tmp.join("receipt.png");
        {
            let mut f = std::fs::File::create(&src).unwrap();
            f.write_all(b"\x89PNG fake").unwrap();
        }

        let rel = store_attachment(&db, "tx01", &src).unwrap();
        assert_eq!(rel, "attachments/tx01/receipt.png");
        let abs = resolve_attachment_path(&db, &rel);
        assert!(abs.is_file());
        assert_eq!(std::fs::read(&abs).unwrap(), b"\x89PNG fake");

        let files = collect_attachment_files(&db).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "attachments/tx01/receipt.png");
        assert_eq!(files[0].1, b"\x89PNG fake");

        remove_stored_attachment(&db, &rel).unwrap();
        assert!(!abs.is_file());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn write_from_archive_names() {
        let tmp = std::env::temp_dir().join(format!("rradar-att-w-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let db = tmp.join("ledger.db");
        let files = vec![("attachments/abc/photo.jpg".into(), b"JPEG".to_vec())];
        let n = write_attachment_files(&db, &files).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            std::fs::read(tmp.join("attachments/abc/photo.jpg")).unwrap(),
            b"JPEG"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tags_normalize() {
        assert_eq!(normalize_tags(" a, b ;c ").as_deref(), Some("a,b,c"));
        assert_eq!(normalize_tags("  ,  "), None);
        assert_eq!(normalize_tags(""), None);
    }

    #[test]
    fn safe_name_strips_weird() {
        assert_eq!(safe_filename("my receipt (1).PNG"), "my_receipt__1_.PNG");
        assert_eq!(safe_filename("../../../etc/passwd"), "passwd");
    }
}
