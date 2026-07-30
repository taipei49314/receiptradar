//! ReceiptRadar core library.
//!
//! Local-first receipt → ledger types and orchestration stubs.
//! Real OCR, extractors, and SQLite land in later Track A PRs.

#![deny(unsafe_code)]

/// Crate version (workspace package version).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Project codename / product id used in backup headers and docs.
pub const PRODUCT_ID: &str = "receiptradar";

/// Returns a short identify string for CLI / FFI hello paths.
pub fn identify() -> String {
    format!("{PRODUCT_ID} core {VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_contains_product_id() {
        let s = identify();
        assert!(s.contains(PRODUCT_ID));
        assert!(s.contains(VERSION));
    }
}
