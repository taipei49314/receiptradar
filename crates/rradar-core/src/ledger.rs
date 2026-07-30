//! SQLite ledger: confirm drafts, list, stats by currency/month.

use crate::money::{Iso4217, Money};
use crate::types::{FieldSource, ReceiptDraft, SourcePath};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

const SCHEMA: &str = r#"
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrencyMonthStat {
    pub currency: String,
    pub year: i32,
    pub month: u32,
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

    fn migrate(&mut self) -> Result<(), LedgerError> {
        self.conn.execute_batch(SCHEMA)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO meta(key,value) VALUES('schema_version','1')",
            [],
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
                category, invoice_id, source_path, overall_confidence, content_hash, notes, raw_text, draft_json
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)"#,
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
                let day = draft.transacted_at.value.get(..10).unwrap_or(&draft.transacted_at.value);
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
                        message: "duplicate invoice_id + amount + day; pass --force to insert again"
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
                          category, invoice_id, source_path, overall_confidence, content_hash, notes
                   FROM transactions WHERE id = ?1"#,
                params![id],
                row_to_tx,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => LedgerError::NotFound(id.into()),
                other => other.into(),
            })
    }

    pub fn list_transactions(&self, limit: usize, offset: usize) -> Result<Vec<Transaction>, LedgerError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, confirmed_at, transacted_at, merchant, amount_minor, currency, exponent,
                      category, invoice_id, source_path, overall_confidence, content_hash, notes
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

    /// Serialize DB file bytes (for backup). In-memory DBs are dumped via VACUUM INTO temp.
    pub fn export_sqlite_bytes(&self) -> Result<Vec<u8>, LedgerError> {
        if self.path == Path::new(":memory:") || self.path.as_os_str().is_empty() {
            let tmp = std::env::temp_dir().join(format!("rradar-dump-{}.db", ulid::Ulid::new()));
            self.conn.execute(
                "VACUUM INTO ?1",
                params![tmp.to_string_lossy().as_ref()],
            )?;
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
        db.confirm_draft(&d1, Some("samehash"), None, false).unwrap();
        let d2 = sample_draft("tx2", None, 200);
        let r = db.confirm_draft(&d2, Some("samehash"), None, false).unwrap();
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
}
