//! ReceiptRadar core library — local-first receipt → ledger.

#![deny(unsafe_code)]

pub mod attachments;
pub mod budget;
pub mod category;
pub mod config;
pub mod crypto;
pub mod explain;
pub mod export;
pub mod extract;
pub mod handoff;
pub mod ledger;
pub mod money;
pub mod paths;
pub mod pipeline;
pub mod preprocess;
pub mod qr;
pub mod report;
pub mod rules;
pub mod sealed;
pub mod types;

pub use attachments::{
    attachments_root_for_db, collect_attachment_files, ensure_attachments_root, normalize_tags,
    remove_stored_attachment, resolve_attachment_path, store_attachment, store_attachment_bytes,
    write_attachment_files, AttachmentError,
};
pub use budget::{
    budget_markdown_section, budget_status_month, parse_budgets_toml, BudgetBook, BudgetLine,
    BudgetStatus,
};
pub use category::CategoryEngine;
pub use config::AppConfig;
pub use explain::ExplainTrace;
pub use export::{
    create_backup, create_backup_default_params, inspect_backup, restore_backup,
    transactions_from_backup, transactions_to_csv, transactions_to_json, verify_backup,
    write_restored_attachments, write_restored_db, BackupFileInfo, BackupInspect, BackupManifest,
    ExportError, RestoredBackup,
};
pub use handoff::{apply_handoff_merge, create_handoff, inspect_handoff, write_handoff_file};
pub use ledger::{
    apply_edits, CategoryStat, ConfirmResult, CurrencyMonthStat, DedupeLevel, DedupeWarning,
    Ledger, LedgerError, Transaction, TxFilter, TxUpdate, UserEdits, LEDGER_SCHEMA_VERSION,
};
pub use money::{sum_same_currency, Iso4217, Money, MoneyError};
pub use paths::{data_dir, default_db_path, ensure_data_dir, ensure_inbox_dir, inbox_dir};
pub use pipeline::{process_bytes, process_path, utc_now_iso, ProcessError, ProcessOptions};
pub use report::{monthly_markdown, monthly_markdown_with_budgets};
pub use rules::{
    category_engine_with_packs, ensure_rules_dir, install_rule_pack, list_rule_files, rules_dir,
};
pub use sealed::{open_ledger_auto, save_sealed, seal_db_file};
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
