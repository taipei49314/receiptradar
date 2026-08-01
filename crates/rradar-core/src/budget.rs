//! Local monthly budgets — soft limits only, never cloud-synced.
//!
//! Budgets are stored under the data dir (`budgets.toml`) as **major** amounts
//! for readability. Status checks use ledger minor units within **one currency**
//! (no cross-currency math).

use crate::ledger::{Ledger, LedgerError};
use crate::money::{Iso4217, Money};
use crate::paths::data_dir;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One budget line: overall monthly (category=None) or per-category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetLine {
    pub currency: String,
    /// Limit in **minor** units for storage / compare.
    pub limit_minor: i64,
    /// `None` = overall monthly limit for this currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Full local budget book.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BudgetBook {
    pub lines: Vec<BudgetLine>,
}

/// Status of one line against actual spend in a YYYY-MM window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetStatus {
    pub currency: String,
    pub category: Option<String>,
    pub year: i32,
    pub month: u32,
    pub limit_minor: i64,
    pub spent_minor: i64,
    pub remaining_minor: i64,
    /// 0.0 ..= +∞ (can exceed 1.0 when over budget).
    pub ratio: f64,
    pub over: bool,
}

impl BudgetBook {
    pub fn path() -> PathBuf {
        data_dir().join("budgets.toml")
    }

    pub fn load() -> Self {
        Self::load_from(&Self::path())
    }

    pub fn load_from(path: &Path) -> Self {
        if let Ok(s) = std::fs::read_to_string(path) {
            return parse_budgets_toml(&s);
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&Self::path())
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_toml_string())
    }

    pub fn to_toml_string(&self) -> String {
        let mut out = String::from(
            "# ReceiptRadar local budgets (major units in file)\n\
             # Never mixed across currencies. Soft limits only — no cloud.\n\
             # overall: monthly.<CCY> = <major>\n\
             # category: category.<CCY>.<id> = <major>\n",
        );
        for line in &self.lines {
            let iso = Iso4217::parse(&line.currency).unwrap_or(Iso4217::TWD);
            let major = Money::new(line.limit_minor, iso).display_major();
            match &line.category {
                None => out.push_str(&format!("monthly.{} = {}\n", line.currency, major)),
                Some(cat) => {
                    out.push_str(&format!("category.{}.{} = {}\n", line.currency, cat, major));
                }
            }
        }
        out
    }

    /// Upsert overall or category limit from major-unit string (e.g. "30000" or "89.50").
    pub fn set_major(
        &mut self,
        currency: &str,
        major: &str,
        category: Option<&str>,
    ) -> Result<(), String> {
        let iso = Iso4217::parse(currency).ok_or_else(|| format!("bad currency `{currency}`"))?;
        let money = Money::from_major_str(major, iso).map_err(|e| e.to_string())?;
        let cat = category
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        if let Some(existing) = self
            .lines
            .iter_mut()
            .find(|l| l.currency.eq_ignore_ascii_case(currency) && l.category == cat)
        {
            existing.limit_minor = money.amount_minor;
            existing.currency = iso.to_string();
        } else {
            self.lines.push(BudgetLine {
                currency: iso.to_string(),
                limit_minor: money.amount_minor,
                category: cat,
            });
        }
        Ok(())
    }

    pub fn clear_line(&mut self, currency: &str, category: Option<&str>) -> bool {
        let cat = category
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        let before = self.lines.len();
        self.lines
            .retain(|l| !(l.currency.eq_ignore_ascii_case(currency) && l.category == cat));
        self.lines.len() < before
    }

    pub fn clear_all(&mut self) {
        self.lines.clear();
    }
}

/// Soft-parse budgets.toml (no heavy toml crate).
pub fn parse_budgets_toml(s: &str) -> BudgetBook {
    let mut book = BudgetBook::default();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"');
        if let Some(rest) = k.strip_prefix("monthly.") {
            let ccy = rest.trim();
            if let Some(iso) = Iso4217::parse(ccy) {
                if let Ok(m) = Money::from_major_str(v, iso) {
                    book.lines.push(BudgetLine {
                        currency: iso.to_string(),
                        limit_minor: m.amount_minor,
                        category: None,
                    });
                }
            }
        } else if let Some(rest) = k.strip_prefix("category.") {
            // category.TWD.food_dining
            let mut parts = rest.splitn(2, '.');
            let ccy = parts.next().unwrap_or("");
            let cat = parts.next().unwrap_or("");
            if cat.is_empty() {
                continue;
            }
            if let Some(iso) = Iso4217::parse(ccy) {
                if let Ok(m) = Money::from_major_str(v, iso) {
                    book.lines.push(BudgetLine {
                        currency: iso.to_string(),
                        limit_minor: m.amount_minor,
                        category: Some(cat.to_string()),
                    });
                }
            }
        }
    }
    book
}

