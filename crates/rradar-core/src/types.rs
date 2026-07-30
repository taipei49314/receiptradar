//! Core domain types: fields, drafts, OCR blocks.

use crate::explain::ExplainTrace;
use crate::money::Money;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldSource {
    Rule,
    Qr,
    User,
    /// Reserved for Track B L2 ML.
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field<T> {
    pub value: T,
    pub confidence: f32,
    pub source: FieldSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<T>>,
}

impl<T> Field<T> {
    pub fn new(value: T, confidence: f32, source: FieldSource) -> Self {
        Self {
            value,
            confidence,
            source,
            alternatives: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePath {
    Qr,
    Ocr,
    Mixed,
    Manual,
}

impl SourcePath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Qr => "qr",
            Self::Ocr => "ocr",
            Self::Mixed => "mixed",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBlock {
    pub text: String,
    pub confidence: f32,
}

/// Category identifier (taxonomy pack relative id).
pub type CategoryId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptDraft {
    pub id: String,
    pub captured_at: String,
    pub merchant: Field<String>,
    pub total: Field<Money>,
    pub transacted_at: Field<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax: Option<Field<Money>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<Field<String>>,
    pub category: Field<CategoryId>,
    pub raw_text: String,
    pub ocr_blocks: Vec<TextBlock>,
    pub overall_confidence: f32,
    pub explain: ExplainTrace,
    pub source_path: SourcePath,
}

impl ReceiptDraft {
    pub fn new_id() -> String {
        Ulid::new().to_string()
    }
}
