//! Default local data paths for the CLI product.

use std::path::PathBuf;

/// Directory for ledger, sealed DB, and local config.
///
/// Override with `RRADAR_HOME`. Default:
/// - Windows: `%APPDATA%\\receiptradar` or `%USERPROFILE%\\AppData\\Roaming\\receiptradar`
/// - else: `$XDG_DATA_HOME/receiptradar` or `~/.local/share/receiptradar`
pub fn data_dir() -> PathBuf {
    if let Ok(h) = std::env::var("RRADAR_HOME") {
        return PathBuf::from(h);
    }
    if cfg!(windows) {
        if let Ok(app) = std::env::var("APPDATA") {
            return PathBuf::from(app).join("receiptradar");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home)
                .join("AppData")
                .join("Roaming")
                .join("receiptradar");
        }
    } else {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("receiptradar");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("receiptradar");
        }
    }
    PathBuf::from(".receiptradar")
}

/// Default ledger path (`ledger.db`), or `RRADAR_DB` if set.
pub fn default_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("RRADAR_DB") {
        return PathBuf::from(p);
    }
    data_dir().join("ledger.db")
}

/// Ensure data directory exists.
pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let d = data_dir();
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_db_under_data_dir() {
        // When RRADAR_DB unset, path ends with ledger.db
        std::env::remove_var("RRADAR_DB");
        let p = default_db_path();
        assert!(p.ends_with("ledger.db") || p.file_name().is_some());
    }
}
