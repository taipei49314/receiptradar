//! OCR spike harness (PR-A04).
//!
//! Measures process latency on fixtures with the selected engine (default mock).
//! Real ONNX + device matrices land after models are pinned (PR-A05).
//!
//! Usage:
//!   cargo run -p bench-ocr -- fixtures/text
//!   cargo run -p bench-ocr -- fixtures/text --engine mock --json

use rradar_core::{process_path, CategoryEngine, ProcessOptions};
use rradar_ocr::engine_by_name;
use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Serialize)]
struct BenchRow {
    path: String,
    engine: String,
    ok: bool,
    ms: u128,
    total_minor: Option<i64>,
    currency: Option<String>,
    merchant: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct BenchReport {
    created_note: String,
    engine: String,
    rows: Vec<BenchRow>,
    p50_ms: Option<u128>,
    p95_ms: Option<u128>,
    success: usize,
    fail: usize,
}

fn main() {
    let mut args = env::args().skip(1);
    let mut root = PathBuf::from("fixtures/text");
    let mut engine = "mock".to_string();
    let mut json = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--engine" => engine = args.next().unwrap_or_else(|| "mock".into()),
            "--json" => json = true,
            s if !s.starts_with('-') => root = PathBuf::from(s),
            _ => {}
        }
    }

    let eng = match engine_by_name(&engine) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("engine error: {e}");
            std::process::exit(2);
        }
    };
    let cats = CategoryEngine::with_seed();
    let mut rows = Vec::new();
    let mut times = Vec::new();

    let paths = collect_inputs(&root);
    if paths.is_empty() {
        eprintln!("no inputs under {}", root.display());
        std::process::exit(1);
    }

    for path in paths {
        let t0 = Instant::now();
        let res = process_path(&path, eng.as_ref(), &cats, ProcessOptions::default());
        let ms = t0.elapsed().as_millis();
        match res {
            Ok(d) => {
                times.push(ms);
                rows.push(BenchRow {
                    path: path.display().to_string(),
                    engine: eng.name().into(),
                    ok: true,
                    ms,
                    total_minor: Some(d.total.value.amount_minor),
                    currency: Some(d.total.value.currency.to_string()),
                    merchant: Some(d.merchant.value),
                    error: None,
                });
            }
            Err(e) => rows.push(BenchRow {
                path: path.display().to_string(),
                engine: eng.name().into(),
                ok: false,
                ms,
                total_minor: None,
                currency: None,
                merchant: None,
                error: Some(e.to_string()),
            }),
        }
    }

    times.sort_unstable();
    let report = BenchReport {
        created_note: "A04 harness — fill docs/spike-ocr-size.md after device runs".into(),
        engine: engine.clone(),
        success: rows.iter().filter(|r| r.ok).count(),
        fail: rows.iter().filter(|r| !r.ok).count(),
        p50_ms: percentile(&times, 50),
        p95_ms: percentile(&times, 95),
        rows,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!(
            "engine={} success={} fail={} p50_ms={:?} p95_ms={:?}",
            report.engine, report.success, report.fail, report.p50_ms, report.p95_ms
        );
        for r in &report.rows {
            println!(
                "  {:>5}ms  ok={}  {}  {:?}",
                r.ms, r.ok, r.path, r.total_minor
            );
        }
    }

    if engine == "onnx" {
        eprintln!("note: onnx backend is stub until PR-A05 model pin");
    }
}

fn collect_inputs(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root) {
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

fn percentile(sorted: &[u128], p: u8) -> Option<u128> {
    if sorted.is_empty() {
        return None;
    }
    let idx = ((p as usize) * (sorted.len() - 1)) / 100;
    Some(sorted[idx])
}
