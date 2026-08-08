//! Local receipt attachment store (schema v3 `attachment_path`).
//!
//! Files live next to the ledger: `{db_parent}/attachments/{tx_id}/{filename}`.
//! The DB stores a **relative** path (`attachments/…`) so a whole data dir can
//! move, and so encrypted backups can re-hydrate blobs on another device.
//!
//! Multi-device = user copies `backup.rradar` (or the folder). **No** cloud relay.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AttachmentError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

/// One transaction-owned attachment path captured before a database purge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AttachmentRecord {
    pub transaction_id: String,
    pub path: String,
}

/// Stage associated with a post-commit attachment cleanup issue.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentCleanupOperation {
    ValidatePath,
    CompareIdentity,
    Metadata,
    Canonicalize,
    DeleteFile,
    ReadDirectory,
    RemoveDirectory,
    AcquireCleanupLock,
    RefreshReferences,
    ReleaseCleanupLock,
}

/// Structured cleanup failure. Database purges may still have committed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentCleanupIssue {
    pub transaction_id: String,
    pub path: String,
    pub operation: AttachmentCleanupOperation,
    pub message: String,
}

/// Best-effort filesystem results returned after a committed database purge.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentCleanupReport {
    pub considered: Vec<AttachmentRecord>,
    pub deleted: Vec<AttachmentRecord>,
    pub already_missing: Vec<AttachmentRecord>,
    pub shared_references_skipped: Vec<AttachmentRecord>,
    pub duplicate_candidates_skipped: Vec<AttachmentRecord>,
    pub unsafe_paths_skipped: Vec<AttachmentCleanupIssue>,
    pub cleanup_errors: Vec<AttachmentCleanupIssue>,
    pub empty_dirs_removed: Vec<AttachmentRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedAttachmentPath {
    owner: String,
    filename: String,
    normalized: String,
}

fn normal_component(component: Option<Component<'_>>, label: &str) -> Result<String, String> {
    match component {
        Some(Component::Normal(value)) => value
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("{label} is not valid UTF-8")),
        Some(other) => Err(format!("{label} is not a normal path component: {other:?}")),
        None => Err(format!("missing {label}")),
    }
}

/// Parse exactly `attachments/<transaction-id>/<filename>` using path components.
fn parse_attachment_path(stored: &str) -> Result<ParsedAttachmentPath, String> {
    let normalized = stored.replace('\\', "/");
    let mut components = Path::new(&normalized).components();
    let root = normal_component(components.next(), "attachments root")?;
    let owner = normal_component(components.next(), "transaction id")?;
    let filename = normal_component(components.next(), "filename")?;

    if components.next().is_some() {
        return Err("attachment path has extra components".into());
    }
    if root != "attachments" {
        return Err("attachment path must start with the attachments component".into());
    }

    let canonical_shape = format!("attachments/{owner}/{filename}");
    if normalized != canonical_shape {
        return Err("attachment path must contain exactly three non-empty components".into());
    }

    Ok(ParsedAttachmentPath {
        owner,
        filename,
        normalized: canonical_shape,
    })
}

#[derive(Debug)]
enum FileIdentity {
    Real(same_file::Handle),
    #[allow(dead_code)]
    Synthetic(u64),
}

