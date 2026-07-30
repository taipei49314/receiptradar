//! Optional local config file (`config.toml` under data dir).

use crate::money::Iso4217;
use crate::paths::data_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Default currency code (e.g. TWD).
    #[serde(default = "default_currency_str")]
    pub default_currency: String,
    /// Soft session: unused on CLI today; reserved.
    #[serde(default = "default_list_limit")]
    pub list_limit: usize,
}

fn default_currency_str() -> String {
    "TWD".into()
}

fn default_list_limit() -> usize {
    50
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_currency: default_currency_str(),
            list_limit: default_list_limit(),
        }
    }
}

impl AppConfig {
    pub fn path() -> PathBuf {
        data_dir().join("config.toml")
    }

    pub fn load() -> Self {
        let p = Self::path();
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(c) = toml_soft_parse(&s) {
                return c;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let p = Self::path();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, self.to_toml_string())
    }

    pub fn currency(&self) -> Iso4217 {
        Iso4217::parse(&self.default_currency).unwrap_or(Iso4217::TWD)
    }

    fn to_toml_string(&self) -> String {
        format!(
            "# ReceiptRadar local config\ndefault_currency = \"{}\"\nlist_limit = {}\n",
            self.default_currency, self.list_limit
        )
    }
}

/// Minimal TOML parse for our two keys (avoid heavy toml dep).
fn toml_soft_parse(s: &str) -> Result<AppConfig, ()> {
    let mut c = AppConfig::default();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"');
            match k {
                "default_currency" => c.default_currency = v.to_string(),
                "list_limit" => {
                    if let Ok(n) = v.parse() {
                        c.list_limit = n;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_string() {
        let c = AppConfig {
            default_currency: "USD".into(),
            list_limit: 20,
        };
        let s = c.to_toml_string();
        let p = toml_soft_parse(&s).unwrap();
        assert_eq!(p.default_currency, "USD");
        assert_eq!(p.list_limit, 20);
        assert_eq!(p.currency(), Iso4217::USD);
    }
}
