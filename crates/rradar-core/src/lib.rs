//! ReceiptRadar core library — local-first receipt → draft pipeline.

#![deny(unsafe_code)]

pub mod category;
pub mod explain;
pub mod extract;
pub mod money;
pub mod pipeline;
pub mod preprocess;
pub mod qr;
pub mod types;

pub use category::CategoryEngine;
pub use explain::ExplainTrace;
pub use money::{sum_same_currency, Iso4217, Money, MoneyError};
pub use pipeline::{process_bytes, process_path, ProcessError, ProcessOptions};
pub use types::{Field, FieldSource, ReceiptDraft, SourcePath, TextBlock};

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
