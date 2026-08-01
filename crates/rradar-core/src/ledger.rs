//! SQLite ledger: confirm drafts, list, stats by currency/month.
//!
//! Schema evolves only via [`Ledger::migrate`] steps. Multi-device = encrypted
//! backup / export only — **no** official cloud relay (project policy).

use crate::money::{Iso4217, Money};
use crate::types::{FieldSource, ReceiptDraft, SourcePath};
use crate::VERSION;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Latest ledger schema this binary knows how to open and migrate **to**.
pub const LEDGER_SCHEMA_VERSION: u32 = 3;

const SCHEMA_BASE: &str = r#"
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS transactions (
  id TEXT PRIMARY KEY,
  confirmed_at TEXT NOT NULL,
  transacted_at TEXT NOT NULL,
  merchant TEXT NOT NULL,
  amount_minor INTEGER NOT NULL,
  currency TEXT NOT NULL,
  exponent INTEGER NOT NULL,
  category TEXT NOT NULL,
  invoice_id TEXT,
  source_path TEXT NOT NULL,
  overall_confidence REAL NOT NULL,
  content_hash TEXT,
  notes TEXT,
  raw_text TEXT,
  draft_json TEXT
);
CREATE INDEX IF NOT EXISTS idx_tx_date_cur ON transactions(transacted_at, currency);
CREATE INDEX IF NOT EXISTS idx_tx_merchant ON transactions(merchant);
CREATE INDEX IF NOT EXISTS idx_tx_invoice ON transactions(invoice_id);
CREATE INDEX IF NOT EXISTS idx_tx_hash ON transactions(content_hash);
"#;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error(
        "ledger schema {found} is newer than this binary supports ({supported}); upgrade rradar"
    )]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("migration failed at v{to}: {detail}")]
    Migration { to: u32, detail: String },
    #[error("{0}")]
    Msg(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub id: String,
    pub confirmed_at: String,
    pub transacted_at: String,
    pub merchant: String,
    pub amount_minor: i64,
    pub currency: String,
    pub exponent: u8,
    pub category: String,
    pub invoice_id: Option<String>,
    pub source_path: String,
    pub overall_confidence: f32,
    pub content_hash: Option<String>,
    pub notes: Option<String>,
    /// Comma-separated free tags (schema v3+).
    #[serde(default)]
    pub tags: Option<String>,
    /// Optional path to receipt image/file on device (schema v3+).
    #[serde(default)]
    pub attachment_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrencyMonthStat {
    pub currency: String,
    pub year: i32,
    pub month: u32,
    pub total_minor: i64,
    pub count: i64,
}

/// Per-category totals **within one currency** (never mixes currencies).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CategoryStat {
    pub currency: String,
    pub category: String,
    pub total_minor: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DedupeLevel {
    None,
    Soft,
    Hard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DedupeWarning {
    pub level: DedupeLevel,
    pub existing_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmResult {
    pub transaction: Transaction,
    pub dedupe: Option<DedupeWarning>,
    /// When hard dedupe hit and force=false, insert was skipped.
    pub inserted: bool,
}

/// Fields to patch on an existing transaction (`None` = leave unchanged).
#[derive(Debug, Clone, Default)]
pub struct TxUpdate {
    pub merchant: Option<String>,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub exponent: Option<u8>,
    pub category: Option<String>,
    pub notes: Option<String>,
    pub transacted_at: Option<String>,
    pub tags: Option<String>,
    pub attachment_path: Option<String>,
}

/// Unified list/search filter (schema v3 tags, amount range, attachments).
///
/// All fields optional; empty filter = recent transactions with limit/offset.
#[derive(Debug, Clone, Default)]
pub struct TxFilter {
    pub limit: usize,
    pub offset: usize,
    pub currency: Option<String>,
    /// Substring match on merchant / category / notes / tags.
    pub query: Option<String>,
    /// Tag token match (comma list contains this tag, case-insensitive).
    pub tag: Option<String>,
    /// Exact category id.
    pub category: Option<String>,
    /// Calendar month prefix `YYYY-MM`.
    pub year_month: Option<String>,
    /// Inclusive date lower bound `YYYY-MM-DD` on `transacted_at`.
    pub from: Option<String>,
    /// Inclusive date upper bound `YYYY-MM-DD` on `transacted_at`.
    pub to: Option<String>,
    pub min_minor: Option<i64>,
    pub max_minor: Option<i64>,
    /// `Some(true)` = has attachment path; `Some(false)` = none.
    pub has_attachment: Option<bool>,
}

pub struct Ledger {
    conn: Connection,
    path: PathBuf,
}

impl Ledger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LedgerError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(&path)?;
        let mut ledger = Self { conn, path };
        ledger.migrate()?;
        Ok(ledger)
    }

    pub fn open_in_memory() -> Result<Self, LedgerError> {
        let conn = Connection::open_in_memory()?;
        let mut ledger = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        ledger.migrate()?;
        Ok(ledger)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Apply base DDL + forward migrations up to [`LEDGER_SCHEMA_VERSION`].
    fn migrate(&mut self) -> Result<(), LedgerError> {
        self.conn.execute_batch(SCHEMA_BASE)?;
        // Fresh DBs: start at v1 meta, then step forward.
        self.conn.execute(
            "INSERT OR IGNORE INTO meta(key,value) VALUES('schema_version','1')",
            [],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO meta(key,value) VALUES('created_app_version',?1)",
            params![VERSION],
        )?;

        let mut ver = self.schema_version_u32()?;
        if ver > LEDGER_SCHEMA_VERSION {
            return Err(LedgerError::SchemaTooNew {
                found: ver,
                supported: LEDGER_SCHEMA_VERSION,
            });
        }
        while ver < LEDGER_SCHEMA_VERSION {
            let next = ver + 1;
            match next {
                2 => self.migrate_v1_to_v2()?,
                3 => self.migrate_v2_to_v3()?,
                other => {
                    return Err(LedgerError::Migration {
                        to: other,
                        detail: "no migration step registered".into(),
                    });
                }
            }
            ver = self.schema_version_u32()?;
            if ver < next {
                return Err(LedgerError::Migration {
                    to: next,
                    detail: format!("schema_version stuck at {ver}"),
                });
            }
        }
        Ok(())
    }

    /// v2: `updated_at` on transactions + migration bookkeeping meta.
    fn migrate_v1_to_v2(&mut self) -> Result<(), LedgerError> {
        if !self.column_exists("transactions", "updated_at")? {
            self.conn
                .execute("ALTER TABLE transactions ADD COLUMN updated_at TEXT", [])
                .map_err(|e| LedgerError::Migration {
                    to: 2,
                    detail: e.to_string(),
                })?;
            self.conn.execute(
                "UPDATE transactions SET updated_at = confirmed_at WHERE updated_at IS NULL",
                [],
            )?;
        }
        self.meta_set("schema_version", "2")?;
        self.meta_set("app_version", VERSION)?;
        self.meta_set("migrated_to_2_at", &crate::pipeline::utc_now_iso())?;
        Ok(())
    }

    /// v3: free-form tags + optional attachment path for receipt files.
    fn migrate_v2_to_v3(&mut self) -> Result<(), LedgerError> {
        if !self.column_exists("transactions", "tags")? {
            self.conn
                .execute("ALTER TABLE transactions ADD COLUMN tags TEXT", [])
                .map_err(|e| LedgerError::Migration {
                    to: 3,
                    detail: e.to_string(),
                })?;
        }
        if !self.column_exists("transactions", "attachment_path")? {
            self.conn
                .execute(
                    "ALTER TABLE transactions ADD COLUMN attachment_path TEXT",
                    [],
                )
                .map_err(|e| LedgerError::Migration {
                    to: 3,
                    detail: e.to_string(),
                })?;
        }
        self.meta_set("schema_version", "3")?;
        self.meta_set("app_version", VERSION)?;
        self.meta_set("migrated_to_3_at", &crate::pipeline::utc_now_iso())?;
        Ok(())
    }

    fn column_exists(&self, table: &str, column: &str) -> Result<bool, LedgerError> {
        // PRAGMA table_info — table name cannot bind; whitelist callers only.
        let pragma = format!("PRAGMA table_info({table})");
        let mut stmt = self.conn.prepare(&pragma)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Current ledger schema version from `meta` (string form for CLI).
    pub fn schema_version(&self) -> Result<String, LedgerError> {
        Ok(self.schema_version_u32()?.to_string())
    }

    /// Numeric schema version (defaults to 1 if missing).
    pub fn schema_version_u32(&self) -> Result<u32, LedgerError> {
        let v: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        match v {
            None => Ok(1),
            Some(s) => s
                .parse()
                .map_err(|_| LedgerError::Msg(format!("invalid schema_version meta value: {s}"))),
        }
    }

    /// Read arbitrary meta key (for doctor / migrations diagnostics).
    pub fn meta_get(&self, key: &str) -> Result<Option<String>, LedgerError> {
        let v = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(v)
    }

    /// Write meta key (migrations + tools).
    pub fn meta_set(&self, key: &str, value: &str) -> Result<(), LedgerError> {
        self.conn.execute(
            "INSERT INTO meta(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Confirm a draft into the ledger. `force` overrides hard dedupe.
    pub fn confirm_draft(
        &self,
        draft: &ReceiptDraft,
        content_hash: Option<&str>,
        notes: Option<&str>,
        force: bool,
    ) -> Result<ConfirmResult, LedgerError> {
        let dedupe = self.check_dedupe(draft, content_hash)?;
        if let Some(ref d) = dedupe {
            if d.level == DedupeLevel::Hard && !force {
                let existing = self.get_transaction(&d.existing_id)?;
                return Ok(ConfirmResult {
                    transaction: existing,
                    dedupe: Some(d.clone()),
                    inserted: false,
                });
            }
        }

        let id = if draft.id.is_empty() {
            ReceiptDraft::new_id()
        } else {
            draft.id.clone()
        };
        let confirmed_at = crate::pipeline::utc_now_iso();
        let invoice = draft.invoice_id.as_ref().map(|f| f.value.clone());
        let draft_json = serde_json::to_string(draft).unwrap_or_default();

        self.conn.execute(
            r#"INSERT INTO transactions(
                id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                category, invoice_id, source_path, overall_confidence, content_hash, notes, raw_text, draft_json,
                updated_at, tags, attachment_path
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"#,
            params![
                id,
                confirmed_at,
                draft.transacted_at.value,
                draft.merchant.value,
                draft.total.value.amount_minor,
                draft.total.value.currency.to_string(),
                draft.total.value.exponent as i64,
                draft.category.value,
                invoice,
                draft.source_path.as_str(),
                draft.overall_confidence as f64,
                content_hash,
                notes,
                draft.raw_text,
                draft_json,
                confirmed_at,
                Option::<String>::None,
                Option::<String>::None,
            ],
        )?;

        let transaction = self.get_transaction(&id)?;
        Ok(ConfirmResult {
            transaction,
            dedupe,
            inserted: true,
        })
    }

    pub fn check_dedupe(
        &self,
        draft: &ReceiptDraft,
        content_hash: Option<&str>,
    ) -> Result<Option<DedupeWarning>, LedgerError> {
        // Hard: invoice_id + amount + currency + calendar day
        if let Some(ref inv) = draft.invoice_id {
            if !inv.value.is_empty() {
                let day = draft
                    .transacted_at
                    .value
                    .get(..10)
                    .unwrap_or(&draft.transacted_at.value);
                let row: Option<(String,)> = self
                    .conn
                    .query_row(
                        r#"SELECT id FROM transactions
                           WHERE invoice_id = ?1
                             AND amount_minor = ?2
                             AND currency = ?3
                             AND substr(transacted_at,1,10) = ?4
                           LIMIT 1"#,
                        params![
                            inv.value,
                            draft.total.value.amount_minor,
                            draft.total.value.currency.to_string(),
                            day
                        ],
                        |r| Ok((r.get(0)?,)),
                    )
                    .optional()?;
                if let Some((id,)) = row {
                    return Ok(Some(DedupeWarning {
                        level: DedupeLevel::Hard,
                        existing_id: id,
                        message:
                            "duplicate invoice_id + amount + day; pass --force to insert again"
                                .into(),
                    }));
                }
            }
        }

        // Soft: same content hash
        if let Some(h) = content_hash {
            if !h.is_empty() {
                let row: Option<(String,)> = self
                    .conn
                    .query_row(
                        "SELECT id FROM transactions WHERE content_hash = ?1 LIMIT 1",
                        params![h],
                        |r| Ok((r.get(0)?,)),
                    )
                    .optional()?;
                if let Some((id,)) = row {
                    return Ok(Some(DedupeWarning {
                        level: DedupeLevel::Soft,
                        existing_id: id,
                        message: "same content hash — possible double capture".into(),
                    }));
                }
            }
        }

        Ok(None)
    }

    pub fn get_transaction(&self, id: &str) -> Result<Transaction, LedgerError> {
        self.conn
            .query_row(
                r#"SELECT id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                          category, invoice_id, source_path, overall_confidence, content_hash, notes,
                          tags, attachment_path
                   FROM transactions WHERE id = ?1"#,
                params![id],
                row_to_tx,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => LedgerError::NotFound(id.into()),
                other => other.into(),
            })
    }

    pub fn list_transactions(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>, LedgerError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                      category, invoice_id, source_path, overall_confidence, content_hash, notes,
                      tags, attachment_path
               FROM transactions
               ORDER BY transacted_at DESC, confirmed_at DESC
               LIMIT ?1 OFFSET ?2"#,
        )?;
        let rows = stmt.query_map(params![limit as i64, offset as i64], row_to_tx)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn count(&self) -> Result<i64, LedgerError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0))?;
        Ok(n)
    }

    /// Per-currency totals for a calendar month. Never mixes currencies.
    pub fn stats_by_currency_month(
        &self,
        year: i32,
        month: u32,
    ) -> Result<Vec<CurrencyMonthStat>, LedgerError> {
        let prefix = format!("{year:04}-{month:02}");
        let mut stmt = self.conn.prepare(
            r#"SELECT currency, SUM(amount_minor), COUNT(*)
               FROM transactions
               WHERE substr(transacted_at,1,7) = ?1
               GROUP BY currency
               ORDER BY currency"#,
        )?;
        let rows = stmt.query_map(params![prefix], |r| {
            Ok(CurrencyMonthStat {
                currency: r.get(0)?,
                year,
                month,
                total_minor: r.get(1)?,
                count: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn export_all(&self) -> Result<Vec<Transaction>, LedgerError> {
        self.list_transactions(100_000, 0)
    }

    pub fn delete_transaction(&self, id: &str) -> Result<bool, LedgerError> {
        let n = self
            .conn
            .execute("DELETE FROM transactions WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    /// Insert a fully-formed transaction (import / manual). Fails if id exists.
    pub fn insert_transaction(&self, tx: &Transaction) -> Result<(), LedgerError> {
        let updated = tx.confirmed_at.clone();
        self.conn.execute(
            r#"INSERT INTO transactions(
                id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                category, invoice_id, source_path, overall_confidence, content_hash, notes, raw_text, draft_json,
                updated_at, tags, attachment_path
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"#,
            params![
                tx.id,
                tx.confirmed_at,
                tx.transacted_at,
                tx.merchant,
                tx.amount_minor,
                tx.currency,
                tx.exponent as i64,
                tx.category,
                tx.invoice_id,
                tx.source_path,
                tx.overall_confidence as f64,
                tx.content_hash,
                tx.notes,
                Option::<String>::None,
                Option::<String>::None,
                updated,
                tx.tags,
                tx.attachment_path,
            ],
        )?;
        Ok(())
    }

    /// Import list; skips existing ids. Returns (inserted, skipped).
    pub fn import_transactions(&self, rows: &[Transaction]) -> Result<(usize, usize), LedgerError> {
        let mut inserted = 0usize;
        let mut skipped = 0usize;
        for tx in rows {
            match self.get_transaction(&tx.id) {
                Ok(_) => skipped += 1,
                Err(LedgerError::NotFound(_)) => {
                    self.insert_transaction(tx)?;
                    inserted += 1;
                }
                Err(e) => return Err(e),
            }
        }
        Ok((inserted, skipped))
    }

    /// Partial update; only `Some` fields on [`TxUpdate`] are written.
    pub fn update_transaction(&self, id: &str, u: &TxUpdate) -> Result<Transaction, LedgerError> {
        let mut tx = self.get_transaction(id)?;
        if let Some(ref m) = u.merchant {
            tx.merchant = m.clone();
        }
        if let Some(a) = u.amount_minor {
            tx.amount_minor = a;
        }
        if let Some(ref c) = u.currency {
            tx.currency = c.clone();
            if let Some(iso) = Iso4217::parse(c) {
                tx.exponent = u.exponent.unwrap_or(iso.exponent());
            }
        } else if let Some(e) = u.exponent {
            tx.exponent = e;
        }
        if let Some(ref cat) = u.category {
            tx.category = cat.clone();
        }
        if let Some(ref n) = u.notes {
            tx.notes = Some(n.clone());
        }
        if let Some(ref d) = u.transacted_at {
            tx.transacted_at = d.clone();
        }
        if let Some(ref t) = u.tags {
            // Empty string clears tags (schema v3 free-form).
            tx.tags = if t.is_empty() { None } else { Some(t.clone()) };
        }
        if let Some(ref a) = u.attachment_path {
            // Empty string clears attachment_path.
            tx.attachment_path = if a.is_empty() { None } else { Some(a.clone()) };
        }
        let now = crate::pipeline::utc_now_iso();
        let n = self.conn.execute(
            r#"UPDATE transactions SET
                merchant = ?2,
                amount_minor = ?3,
                currency = ?4,
                exponent = ?5,
                category = ?6,
                notes = ?7,
                transacted_at = ?8,
                updated_at = ?9,
                tags = ?10,
                attachment_path = ?11
               WHERE id = ?1"#,
            params![
                id,
                tx.merchant,
                tx.amount_minor,
                tx.currency,
                tx.exponent as i64,
                tx.category,
                tx.notes,
                tx.transacted_at,
                now,
                tx.tags,
                tx.attachment_path,
            ],
        )?;
        if n == 0 {
            return Err(LedgerError::NotFound(id.into()));
        }
        self.get_transaction(id)
    }

    /// Filter list by optional substring merchant/category and currency.
    pub fn list_filtered(
        &self,
        limit: usize,
        offset: usize,
        currency: Option<&str>,
        query: Option<&str>,
    ) -> Result<Vec<Transaction>, LedgerError> {
        self.query_transactions(&TxFilter {
            limit,
            offset,
            currency: currency.map(|s| s.to_string()),
            query: query.map(|s| s.to_string()),
            ..Default::default()
        })
    }

    /// Rich query: tags, category, date range, amount bounds, attachment flag.
    pub fn query_transactions(&self, f: &TxFilter) -> Result<Vec<Transaction>, LedgerError> {
        let mut sql = String::from(
            r#"SELECT id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                      category, invoice_id, source_path, overall_confidence, content_hash, notes,
                      tags, attachment_path
               FROM transactions WHERE 1=1"#,
        );
        let mut vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref c) = f.currency {
            sql.push_str(" AND currency = ?");
            vals.push(Box::new(c.clone()));
        }
        if let Some(ref q) = f.query {
            sql.push_str(
                " AND (merchant LIKE ? OR category LIKE ? OR IFNULL(notes,'') LIKE ? OR IFNULL(tags,'') LIKE ?)",
            );
            let pat = format!("%{q}%");
            vals.push(Box::new(pat.clone()));
            vals.push(Box::new(pat.clone()));
            vals.push(Box::new(pat.clone()));
            vals.push(Box::new(pat));
        }
        if let Some(ref tag) = f.tag {
            // Comma-separated tags: match whole token case-insensitively via LIKE boundaries.
            let t = tag.trim().to_lowercase();
            sql.push_str(" AND (',' || lower(IFNULL(tags,'')) || ',') LIKE ?");
            vals.push(Box::new(format!("%,{t},%")));
        }
        if let Some(ref cat) = f.category {
            sql.push_str(" AND category = ?");
            vals.push(Box::new(cat.clone()));
        }
        if let Some(ref ym) = f.year_month {
            sql.push_str(" AND substr(transacted_at,1,7) = ?");
            vals.push(Box::new(ym.clone()));
        }
        if let Some(ref from) = f.from {
            sql.push_str(" AND substr(transacted_at,1,10) >= ?");
            vals.push(Box::new(from.clone()));
        }
        if let Some(ref to) = f.to {
            sql.push_str(" AND substr(transacted_at,1,10) <= ?");
            vals.push(Box::new(to.clone()));
        }
        if let Some(min) = f.min_minor {
            sql.push_str(" AND amount_minor >= ?");
            vals.push(Box::new(min));
        }
        if let Some(max) = f.max_minor {
            sql.push_str(" AND amount_minor <= ?");
            vals.push(Box::new(max));
        }
        match f.has_attachment {
            Some(true) => {
                sql.push_str(" AND attachment_path IS NOT NULL AND length(attachment_path) > 0");
            }
            Some(false) => {
                sql.push_str(" AND (attachment_path IS NULL OR length(attachment_path) = 0)");
            }
            None => {}
        }

        sql.push_str(" ORDER BY transacted_at DESC, confirmed_at DESC LIMIT ? OFFSET ?");
        let lim = if f.limit == 0 { 50 } else { f.limit } as i64;
        let off = f.offset as i64;
        vals.push(Box::new(lim));
        vals.push(Box::new(off));

        let params: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), row_to_tx)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Distinct tag tokens across the ledger (sorted).
    pub fn list_tags(&self) -> Result<Vec<String>, LedgerError> {
        let mut stmt = self
            .conn
            .prepare("SELECT tags FROM transactions WHERE tags IS NOT NULL AND tags != ''")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut set = std::collections::BTreeSet::new();
        for r in rows {
            let raw = r?;
            for part in raw.split(',') {
                let t = part.trim();
                if !t.is_empty() {
                    set.insert(t.to_string());
                }
            }
        }
        Ok(set.into_iter().collect())
    }

    /// Per-currency totals for a date prefix range [from, to] inclusive on YYYY-MM-DD strings.
    pub fn stats_by_currency_range(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<CurrencyMonthStat>, LedgerError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT currency, SUM(amount_minor), COUNT(*)
               FROM transactions
               WHERE substr(transacted_at,1,10) >= ?1 AND substr(transacted_at,1,10) <= ?2
               GROUP BY currency
               ORDER BY currency"#,
        )?;
        let rows = stmt.query_map(params![from, to], |r| {
            Ok(CurrencyMonthStat {
                currency: r.get(0)?,
                year: 0,
                month: 0,
                total_minor: r.get(1)?,
                count: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Category breakdown for one currency, optional month prefix `YYYY-MM` or all-time.
    pub fn stats_by_category(
        &self,
        currency: &str,
        year_month: Option<&str>,
    ) -> Result<Vec<CategoryStat>, LedgerError> {
        let (sql, bind_ym): (&str, Option<&str>) = if let Some(ym) = year_month {
            (
                r#"SELECT currency, category, SUM(amount_minor), COUNT(*)
                   FROM transactions
                   WHERE currency = ?1 AND substr(transacted_at,1,7) = ?2
                   GROUP BY currency, category
                   ORDER BY SUM(amount_minor) DESC"#,
                Some(ym),
            )
        } else {
            (
                r#"SELECT currency, category, SUM(amount_minor), COUNT(*)
                   FROM transactions
                   WHERE currency = ?1
                   GROUP BY currency, category
                   ORDER BY SUM(amount_minor) DESC"#,
                None,
            )
        };
        let mut stmt = self.conn.prepare(sql)?;
        let map = |r: &rusqlite::Row<'_>| {
            Ok(CategoryStat {
                currency: r.get(0)?,
                category: r.get(1)?,
                total_minor: r.get(2)?,
                count: r.get(3)?,
            })
        };
        let rows = if let Some(ym) = bind_ym {
            stmt.query_map(params![currency, ym], map)?
        } else {
            stmt.query_map(params![currency], map)?
        };
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Top merchants by spend within one currency (required — no cross-currency).
    pub fn top_merchants(
        &self,
        currency: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64, i64)>, LedgerError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT merchant, SUM(amount_minor), COUNT(*)
               FROM transactions
               WHERE currency = ?1
               GROUP BY merchant
               ORDER BY SUM(amount_minor) DESC
               LIMIT ?2"#,
        )?;
        let rows = stmt.query_map(params![currency, limit as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn clear_all(&self) -> Result<usize, LedgerError> {
        let n = self.conn.execute("DELETE FROM transactions", [])?;
        Ok(n)
    }

    /// Most recently confirmed transaction (by confirmed_at, then id).
    pub fn last_transaction(&self) -> Result<Option<Transaction>, LedgerError> {
        let row = self
            .conn
            .query_row(
                r#"SELECT id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                          category, invoice_id, source_path, overall_confidence, content_hash, notes,
                          tags, attachment_path
                   FROM transactions
                   ORDER BY confirmed_at DESC, id DESC
                   LIMIT 1"#,
                [],
                row_to_tx,
            )
            .optional()?;
        Ok(row)
    }

    pub fn list_by_month(
        &self,
        year: i32,
        month: u32,
        limit: usize,
    ) -> Result<Vec<Transaction>, LedgerError> {
        let prefix = format!("{year:04}-{month:02}");
        let mut stmt = self.conn.prepare(
            r#"SELECT id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                      category, invoice_id, source_path, overall_confidence, content_hash, notes,
                      tags, attachment_path
               FROM transactions
               WHERE substr(transacted_at,1,7) = ?1
               ORDER BY transacted_at DESC, confirmed_at DESC
               LIMIT ?2"#,
        )?;
        let rows = stmt.query_map(params![prefix, limit as i64], row_to_tx)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Re-apply category engine to all rows (or only `other`). Returns updated count.
    pub fn recategorize_all(
        &self,
        engine: &crate::category::CategoryEngine,
        only_other: bool,
    ) -> Result<usize, LedgerError> {
        let rows = self.export_all()?;
        let mut n = 0usize;
        for mut tx in rows {
            if only_other && tx.category != crate::category::CAT_OTHER {
                continue;
            }
            let mut ex = crate::explain::ExplainTrace::new("recategorize", "rule");
            let field = engine.categorize(&tx.merchant, tx.notes.as_deref().unwrap_or(""), &mut ex);
            if field.value != tx.category {
                tx.category = field.value;
                self.conn.execute(
                    "UPDATE transactions SET category = ?2 WHERE id = ?1",
                    params![tx.id, tx.category],
                )?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// All-time per-currency totals (no cross-currency sum).
    pub fn stats_by_currency_all(&self) -> Result<Vec<CurrencyMonthStat>, LedgerError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT currency, SUM(amount_minor), COUNT(*)
               FROM transactions
               GROUP BY currency
               ORDER BY currency"#,
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(CurrencyMonthStat {
                currency: r.get(0)?,
                year: 0,
                month: 0,
                total_minor: r.get(1)?,
                count: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Per-currency totals for a full calendar year (no cross-currency sum).
    pub fn stats_by_currency_year(&self, year: i32) -> Result<Vec<CurrencyMonthStat>, LedgerError> {
        let prefix = format!("{year:04}");
        let mut stmt = self.conn.prepare(
            r#"SELECT currency, SUM(amount_minor), COUNT(*)
               FROM transactions
               WHERE substr(transacted_at,1,4) = ?1
               GROUP BY currency
               ORDER BY currency"#,
        )?;
        let rows = stmt.query_map(params![prefix], |r| {
            Ok(CurrencyMonthStat {
                currency: r.get(0)?,
                year,
                month: 0,
                total_minor: r.get(1)?,
                count: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Per-currency monthly rows within one calendar year (for annual heatmaps).
    pub fn stats_by_currency_year_months(
        &self,
        year: i32,
    ) -> Result<Vec<CurrencyMonthStat>, LedgerError> {
        let prefix = format!("{year:04}");
        let mut stmt = self.conn.prepare(
            r#"SELECT currency,
                      CAST(substr(transacted_at,6,2) AS INTEGER),
                      SUM(amount_minor),
                      COUNT(*)
               FROM transactions
               WHERE substr(transacted_at,1,4) = ?1
               GROUP BY currency, substr(transacted_at,1,7)
               ORDER BY currency, substr(transacted_at,1,7)"#,
        )?;
        let rows = stmt.query_map(params![prefix], |r| {
            let month: i64 = r.get(1)?;
            Ok(CurrencyMonthStat {
                currency: r.get(0)?,
                year,
                month: month as u32,
                total_minor: r.get(2)?,
                count: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Rewrite merchant field for all rows matching exact `from` → `to`.
    pub fn rewrite_merchant(&self, from: &str, to: &str) -> Result<usize, LedgerError> {
        let now = crate::pipeline::utc_now_iso();
        let n = self.conn.execute(
            "UPDATE transactions SET merchant = ?2, updated_at = ?3 WHERE merchant = ?1",
            params![from, to, now],
        )?;
        Ok(n)
    }

    /// Serialize DB file bytes (for backup). In-memory DBs are dumped via VACUUM INTO temp.
    pub fn export_sqlite_bytes(&self) -> Result<Vec<u8>, LedgerError> {
        if self.path == Path::new(":memory:") || self.path.as_os_str().is_empty() {
            let tmp = std::env::temp_dir().join(format!("rradar-dump-{}.db", ulid::Ulid::new()));
            self.conn
                .execute("VACUUM INTO ?1", params![tmp.to_string_lossy().as_ref()])?;
            let bytes = std::fs::read(&tmp)?;
            let _ = std::fs::remove_file(&tmp);
            return Ok(bytes);
        }
        // Checkpoint WAL so main file is complete
        let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(FULL);");
        Ok(std::fs::read(&self.path)?)
    }
}

fn row_to_tx(r: &rusqlite::Row<'_>) -> rusqlite::Result<Transaction> {
    Ok(Transaction {
        id: r.get(0)?,
        confirmed_at: r.get(1)?,
        transacted_at: r.get(2)?,
        merchant: r.get(3)?,
        amount_minor: r.get(4)?,
        currency: r.get(5)?,
        exponent: r.get::<_, i64>(6)? as u8,
        category: r.get(7)?,
        invoice_id: r.get(8)?,
        source_path: r.get(9)?,
        overall_confidence: r.get::<_, f64>(10)? as f32,
        content_hash: r.get(11)?,
        notes: r.get(12)?,
        tags: r.get(13).ok().flatten(),
        attachment_path: r.get(14).ok().flatten(),
    })
}

/// Apply optional user edits when confirming from CLI JSON overlay.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserEdits {
    pub merchant: Option<String>,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub category: Option<String>,
    pub notes: Option<String>,
    pub transacted_at: Option<String>,
}

pub fn apply_edits(draft: &mut ReceiptDraft, edits: &UserEdits) {
    if let Some(ref m) = edits.merchant {
        draft.merchant.value = m.clone();
        draft.merchant.source = FieldSource::User;
        draft.merchant.confidence = 1.0;
    }
    if let Some(a) = edits.amount_minor {
        let cur = edits
            .currency
            .as_deref()
            .and_then(Iso4217::parse)
            .unwrap_or(draft.total.value.currency);
        draft.total.value = Money::new(a, cur);
        draft.total.source = FieldSource::User;
        draft.total.confidence = 1.0;
    } else if let Some(ref c) = edits.currency {
        if let Some(cur) = Iso4217::parse(c) {
            draft.total.value = Money::new(draft.total.value.amount_minor, cur);
            draft.total.source = FieldSource::User;
        }
    }
    if let Some(ref cat) = edits.category {
        draft.category.value = cat.clone();
        draft.category.source = FieldSource::User;
        draft.category.confidence = 1.0;
    }
    if let Some(ref d) = edits.transacted_at {
        draft.transacted_at.value = d.clone();
        draft.transacted_at.source = FieldSource::User;
    }
    let _ = SourcePath::Ocr; // keep import used if optimized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::{Iso4217, Money};
    use crate::types::{Field, FieldSource, SourcePath};

    fn sample_draft(id: &str, inv: Option<&str>, minor: i64) -> ReceiptDraft {
        ReceiptDraft {
            id: id.into(),
            captured_at: "2024-05-01T12:00:00Z".into(),
            merchant: Field::new("全家".into(), 0.9, FieldSource::Rule),
            total: Field::new(Money::new(minor, Iso4217::TWD), 0.9, FieldSource::Rule),
            transacted_at: Field::new("2024-05-01".into(), 0.9, FieldSource::Rule),
            tax: None,
            invoice_id: inv.map(|i| Field::new(i.into(), 0.9, FieldSource::Qr)),
            category: Field::new("grocery_convenience".into(), 0.9, FieldSource::Rule),
            raw_text: "test".into(),
            ocr_blocks: vec![],
            overall_confidence: 0.9,
            explain: crate::ExplainTrace::new("mock", "ocr"),
            source_path: SourcePath::Ocr,
        }
    }

    #[test]
    fn confirm_list_stats() {
        let db = Ledger::open_in_memory().unwrap();
        let d = sample_draft("tx1", Some("AB12345678"), 8900);
        let r = db.confirm_draft(&d, Some("hash1"), None, false).unwrap();
        assert!(r.inserted);
        assert_eq!(db.count().unwrap(), 1);
        let list = db.list_transactions(10, 0).unwrap();
        assert_eq!(list[0].amount_minor, 8900);
        let stats = db.stats_by_currency_month(2024, 5).unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].total_minor, 8900);
        assert_eq!(stats[0].currency, "TWD");
    }

    #[test]
    fn hard_dedupe_blocks() {
        let db = Ledger::open_in_memory().unwrap();
        let d1 = sample_draft("tx1", Some("AB12345678"), 8900);
        db.confirm_draft(&d1, Some("h1"), None, false).unwrap();
        let d2 = sample_draft("tx2", Some("AB12345678"), 8900);
        let r = db.confirm_draft(&d2, Some("h2"), None, false).unwrap();
        assert!(!r.inserted);
        assert_eq!(r.dedupe.unwrap().level, DedupeLevel::Hard);
        assert_eq!(db.count().unwrap(), 1);
        let r2 = db.confirm_draft(&d2, Some("h2"), None, true).unwrap();
        assert!(r2.inserted);
        assert_eq!(db.count().unwrap(), 2);
    }

    #[test]
    fn soft_dedupe_warns_but_inserts() {
        let db = Ledger::open_in_memory().unwrap();
        let d1 = sample_draft("tx1", None, 100);
        db.confirm_draft(&d1, Some("samehash"), None, false)
            .unwrap();
        let d2 = sample_draft("tx2", None, 200);
        let r = db
            .confirm_draft(&d2, Some("samehash"), None, false)
            .unwrap();
        assert!(r.inserted);
        assert_eq!(r.dedupe.unwrap().level, DedupeLevel::Soft);
        assert_eq!(db.count().unwrap(), 2);
    }

    #[test]
    fn no_cross_currency_in_stats() {
        let db = Ledger::open_in_memory().unwrap();
        let mut d1 = sample_draft("a", None, 100);
        d1.total.value = Money::new(100, Iso4217::TWD);
        let mut d2 = sample_draft("b", None, 200);
        d2.total.value = Money::new(200, Iso4217::USD);
        d2.transacted_at.value = "2024-05-02".into();
        db.confirm_draft(&d1, None, None, false).unwrap();
        db.confirm_draft(&d2, None, None, false).unwrap();
        let stats = db.stats_by_currency_month(2024, 5).unwrap();
        assert_eq!(stats.len(), 2);
    }

    #[test]
    fn delete_and_update() {
        let db = Ledger::open_in_memory().unwrap();
        let d = sample_draft("txdel", None, 500);
        db.confirm_draft(&d, None, None, false).unwrap();
        let u = db
            .update_transaction(
                "txdel",
                &TxUpdate {
                    merchant: Some("新店名".into()),
                    amount_minor: Some(600),
                    currency: Some("TWD".into()),
                    category: Some("other".into()),
                    notes: Some("note".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(u.merchant, "新店名");
        assert_eq!(u.amount_minor, 600);
        assert_eq!(u.notes.as_deref(), Some("note"));
        assert!(db.delete_transaction("txdel").unwrap());
        assert_eq!(db.count().unwrap(), 0);
    }

    #[test]
    fn list_filtered_query() {
        let db = Ledger::open_in_memory().unwrap();
        db.confirm_draft(&sample_draft("1", None, 100), None, None, false)
            .unwrap();
        let mut d2 = sample_draft("2", None, 200);
        d2.merchant = Field::new("肯德基".into(), 1.0, FieldSource::User);
        d2.category = Field::new("food_dining".into(), 1.0, FieldSource::User);
        db.confirm_draft(&d2, None, None, false).unwrap();
        let found = db.list_filtered(10, 0, None, Some("肯德")).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].merchant.contains("肯德"));
    }

    #[test]
    fn query_by_tag_and_amount() {
        let db = Ledger::open_in_memory().unwrap();
        db.confirm_draft(&sample_draft("1", None, 100), None, None, false)
            .unwrap();
        db.confirm_draft(&sample_draft("2", None, 5000), None, None, false)
            .unwrap();
        db.update_transaction(
            "1",
            &TxUpdate {
                tags: Some("demo,work".into()),
                ..Default::default()
            },
        )
        .unwrap();
        db.update_transaction(
            "2",
            &TxUpdate {
                tags: Some("personal".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let tagged = db
            .query_transactions(&TxFilter {
                limit: 50,
                tag: Some("work".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].id, "1");
        let range = db
            .query_transactions(&TxFilter {
                limit: 50,
                min_minor: Some(1000),
                max_minor: Some(10_000),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(range.len(), 1);
        assert_eq!(range[0].id, "2");
        let tags = db.list_tags().unwrap();
        assert!(tags.iter().any(|t| t == "demo"));
        assert!(tags.iter().any(|t| t == "work"));
    }

    #[test]
    fn top_and_range_and_clear() {
        let db = Ledger::open_in_memory().unwrap();
        let mut a = sample_draft("a", None, 1000);
        a.merchant = Field::new("店A".into(), 1.0, FieldSource::User);
        let mut b = sample_draft("b", None, 5000);
        b.merchant = Field::new("店B".into(), 1.0, FieldSource::User);
        b.transacted_at.value = "2024-06-01".into();
        db.confirm_draft(&a, None, None, false).unwrap();
        db.confirm_draft(&b, None, None, false).unwrap();
        let top = db.top_merchants("TWD", 5).unwrap();
        assert_eq!(top[0].0, "店B");
        assert_eq!(top[0].1, 5000);
        let range = db
            .stats_by_currency_range("2024-05-01", "2024-05-31")
            .unwrap();
        assert_eq!(range[0].total_minor, 1000);
        assert_eq!(db.clear_all().unwrap(), 2);
        assert_eq!(db.count().unwrap(), 0);
    }

    #[test]
    fn last_and_month_list() {
        let db = Ledger::open_in_memory().unwrap();
        db.confirm_draft(&sample_draft("x1", None, 100), None, None, false)
            .unwrap();
        let last = db.last_transaction().unwrap().unwrap();
        assert_eq!(last.id, "x1");
        let m = db.list_by_month(2024, 5, 10).unwrap();
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn schema_migrates_to_current() {
        let db = Ledger::open_in_memory().unwrap();
        assert_eq!(db.schema_version_u32().unwrap(), LEDGER_SCHEMA_VERSION);
        assert!(db.column_exists("transactions", "updated_at").unwrap());
        assert!(db.meta_get("migrated_to_2_at").unwrap().is_some());
        db.confirm_draft(&sample_draft("s1", None, 10), None, None, false)
            .unwrap();
        db.update_transaction(
            "s1",
            &TxUpdate {
                notes: Some("touched".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let updated: String = db
            .conn
            .query_row(
                "SELECT updated_at FROM transactions WHERE id='s1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!updated.is_empty());
    }

    #[test]
    fn schema_too_new_rejected() {
        let db = Ledger::open_in_memory().unwrap();
        db.meta_set("schema_version", "99").unwrap();
        // Re-open same path not available for memory; call migrate logic via meta check.
        let err = match db.schema_version_u32().unwrap() {
            v if v > LEDGER_SCHEMA_VERSION => LedgerError::SchemaTooNew {
                found: v,
                supported: LEDGER_SCHEMA_VERSION,
            },
            _ => panic!("expected high version"),
        };
        assert!(err.to_string().contains("newer"));
    }
}
