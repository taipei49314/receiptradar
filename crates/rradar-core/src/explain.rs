//! Explain traces for CLI `--explain` and review UI.

use serde::{Deserialize, Serialize};

/// One step the pipeline took (rule hit, candidate, engine).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExplainStep {
    pub kind: String,
    pub detail: String,
}

/// Full explainability payload attached to a draft.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExplainTrace {
    pub engine_id: String,
    pub source_path: String,
    pub steps: Vec<ExplainStep>,
    pub amount_candidates: Vec<AmountCandidate>,
    pub matched_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmountCandidate {
    pub raw: String,
    pub amount_minor: i64,
    pub currency: String,
    pub rank_score: i32,
    pub reason: String,
}

impl ExplainTrace {
    pub fn new(engine_id: impl Into<String>, source_path: impl Into<String>) -> Self {
        Self {
            engine_id: engine_id.into(),
            source_path: source_path.into(),
            steps: Vec::new(),
            amount_candidates: Vec::new(),
            matched_keywords: Vec::new(),
        }
    }

    pub fn step(&mut self, kind: impl Into<String>, detail: impl Into<String>) {
        self.steps.push(ExplainStep {
            kind: kind.into(),
            detail: detail.into(),
        });
    }

    /// Human-readable multi-line dump for CLI.
    pub fn format_pretty(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "engine={}  path={}\n",
            self.engine_id, self.source_path
        ));
        for s in &self.steps {
            out.push_str(&format!("  [{kind}] {detail}\n", kind = s.kind, detail = s.detail));
        }
        if !self.amount_candidates.is_empty() {
            out.push_str("amount candidates (best first):\n");
            for (i, c) in self.amount_candidates.iter().enumerate() {
                out.push_str(&format!(
                    "  #{i} score={score} {cur} minor={minor} raw={raw:?} ({reason})\n",
                    score = c.rank_score,
                    cur = c.currency,
                    minor = c.amount_minor,
                    raw = c.raw,
                    reason = c.reason
                ));
            }
        }
        if !self.matched_keywords.is_empty() {
            out.push_str(&format!(
                "keywords: {}\n",
                self.matched_keywords.join(", ")
            ));
        }
        out
    }
}
