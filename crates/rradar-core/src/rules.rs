//! Loadable merchant rule packs from `data_dir/rules/*.yml` (simple line format).

use crate::category::{normalize_key, CategoryEngine, MerchantEntry};
use crate::paths::data_dir;
use std::path::{Path, PathBuf};

/// Soft-normalize key for merchant matching (re-export style helper).
pub fn rules_dir() -> PathBuf {
    data_dir().join("rules")
}

pub fn ensure_rules_dir() -> std::io::Result<PathBuf> {
    let d = rules_dir();
    std::fs::create_dir_all(&d)?;
    // seed example pack if empty
    let example = d.join("community.example.yml");
    if !example.exists() {
        std::fs::write(
            &example,
            r#"# ReceiptRadar merchant rule pack (YAML-lite)
# Each line: key | display | category_id
# category_id: food_dining | grocery_convenience | transport | shopping | health | utilities | entertainment | other

# example_local_cafe | 巷口咖啡 | food_dining
"#,
        )?;
    }
    Ok(d)
}

/// Parse soft YAML-like merchant lines: `key | display | category`
pub fn parse_rule_pack(text: &str) -> Vec<MerchantEntry> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if parts.len() < 3 {
            continue;
        }
        out.push(MerchantEntry {
            key: normalize_key(parts[0]),
            display: parts[1].to_string(),
            category: parts[2].to_string(),
        });
    }
    out
}

pub fn load_all_rule_packs() -> Vec<MerchantEntry> {
    let dir = rules_dir();
    let mut all = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return all;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.ends_with(".example.yml") {
            continue;
        }
        if !(name.ends_with(".yml") || name.ends_with(".yaml") || name.ends_with(".txt")) {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&p) {
            all.extend(parse_rule_pack(&text));
        }
    }
    all
}

/// Built-in seed + community packs (community overrides by appending after seed).
pub fn category_engine_with_packs() -> CategoryEngine {
    let mut eng = CategoryEngine::with_seed();
    let extra = load_all_rule_packs();
    // prepend community so they win first-match? Actually categorize iterates merchants first-to-last;
    // put community first for override.
    if !extra.is_empty() {
        let mut m = extra;
        m.append(&mut eng.merchants);
        eng.merchants = m;
    }
    eng
}

pub fn list_rule_files() -> Vec<PathBuf> {
    let dir = rules_dir();
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

pub fn install_rule_pack(src: &Path, name: Option<&str>) -> std::io::Result<PathBuf> {
    let dir = ensure_rules_dir()?;
    let fname = name.map(|s| s.to_string()).unwrap_or_else(|| {
        src.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pack.yml")
            .to_string()
    });
    let dest = dir.join(fname);
    std::fs::copy(src, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lines() {
        let t = "mycafe | 我的咖啡 | food_dining\n# comment\nbads\n";
        let v = parse_rule_pack(t);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].category, "food_dining");
    }
}
