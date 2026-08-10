//! Local merchant display aliases — soft rename map, never cloud-synced.
//!
//! Stored as `{data_dir}/merchant_aliases.toml` (simple `from = to` lines).
//! Used for report/list display and optional ledger rewrite.

use crate::paths::data_dir;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Map of exact merchant string → preferred display name.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AliasBook {
    /// Sorted for stable file output.
    pub map: BTreeMap<String, String>,
}

impl AliasBook {
    pub fn path() -> PathBuf {
        data_dir().join("merchant_aliases.toml")
    }

    pub fn load() -> Self {
        Self::load_from(&Self::path())
    }

    pub fn load_from(path: &Path) -> Self {
        if let Ok(s) = std::fs::read_to_string(path) {
            return parse_aliases_toml(&s);
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
            "# ReceiptRadar merchant aliases (exact match → display name)\n\
             # Local-only; packed into backup.rradar when present.\n\
             # \"全家便利商店\" = \"全家\"\n",
        );
        for (k, v) in &self.map {
            out.push_str(&format!(
                "\"{}\" = \"{}\"\n",
                escape_toml(k),
                escape_toml(v)
            ));
        }
        out
    }

    pub fn set(&mut self, from: &str, to: &str) {
        let f = from.trim();
        let t = to.trim();
        if f.is_empty() || t.is_empty() {
            return;
        }
        self.map.insert(f.to_string(), t.to_string());
    }

    pub fn remove(&mut self, from: &str) -> bool {
        self.map.remove(from.trim()).is_some()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Exact match rewrite; returns original if no alias.
    pub fn apply(&self, merchant: &str) -> String {
        self.display_for(merchant)
    }

    /// Display name: exact alias, else longest key contained in / prefix of merchant.
    pub fn display_for(&self, merchant: &str) -> String {
        let m = merchant.trim();
        if m.is_empty() {
            return merchant.to_string();
        }
        if let Some(v) = self.map.get(m) {
            return v.clone();
        }
        let mut best: Option<(&String, &String)> = None;
        for (k, v) in &self.map {
            if k.is_empty() {
                continue;
            }
            let hit = m == k.as_str() || m.starts_with(k) || m.contains(k.as_str());
            if !hit {
                continue;
            }
            let better = match best {
                None => true,
                Some((bk, _)) => k.len() > bk.len(),
            };
            if better {
                best = Some((k, v));
            }
        }
        best.map(|(_, v)| v.clone())
            .unwrap_or_else(|| merchant.to_string())
    }

    /// Seed common TW short names when the book is empty (init / first today).
    pub fn ensure_tw_defaults(&mut self) -> bool {
        if !self.map.is_empty() {
            return false;
        }
        let pairs = [
            ("全家便利商店", "全家"),
            ("全家", "全家"),
            ("7-ELEVEN", "7-ELEVEN"),
            ("萊爾富", "萊爾富"),
            ("OK超商", "OK超商"),
            ("全聯福利中心", "全聯"),
            ("家樂福", "家樂福"),
            ("好市多 Costco", "好市多"),
            ("好市多", "好市多"),
            ("麥當勞", "麥當勞"),
            ("肯德基 KFC", "肯德基"),
            ("肯德基", "肯德基"),
            ("鼎泰豐", "鼎泰豐"),
            ("台北捷運", "北捷"),
            ("台灣高鐵", "高鐵"),
            ("台灣中油", "中油"),
            ("屈臣氏", "屈臣氏"),
            ("康是美", "康是美"),
            ("中華電信", "中華電信"),
            ("麥味登", "麥味登"),
            ("50嵐", "50嵐"),
            ("可不可熟成紅茶", "可不可"),
            ("ibon", "ibon"),
        ];
        for (from, to) in pairs {
            self.set(from, to);
        }
        true
    }
}

fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Soft-parse alias file (quoted or bare keys).
pub fn parse_aliases_toml(s: &str) -> AliasBook {
    let mut book = AliasBook::default();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = unquote(k.trim());
        let v = unquote(v.trim());
        if !k.is_empty() && !v.is_empty() {
            book.map.insert(k, v);
        }
    }
    book
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        s.to_string()
    }
}

/// Locate aliases file next to ledger or under data dir.
pub fn find_aliases_file(ledger_path: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(parent) = ledger_path.parent() {
        if !parent.as_os_str().is_empty() {
            candidates.push(parent.join("merchant_aliases.toml"));
        }
    }
    candidates.push(AliasBook::path());
    candidates.into_iter().find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_apply() {
        let s = r#"
# comment
"全家便利商店" = "全家"
FAMILYMART = 全家
"#;
        let book = parse_aliases_toml(s);
        assert_eq!(book.apply("全家便利商店"), "全家");
        assert_eq!(book.apply("FAMILYMART"), "全家");
        assert_eq!(book.apply("unknown"), "unknown");
        assert_eq!(book.display_for("全家便利商店 臨江店"), "全家");
    }

    #[test]
    fn tw_defaults_shorten_branch_names() {
        let mut b = AliasBook::default();
        assert!(b.ensure_tw_defaults());
        assert!(!b.ensure_tw_defaults()); // already filled
        assert_eq!(b.display_for("全家便利商店 臨江店"), "全家");
        assert_eq!(b.display_for("麥當勞 忠孝店"), "麥當勞");
    }

    #[test]
    fn roundtrip_toml() {
        let mut b = AliasBook::default();
        b.set("A Store", "A");
        let parsed = parse_aliases_toml(&b.to_toml_string());
        assert_eq!(parsed.apply("A Store"), "A");
    }
}