/// Evaluate all budget lines against a ledger for calendar month.
pub fn budget_status_month(
    ledger: &Ledger,
    book: &BudgetBook,
    year: i32,
    month: u32,
) -> Result<Vec<BudgetStatus>, LedgerError> {
    let ym = format!("{year:04}-{month:02}");
    let mut out = Vec::new();
    for line in &book.lines {
        let spent = match &line.category {
            None => {
                let stats = ledger.stats_by_currency_month(year, month)?;
                stats
                    .iter()
                    .find(|s| s.currency.eq_ignore_ascii_case(&line.currency))
                    .map(|s| s.total_minor)
                    .unwrap_or(0)
            }
            Some(cat) => {
                let cats = ledger.stats_by_category(&line.currency, Some(&ym))?;
                cats.iter()
                    .find(|c| c.category == *cat)
                    .map(|c| c.total_minor)
                    .unwrap_or(0)
            }
        };
        let remaining = line.limit_minor - spent;
        let ratio = if line.limit_minor > 0 {
            spent as f64 / line.limit_minor as f64
        } else {
            0.0
        };
        out.push(BudgetStatus {
            currency: line.currency.clone(),
            category: line.category.clone(),
            year,
            month,
            limit_minor: line.limit_minor,
            spent_minor: spent,
            remaining_minor: remaining,
            ratio,
            over: spent > line.limit_minor,
        });
    }
    Ok(out)
}

/// Markdown section for monthly report (empty string if no budgets).
pub fn budget_markdown_section(
    ledger: &Ledger,
    book: &BudgetBook,
    year: i32,
    month: u32,
) -> Result<String, LedgerError> {
    if book.lines.is_empty() {
        return Ok(String::new());
    }
    let statuses = budget_status_month(ledger, book, year, month)?;
    let mut out = String::from("## Budgets (local soft limits)\n\n");
    out.push_str("| Scope | Currency | Spent | Limit | Remaining | Status |\n");
    out.push_str("|---|---|---:|---:|---:|---|\n");
    for s in &statuses {
        let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
        let spent = Money::new(s.spent_minor, iso).display_major();
        let limit = Money::new(s.limit_minor, iso).display_major();
        let rem = Money::new(s.remaining_minor, iso).display_major();
        let scope = s
            .category
            .as_deref()
            .map(|c| format!("category `{c}`"))
            .unwrap_or_else(|| "overall".into());
        let status = if s.over {
            format!("OVER ({:.0}%)", s.ratio * 100.0)
        } else {
            format!("ok ({:.0}%)", s.ratio * 100.0)
        };
        out.push_str(&format!(
            "| {scope} | {} | {spent} | {limit} | {rem} | {status} |\n",
            s.currency
        ));
    }
    out.push('\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Ledger;
    use crate::types::{Field, FieldSource, ReceiptDraft, SourcePath};

    fn sample(id: &str, cat: &str, minor: i64) -> ReceiptDraft {
        ReceiptDraft {
            id: id.into(),
            captured_at: "2024-05-10T00:00:00Z".into(),
            merchant: Field::new("店".into(), 1.0, FieldSource::User),
            total: Field::new(Money::new(minor, Iso4217::TWD), 1.0, FieldSource::User),
            transacted_at: Field::new("2024-05-10".into(), 1.0, FieldSource::User),
            tax: None,
            invoice_id: None,
            category: Field::new(cat.into(), 1.0, FieldSource::User),
            raw_text: String::new(),
            ocr_blocks: vec![],
            overall_confidence: 1.0,
            explain: crate::ExplainTrace::new("t", "ocr"),
            source_path: SourcePath::Manual,
        }
    }

    #[test]
    fn parse_and_status() {
        let s = r#"
monthly.TWD = 100
category.TWD.food_dining = 50
"#;
        let book = parse_budgets_toml(s);
        assert_eq!(book.lines.len(), 2);
        // TWD exponent 2 → 100.00 major = 10000 minor
        assert_eq!(book.lines[0].limit_minor, 10000);
        assert_eq!(book.lines[1].limit_minor, 5000);

        let db = Ledger::open_in_memory().unwrap();
        db.confirm_draft(&sample("a", "food_dining", 6000), None, None, false)
            .unwrap();
        db.confirm_draft(&sample("b", "other", 1000), None, None, false)
            .unwrap();
        let st = budget_status_month(&db, &book, 2024, 5).unwrap();
        let overall = st.iter().find(|x| x.category.is_none()).unwrap();
        assert_eq!(overall.spent_minor, 7000);
        assert!(!overall.over);
        let food = st
            .iter()
            .find(|x| x.category.as_deref() == Some("food_dining"))
            .unwrap();
        assert_eq!(food.spent_minor, 6000);
        assert!(food.over);
    }

    #[test]
    fn roundtrip_toml() {
        let mut book = BudgetBook::default();
        book.set_major("TWD", "30000", None).unwrap();
        book.set_major("TWD", "5000", Some("grocery_convenience"))
            .unwrap();
        let s = book.to_toml_string();
        let p = parse_budgets_toml(&s);
        assert_eq!(p.lines.len(), 2);
        assert_eq!(p.lines[0].limit_minor, 3_000_000);
    }
}