impl PartialEq for FileIdentity {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Real(left), Self::Real(right)) => left == right,
            (Self::Synthetic(left), Self::Synthetic(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for FileIdentity {}

#[cfg(windows)]
type CanonicalPathIdentity = String;
#[cfg(not(windows))]
type CanonicalPathIdentity = PathBuf;

#[derive(Debug)]
struct ResolvedAttachment {
    canonical_root: PathBuf,
    canonical_owner: PathBuf,
    canonical_file: PathBuf,
    root_identity: FileIdentity,
    owner_identity: FileIdentity,
    path_identity: CanonicalPathIdentity,
    file_identity: FileIdentity,
}

#[derive(Debug)]
struct ResolutionFailure {
    issue: AttachmentCleanupIssue,
    missing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

trait CleanupFileSystem {
    fn entry_kind(&self, path: &Path) -> io::Result<EntryKind>;
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn file_identity(&self, path: &Path) -> io::Result<FileIdentity>;
    fn same_file(&self, left: &Path, right: &Path) -> io::Result<bool>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn directory_is_empty(&self, path: &Path) -> io::Result<bool>;
    fn remove_dir(&self, path: &Path) -> io::Result<()>;
}

struct StdCleanupFileSystem;

impl CleanupFileSystem for StdCleanupFileSystem {
    fn entry_kind(&self, path: &Path) -> io::Result<EntryKind> {
        let metadata = std::fs::symlink_metadata(path)?;
        let file_type = metadata.file_type();
        Ok(if file_type.is_symlink() {
            EntryKind::Symlink
        } else if file_type.is_file() {
            EntryKind::File
        } else if file_type.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::Other
        })
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::canonicalize(path)
    }

    fn file_identity(&self, path: &Path) -> io::Result<FileIdentity> {
        same_file::Handle::from_path(path).map(FileIdentity::Real)
    }

    fn same_file(&self, left: &Path, right: &Path) -> io::Result<bool> {
        same_file::is_same_file(left, right)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn directory_is_empty(&self, path: &Path) -> io::Result<bool> {
        let mut entries = std::fs::read_dir(path)?;
        match entries.next() {
            Some(entry) => {
                entry?;
                Ok(false)
            }
            None => Ok(true),
        }
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir(path)
    }
}

fn cleanup_issue(
    record: &AttachmentRecord,
    operation: AttachmentCleanupOperation,
    message: impl Into<String>,
) -> AttachmentCleanupIssue {
    AttachmentCleanupIssue {
        transaction_id: record.transaction_id.clone(),
        path: record.path.clone(),
        operation,
        message: message.into(),
    }
}

fn resolution_failure(
    record: &AttachmentRecord,
    operation: AttachmentCleanupOperation,
    message: impl Into<String>,
    missing: bool,
) -> ResolutionFailure {
    ResolutionFailure {
        issue: cleanup_issue(record, operation, message),
        missing,
    }
}

fn require_entry_kind<F: CleanupFileSystem>(
    fs: &F,
    path: &Path,
    expected: EntryKind,
    label: &str,
    record: &AttachmentRecord,
) -> Result<(), ResolutionFailure> {
    match fs.entry_kind(path) {
        Ok(kind) if kind == expected => Ok(()),
        Ok(EntryKind::Symlink) => Err(resolution_failure(
            record,
            AttachmentCleanupOperation::ValidatePath,
            format!("{label} is a symbolic link: {}", path.display()),
            false,
        )),
        Ok(kind) => Err(resolution_failure(
            record,
            AttachmentCleanupOperation::Metadata,
            format!(
                "{label} has the wrong filesystem type ({kind:?}): {}",
                path.display()
            ),
            false,
        )),
        Err(error) => Err(resolution_failure(
            record,
            AttachmentCleanupOperation::Metadata,
            format!("{}: {error}", path.display()),
            error.kind() == io::ErrorKind::NotFound,
        )),
    }
}

fn canonicalize_required<F: CleanupFileSystem>(
    fs: &F,
    path: &Path,
    record: &AttachmentRecord,
) -> Result<PathBuf, ResolutionFailure> {
    fs.canonicalize(path).map_err(|error| {
        resolution_failure(
            record,
            AttachmentCleanupOperation::Canonicalize,
            format!("{}: {error}", path.display()),
            false,
        )
    })
}

fn identity_required<F: CleanupFileSystem>(
    fs: &F,
    path: &Path,
    record: &AttachmentRecord,
) -> Result<FileIdentity, ResolutionFailure> {
    fs.file_identity(path).map_err(|error| {
        resolution_failure(
            record,
            AttachmentCleanupOperation::Metadata,
            format!("cannot identify {}: {error}", path.display()),
            false,
        )
    })
}

#[cfg(windows)]
fn canonical_path_identity(path: &Path) -> CanonicalPathIdentity {
    // `canonicalize` resolves DOS aliases and trailing-dot/space spellings.
    // Unicode folding handles the remaining case-insensitive spelling aliases;
    // filesystem IDs below provide a second, authoritative comparison.
    path.to_string_lossy().to_lowercase()
}

#[cfg(not(windows))]
fn canonical_path_identity(path: &Path) -> CanonicalPathIdentity {
    // Preserve case on Unix so case-sensitive macOS/Linux volumes do not merge
    // distinct attachment paths. Case-insensitive volumes canonicalize aliases
    // to the filesystem's actual spelling.
    path.to_path_buf()
}

fn resolve_for_cleanup<F: CleanupFileSystem>(
    fs: &F,
    db_path: &Path,
    record: &AttachmentRecord,
    parsed: &ParsedAttachmentPath,
) -> Result<ResolvedAttachment, ResolutionFailure> {
    let base = db_parent(db_path);
    let canonical_base = canonicalize_required(fs, &base, record)?;

    let root = attachments_root_for_db(db_path);
    require_entry_kind(fs, &root, EntryKind::Directory, "attachments root", record)?;
    let canonical_root = canonicalize_required(fs, &root, record)?;
    if canonical_root.parent() != Some(canonical_base.as_path()) {
        return Err(resolution_failure(
            record,
            AttachmentCleanupOperation::ValidatePath,
            "attachments root resolves outside the ledger directory",
            false,
        ));
    }
    let root_identity = identity_required(fs, &canonical_root, record)?;

    let owner_dir = root.join(&parsed.owner);
    require_entry_kind(
        fs,
        &owner_dir,
        EntryKind::Directory,
        "transaction directory",
        record,
    )?;
    let canonical_owner = canonicalize_required(fs, &owner_dir, record)?;
    if canonical_owner.parent() != Some(canonical_root.as_path()) {
        return Err(resolution_failure(
            record,
            AttachmentCleanupOperation::ValidatePath,
            "transaction directory escapes the attachments root",
            false,
        ));
    }
    let owner_identity = identity_required(fs, &canonical_owner, record)?;

    let file_path = owner_dir.join(&parsed.filename);
    require_entry_kind(fs, &file_path, EntryKind::File, "attachment file", record)?;
    let canonical_file = canonicalize_required(fs, &file_path, record)?;
    if canonical_file.parent() != Some(canonical_owner.as_path()) {
        return Err(resolution_failure(
            record,
            AttachmentCleanupOperation::ValidatePath,
            "attachment file escapes its transaction directory",
            false,
        ));
    }
    let file_identity = identity_required(fs, &canonical_file, record)?;

    Ok(ResolvedAttachment {
        canonical_root,
        canonical_owner,
        path_identity: canonical_path_identity(&canonical_file),
        canonical_file,
        root_identity,
        owner_identity,
        file_identity,
    })
}

fn revalidate_entry<F: CleanupFileSystem>(
    fs: &F,
    path: &Path,
    expected_kind: EntryKind,
    expected_identity: &FileIdentity,
    label: &str,
    record: &AttachmentRecord,
    report: &mut AttachmentCleanupReport,
) -> bool {
    if let Err(failure) = require_entry_kind(fs, path, expected_kind, label, record) {
        report.cleanup_errors.push(failure.issue);
        return false;
    }
    let current_path = match canonicalize_required(fs, path, record) {
        Ok(path) => path,
        Err(failure) => {
            report.cleanup_errors.push(failure.issue);
            return false;
        }
    };
    if current_path != path {
        report.unsafe_paths_skipped.push(cleanup_issue(
            record,
            AttachmentCleanupOperation::ValidatePath,
            format!("{label} changed after validation"),
        ));
        return false;
    }
    match identity_required(fs, path, record) {
        Ok(identity) if &identity == expected_identity => true,
        Ok(_) => {
            report.unsafe_paths_skipped.push(cleanup_issue(
                record,
                AttachmentCleanupOperation::ValidatePath,
                format!("{label} identity changed after validation"),
            ));
            false
        }
        Err(failure) => {
            report.cleanup_errors.push(failure.issue);
            false
        }
    }
}

fn cleanup_resolved<F: CleanupFileSystem>(
    fs: &F,
    record: &AttachmentRecord,
    resolved: ResolvedAttachment,
    report: &mut AttachmentCleanupReport,
) {
    let ResolvedAttachment {
        canonical_root,
        canonical_owner,
        canonical_file,
        root_identity,
        owner_identity,
        path_identity: _,
        file_identity,
    } = resolved;
    // Revalidate the exact canonical chain immediately before deletion and
    // delete that canonical file, never a path reconstructed from DB text.
    if !revalidate_entry(
        fs,
        &canonical_root,
        EntryKind::Directory,
        &root_identity,
        "attachments root",
        record,
        report,
    ) || !revalidate_entry(
        fs,
        &canonical_owner,
        EntryKind::Directory,
        &owner_identity,
        "transaction directory",
        record,
        report,
    ) || !revalidate_entry(
        fs,
        &canonical_file,
        EntryKind::File,
        &file_identity,
        "attachment file",
        record,
        report,
    ) {
        return;
    }

    // Release the identity handle before unlinking; some Windows filesystems
    // otherwise keep the directory entry alive until the handle closes.
    drop(file_identity);
    if let Err(error) = fs.remove_file(&canonical_file) {
        report.cleanup_errors.push(cleanup_issue(
            record,
            AttachmentCleanupOperation::DeleteFile,
            error.to_string(),
        ));
        return;
    }
    report.deleted.push(record.clone());

    // Only the validated transaction directory may be removed. The root and
    // all parent directories are intentionally left untouched.
    if !revalidate_entry(
        fs,
        &canonical_root,
        EntryKind::Directory,
        &root_identity,
        "attachments root",
        record,
        report,
    ) || !revalidate_entry(
        fs,
        &canonical_owner,
        EntryKind::Directory,
        &owner_identity,
        "transaction directory",
        record,
        report,
    ) {
        return;
    }
    match fs.directory_is_empty(&canonical_owner) {
        Ok(false) => {}
        Ok(true) => {
            drop(owner_identity);
            match fs.remove_dir(&canonical_owner) {
                Ok(()) => report.empty_dirs_removed.push(record.clone()),
                Err(error) => report.cleanup_errors.push(cleanup_issue(
                    record,
                    AttachmentCleanupOperation::RemoveDirectory,
                    error.to_string(),
                )),
            }
        }
        Err(error) => report.cleanup_errors.push(cleanup_issue(
            record,
            AttachmentCleanupOperation::ReadDirectory,
            error.to_string(),
        )),
    }
}

fn cleanup_purged_attachments_with_fs<F: CleanupFileSystem>(
    fs: &F,
    db_path: &Path,
    candidates: Vec<AttachmentRecord>,
    remaining_references: Vec<AttachmentRecord>,
) -> AttachmentCleanupReport {
    let mut report = AttachmentCleanupReport::default();
    let mut invalid_seen = HashSet::new();
    let mut paths_seen = HashSet::new();
    let mut prepared = Vec::new();

    for record in candidates {
        let parsed = match parse_attachment_path(&record.path) {
            Ok(parsed) => parsed,
            Err(message) => {
                if invalid_seen.insert((record.transaction_id.clone(), record.path.clone())) {
                    report.considered.push(record.clone());
                    report.unsafe_paths_skipped.push(cleanup_issue(
                        &record,
                        AttachmentCleanupOperation::ValidatePath,
                        message,
                    ));
                }
                continue;
            }
        };
        if parsed.owner != record.transaction_id {
            report.considered.push(record.clone());
            report.unsafe_paths_skipped.push(cleanup_issue(
                &record,
                AttachmentCleanupOperation::ValidatePath,
                format!(
                    "path owner {} does not match transaction {}",
                    parsed.owner, record.transaction_id
                ),
            ));
            continue;
        }
        report.considered.push(record.clone());
        if paths_seen.insert(parsed.normalized.clone()) {
            prepared.push((record, parsed));
        } else {
            report.duplicate_candidates_skipped.push(record);
        }
    }

    let mut remaining_paths = HashSet::new();
    let mut parsed_remaining = Vec::new();
    let mut unsafe_comparison = false;
    for reference in remaining_references {
        match parse_attachment_path(&reference.path) {
            Ok(parsed) => {
                if remaining_paths.insert(parsed.normalized.clone()) {
                    parsed_remaining.push((reference, parsed));
                }
            }
            Err(message) => {
                unsafe_comparison = true;
                report.cleanup_errors.push(cleanup_issue(
                    &reference,
                    AttachmentCleanupOperation::CompareIdentity,
                    message,
                ));
            }
        }
    }

    if db_path == Path::new(":memory:") || db_path.as_os_str().is_empty() {
        for (record, _) in prepared {
            report.unsafe_paths_skipped.push(cleanup_issue(
                &record,
                AttachmentCleanupOperation::ValidatePath,
                "attachment cleanup is unavailable for an in-memory ledger",
            ));
        }
        return report;
    }

    let mut pending_candidates = Vec::new();
    for (record, parsed) in prepared {
        if remaining_paths.contains(&parsed.normalized) {
            report.shared_references_skipped.push(record);
            continue;
        }
        pending_candidates.push((record, parsed));
    }

    let mut resolved_remaining = Vec::new();
    for (reference, parsed) in parsed_remaining {
        match resolve_for_cleanup(fs, db_path, &reference, &parsed) {
            // Only the canonical path identity is needed after validation.
            // Surviving references only need their canonical identity after
            // validation, so their filesystem handles are released here.
            Ok(resolved) => {
                resolved_remaining.push((resolved.path_identity, resolved.canonical_file))
            }
            Err(failure) => {
                unsafe_comparison = true;
                report.cleanup_errors.push(failure.issue);
            }
        }
    }
    if unsafe_comparison {
        for (record, _) in pending_candidates {
            report.cleanup_errors.push(cleanup_issue(
                &record,
                AttachmentCleanupOperation::CompareIdentity,
                "cleanup skipped because a surviving attachment identity could not be validated",
            ));
        }
        return report;
    }
    let mut candidate_paths_seen = HashSet::new();
    let mut unique_candidates = Vec::new();
    for (record, parsed) in pending_candidates {
        let resolved = match resolve_for_cleanup(fs, db_path, &record, &parsed) {
            Ok(resolved) => resolved,
            Err(failure) if failure.missing => {
                report.already_missing.push(record);
                continue;
            }
            Err(failure) if failure.issue.operation == AttachmentCleanupOperation::ValidatePath => {
                report.unsafe_paths_skipped.push(failure.issue);
                continue;
            }
            Err(failure) => {
                report.cleanup_errors.push(failure.issue);
                continue;
            }
        };
        if !candidate_paths_seen.insert(resolved.path_identity.clone()) {
            report.duplicate_candidates_skipped.push(record);
            continue;
        }
        let mut shared = false;
        let mut comparison_error = None;
        for (remaining_identity, remaining_file) in &resolved_remaining {
            if remaining_identity == &resolved.path_identity {
                shared = true;
                break;
            }
            match fs.same_file(&resolved.canonical_file, remaining_file) {
                Ok(true) => {
                    shared = true;
                    break;
                }
                Ok(false) => {}
                Err(error) => {
                    comparison_error = Some(error);
                    break;
                }
            }
        }
        if let Some(error) = comparison_error {
            report.cleanup_errors.push(cleanup_issue(
                &record,
                AttachmentCleanupOperation::CompareIdentity,
                error.to_string(),
            ));
        } else if shared {
            report.shared_references_skipped.push(record);
        } else {
            let identity = resolved.path_identity;
            let file_identity = resolved.file_identity;
            unique_candidates.push((record, parsed, identity, file_identity));
        }
    }

    // Resolve again only after the entire batch has been canonicalized and
    // deduplicated. This prevents the first alias from being deleted before a
    // later alias can be recognized. One file handle per unique candidate is
    // deliberately retained so replacements cannot inherit a validated path.
    for (record, parsed, expected_path_identity, expected_file_identity) in unique_candidates {
        match resolve_for_cleanup(fs, db_path, &record, &parsed) {
            Ok(resolved)
                if resolved.path_identity == expected_path_identity
                    && resolved.file_identity == expected_file_identity =>
            {
                drop(expected_file_identity);
                cleanup_resolved(fs, &record, resolved, &mut report);
            }
            Ok(_) => report.unsafe_paths_skipped.push(cleanup_issue(
                &record,
                AttachmentCleanupOperation::ValidatePath,
                "attachment identity changed after batch validation",
            )),
            Err(failure) if failure.issue.operation == AttachmentCleanupOperation::ValidatePath => {
                report.unsafe_paths_skipped.push(failure.issue);
            }
            Err(failure) => report.cleanup_errors.push(failure.issue),
        }
    }
    report
}

/// Delete deduplicated, unreferenced attachment files after a committed purge.
pub(crate) fn cleanup_purged_attachments(
    db_path: &Path,
    candidates: Vec<AttachmentRecord>,
    remaining_references: Vec<AttachmentRecord>,
) -> AttachmentCleanupReport {
    cleanup_purged_attachments_with_fs(
        &StdCleanupFileSystem,
        db_path,
        candidates,
        remaining_references,
    )
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
    if !source.is_file() {
        return Err(AttachmentError::Msg(format!(
            "not a file: {}",
            source.display()
        )));
    }
    let fname = safe_filename(
        source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment.bin"),
    );
    let bytes = std::fs::read(source)?;
    store_attachment_bytes(db_path, tx_id, &fname, &bytes)
}

/// Write raw bytes into the local store (mobile camera / in-memory capture).
///
/// `filename` is sanitized; defaults to `capture.bin` when empty.
pub fn store_attachment_bytes(
    db_path: &Path,
    tx_id: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<String, AttachmentError> {
    if tx_id.is_empty() || tx_id.contains('/') || tx_id.contains('\\') || tx_id.contains("..") {
        return Err(AttachmentError::Msg("invalid transaction id".into()));
    }
    if bytes.is_empty() {
        return Err(AttachmentError::Msg("empty attachment bytes".into()));
    }
    let root = ensure_attachments_root(db_path)?;
    let dest_dir = root.join(tx_id);
    std::fs::create_dir_all(&dest_dir)?;
    let fname = if filename.trim().is_empty() {
        "capture.bin".into()
    } else {
        safe_filename(filename)
    };
    let dest = dest_dir.join(&fname);
    std::fs::write(&dest, bytes)?;
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

    fn temp_case(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rradar-attachment-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn record(transaction_id: &str, path: &str) -> AttachmentRecord {
        AttachmentRecord {
            transaction_id: transaction_id.into(),
            path: path.into(),
        }
    }

    #[derive(Clone, Copy)]
    struct FaultFileSystem {
        fail: Option<AttachmentCleanupOperation>,
    }

    impl CleanupFileSystem for FaultFileSystem {
        fn entry_kind(&self, path: &Path) -> io::Result<EntryKind> {
            if self.fail == Some(AttachmentCleanupOperation::Metadata) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "metadata"));
            }
            if path.file_name().and_then(|name| name.to_str()) == Some("receipt.jpg") {
                Ok(EntryKind::File)
            } else {
                Ok(EntryKind::Directory)
            }
        }

        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
            if self.fail == Some(AttachmentCleanupOperation::Canonicalize) {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "canonicalize",
                ))
            } else {
                Ok(path.to_path_buf())
            }
        }

        fn file_identity(&self, path: &Path) -> io::Result<FileIdentity> {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            let file = match name {
                "attachments" => 1,
                "tx01" => 2,
                _ => 3,
            };
            Ok(FileIdentity::Synthetic(file))
        }

        fn same_file(&self, left: &Path, right: &Path) -> io::Result<bool> {
            Ok(left == right)
        }

        fn remove_file(&self, _path: &Path) -> io::Result<()> {
            if self.fail == Some(AttachmentCleanupOperation::DeleteFile) {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "delete"))
            } else {
                Ok(())
            }
        }

        fn directory_is_empty(&self, _path: &Path) -> io::Result<bool> {
            if self.fail == Some(AttachmentCleanupOperation::ReadDirectory) {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "read_dir"))
            } else {
                Ok(true)
            }
        }

        fn remove_dir(&self, _path: &Path) -> io::Result<()> {
            if self.fail == Some(AttachmentCleanupOperation::RemoveDirectory) {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "remove_dir",
                ))
            } else {
                Ok(())
            }
        }
    }

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

        let rel2 = store_attachment_bytes(&db, "tx02", "cam.jpg", b"JPEGDATA").unwrap();
        assert_eq!(rel2, "attachments/tx02/cam.jpg");
        assert_eq!(
            std::fs::read(resolve_attachment_path(&db, &rel2)).unwrap(),
            b"JPEGDATA"
        );

        let files = collect_attachment_files(&db).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.0 == "attachments/tx01/receipt.png"));
        assert!(files.iter().any(|f| f.0 == "attachments/tx02/cam.jpg"));

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
    fn purge_paths_require_exact_components() {
        let parsed = parse_attachment_path("attachments/tx01/receipt..jpg").unwrap();
        assert_eq!(parsed.owner, "tx01");
        assert_eq!(parsed.filename, "receipt..jpg");
        assert!(parse_attachment_path(r"attachments\tx01\receipt.jpg").is_ok());

        for unsafe_path in [
            "/attachments/tx01/receipt.jpg",
            r"C:\attachments\tx01\receipt.jpg",
            "other/tx01/receipt.jpg",
            "attachments/../tx01/receipt.jpg",
            "attachments/./tx01/receipt.jpg",
            "attachments//tx01/receipt.jpg",
            "attachments/tx01",
            "attachments/tx01/",
            "attachments/tx01/nested/receipt.jpg",
            "attachments/receipt.jpg",
        ] {
            assert!(
                parse_attachment_path(unsafe_path).is_err(),
                "accepted {unsafe_path}"
            );
        }
    }

    #[test]
    fn cleanup_validates_owner_and_never_removes_root() {
        let tmp = temp_case("owner");
        let db = tmp.join("ledger.db");
        std::fs::create_dir_all(tmp.join("attachments/owner-b")).unwrap();
        let file = tmp.join("attachments/owner-b/receipt.jpg");
        std::fs::write(&file, b"receipt").unwrap();

        let report = cleanup_purged_attachments(
            &db,
            vec![record("owner-a", "attachments/owner-b/receipt.jpg")],
            Vec::new(),
        );
        assert_eq!(report.considered.len(), 1);
        assert_eq!(report.unsafe_paths_skipped.len(), 1);
        assert!(file.is_file());
        assert!(tmp.join("attachments").is_dir());
        assert!(report.empty_dirs_removed.is_empty());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cleanup_normalizes_shared_references_and_deduplicates_candidates() {
        let tmp = temp_case("identity");
        std::fs::create_dir_all(&tmp).unwrap();
        let db = tmp.join("ledger.db");
        let stored = store_attachment_bytes(&db, "tx01", "receipt.jpg", b"receipt").unwrap();
        let backslash = stored.replace('/', "\\");

        let shared = cleanup_purged_attachments(
            &db,
            vec![record("tx01", &stored), record("tx01", &backslash)],
            vec![record("other", &backslash)],
        );
        assert_eq!(shared.considered.len(), 2);
        assert_eq!(shared.duplicate_candidates_skipped.len(), 1);
        assert_eq!(shared.shared_references_skipped.len(), 1);
        assert!(resolve_attachment_path(&db, &stored).is_file());

        let deleted = cleanup_purged_attachments(
            &db,
            vec![record("tx01", &stored), record("tx01", &backslash)],
            Vec::new(),
        );
        assert_eq!(deleted.considered.len(), 2);
        assert_eq!(deleted.duplicate_candidates_skipped.len(), 1);
        assert_eq!(deleted.deleted.len(), 1);
        assert!(deleted.already_missing.is_empty());
        assert_eq!(deleted.empty_dirs_removed.len(), 1);
        assert!(tmp.join("attachments").is_dir());
        assert!(!tmp.join("attachments/tx01").exists());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cleanup_only_removes_the_exact_empty_owner_directory() {
        let tmp = temp_case("directory");
        std::fs::create_dir_all(&tmp).unwrap();
        let db = tmp.join("ledger.db");
        let stored = store_attachment_bytes(&db, "tx01", "receipt.jpg", b"receipt").unwrap();
        std::fs::write(tmp.join("attachments/tx01/keep.txt"), b"keep").unwrap();

        let report = cleanup_purged_attachments(&db, vec![record("tx01", &stored)], Vec::new());
        assert_eq!(report.deleted.len(), 1);
        assert!(report.empty_dirs_removed.is_empty());
        assert!(tmp.join("attachments/tx01/keep.txt").is_file());
        assert!(tmp.join("attachments").is_dir());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cleanup_reports_missing_file_without_removing_directories() {
        let tmp = temp_case("missing");
        let db = tmp.join("ledger.db");
        std::fs::create_dir_all(tmp.join("attachments/tx01")).unwrap();
        let report = cleanup_purged_attachments(
            &db,
            vec![record("tx01", "attachments/tx01/missing.jpg")],
            Vec::new(),
        );
        assert_eq!(report.already_missing.len(), 1);
        assert!(report.deleted.is_empty());
        assert!(tmp.join("attachments/tx01").is_dir());
        assert!(tmp.join("attachments").is_dir());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cleanup_records_every_filesystem_failure_stage() {
        let db = temp_case("faults").join("ledger.db");
        for operation in [
            AttachmentCleanupOperation::Metadata,
            AttachmentCleanupOperation::Canonicalize,
            AttachmentCleanupOperation::DeleteFile,
            AttachmentCleanupOperation::ReadDirectory,
            AttachmentCleanupOperation::RemoveDirectory,
        ] {
            let report = cleanup_purged_attachments_with_fs(
                &FaultFileSystem {
                    fail: Some(operation),
                },
                &db,
                vec![record("tx01", "attachments/tx01/receipt.jpg")],
                Vec::new(),
            );
            assert!(
                report
                    .cleanup_errors
                    .iter()
                    .any(|issue| issue.operation == operation),
                "missing {operation:?}: {report:?}"
            );
        }
    }

    #[test]
    fn cleanup_rejects_a_candidate_replaced_between_batch_validation_and_delete() {
        use std::cell::Cell;

        struct ReplacingFileSystem {
            file_identity: Cell<u64>,
        }

        impl CleanupFileSystem for ReplacingFileSystem {
            fn entry_kind(&self, path: &Path) -> io::Result<EntryKind> {
                if path.file_name().and_then(|name| name.to_str()) == Some("receipt.jpg") {
                    Ok(EntryKind::File)
                } else {
                    Ok(EntryKind::Directory)
                }
            }

            fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
                Ok(path.to_path_buf())
            }

            fn file_identity(&self, path: &Path) -> io::Result<FileIdentity> {
                if path.file_name().and_then(|name| name.to_str()) == Some("receipt.jpg") {
                    let next = self.file_identity.get() + 1;
                    self.file_identity.set(next);
                    Ok(FileIdentity::Synthetic(next))
                } else {
                    Ok(FileIdentity::Synthetic(0))
                }
            }

            fn same_file(&self, left: &Path, right: &Path) -> io::Result<bool> {
                Ok(left == right)
            }

            fn remove_file(&self, _path: &Path) -> io::Result<()> {
                panic!("a replaced attachment must not be deleted")
            }

            fn directory_is_empty(&self, _path: &Path) -> io::Result<bool> {
                Ok(true)
            }

            fn remove_dir(&self, _path: &Path) -> io::Result<()> {
                Ok(())
            }
        }

        let report = cleanup_purged_attachments_with_fs(
            &ReplacingFileSystem {
                file_identity: Cell::new(0),
            },
            &temp_case("replaced").join("ledger.db"),
            vec![record("tx01", "attachments/tx01/receipt.jpg")],
            Vec::new(),
        );
        assert_eq!(report.unsafe_paths_skipped.len(), 1, "{report:?}");
        assert!(report.deleted.is_empty());
    }

    #[test]
    fn malformed_remaining_reference_fails_closed() {
        let tmp = temp_case("remaining");
        std::fs::create_dir_all(&tmp).unwrap();
        let db = tmp.join("ledger.db");
        let stored = store_attachment_bytes(&db, "tx01", "receipt.jpg", b"receipt").unwrap();
        let report = cleanup_purged_attachments(
            &db,
            vec![record("tx01", &stored)],
            vec![record("other", "attachments/../tx01/receipt.jpg")],
        );
        assert!(report.cleanup_errors.iter().any(|issue| {
            issue.transaction_id == "tx01"
                && issue.operation == AttachmentCleanupOperation::CompareIdentity
        }));
        assert!(resolve_attachment_path(&db, &stored).is_file());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_keeps_case_distinct_candidates_on_case_sensitive_filesystems() {
        let tmp = temp_case("case-sensitive");
        let db = tmp.join("ledger.db");
        let upper = store_attachment_bytes(&db, "Tx", "receipt.jpg", b"upper").unwrap();
        let lower = store_attachment_bytes(&db, "tx", "receipt.jpg", b"lower").unwrap();
        let upper_file = resolve_attachment_path(&db, &upper);
        let lower_file = resolve_attachment_path(&db, &lower);
        if std::fs::canonicalize(&upper_file).unwrap()
            == std::fs::canonicalize(&lower_file).unwrap()
        {
            std::fs::remove_dir_all(&tmp).unwrap();
            return;
        }

        let report = cleanup_purged_attachments(
            &db,
            vec![record("Tx", &upper), record("tx", &lower)],
            Vec::new(),
        );
        assert_eq!(report.deleted.len(), 2, "{report:?}");
        assert!(report.duplicate_candidates_skipped.is_empty());
        assert!(!upper_file.exists());
        assert!(!lower_file.exists());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn cleanup_protects_windows_case_and_trailing_dot_aliases() {
        let tmp = temp_case("windows-alias");
        std::fs::create_dir_all(&tmp).unwrap();
        let db = tmp.join("ledger.db");
        let stored = store_attachment_bytes(&db, "CaseTx", "receipt.jpg", b"receipt").unwrap();
        let report = cleanup_purged_attachments(
            &db,
            vec![record("CaseTx", &stored)],
            vec![record("survivor", "attachments/casetx/RECEIPT.JPG.")],
        );
        assert_eq!(report.shared_references_skipped.len(), 1, "{report:?}");
        assert!(resolve_attachment_path(&db, &stored).is_file());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn cleanup_deduplicates_windows_canonical_candidate_aliases_before_delete() {
        let tmp = temp_case("windows-candidate-alias");
        std::fs::create_dir_all(&tmp).unwrap();
        let db = tmp.join("ledger.db");
        let stored = store_attachment_bytes(&db, "CaseTx", "receipt.jpg", b"receipt").unwrap();
        let report = cleanup_purged_attachments(
            &db,
            vec![
                record("CaseTx", &stored),
                record("casetx", "attachments/casetx/RECEIPT.JPG."),
            ],
            Vec::new(),
        );
        assert_eq!(report.deleted.len(), 1, "{report:?}");
        assert_eq!(report.duplicate_candidates_skipped.len(), 1, "{report:?}");
        assert!(report.already_missing.is_empty(), "{report:?}");
        assert!(!resolve_attachment_path(&db, &stored).exists());
        assert!(tmp.join("attachments").is_dir());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_conservatively_protects_a_surviving_hard_link_identity() {
        let tmp = temp_case("hard-link");
        let db = tmp.join("ledger.db");
        let stored = store_attachment_bytes(&db, "tx01", "receipt.jpg", b"receipt").unwrap();
        std::fs::create_dir_all(tmp.join("attachments/survivor")).unwrap();
        let survivor = "attachments/survivor/receipt-link.jpg";
        std::fs::hard_link(
            resolve_attachment_path(&db, &stored),
            resolve_attachment_path(&db, survivor),
        )
        .unwrap();

        let report = cleanup_purged_attachments(
            &db,
            vec![record("tx01", &stored)],
            vec![record("survivor-row", survivor)],
        );
        assert_eq!(report.shared_references_skipped.len(), 1, "{report:?}");
        assert!(resolve_attachment_path(&db, &stored).is_file());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_rejects_symlinked_attachment_file() {
        use std::os::unix::fs::symlink;

        let tmp = temp_case("symlink");
        let outside = temp_case("outside");
        std::fs::create_dir_all(tmp.join("attachments/tx01")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let target = outside.join("receipt.jpg");
        std::fs::write(&target, b"outside").unwrap();
        symlink(&target, tmp.join("attachments/tx01/receipt.jpg")).unwrap();
        let db = tmp.join("ledger.db");

        let report = cleanup_purged_attachments(
            &db,
            vec![record("tx01", "attachments/tx01/receipt.jpg")],
            Vec::new(),
        );
        assert_eq!(report.unsafe_paths_skipped.len(), 1);
        assert_eq!(std::fs::read(&target).unwrap(), b"outside");
        std::fs::remove_dir_all(&tmp).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
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
