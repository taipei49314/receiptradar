//! `rradar` CLI — local-first receipt processing (Track A through A12).

use rradar_core::{
    process_path, CategoryEngine, Iso4217, ProcessOptions, ReceiptDraft,
};
use rradar_ocr::engine_by_name;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        return ExitCode::SUCCESS;
    }
    match args[0].as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            ExitCode::SUCCESS
        }
        "version" | "--version" | "-V" => {
            println!("{}", rradar_core::identify());
            ExitCode::SUCCESS
        }
        "process" => match cmd_process(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
        other => {
            eprintln!("error: unknown command `{other}`\n");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn cmd_process(args: &[String]) -> Result<(), String> {
    let mut path: Option<PathBuf> = None;
    let mut explain = false;
    let mut engine = "mock".to_string();
    let mut qr: Option<String> = None;
    let mut json = false;
    let mut currency = Iso4217::TWD;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--explain" => explain = true,
            "--json" => json = true,
            "--engine" => {
                i += 1;
                engine = args
                    .get(i)
                    .ok_or("--engine requires a value")?
                    .clone();
            }
            "--qr" => {
                i += 1;
                qr = Some(args.get(i).ok_or("--qr requires a value")?.clone());
            }
            "--currency" => {
                i += 1;
                let c = args.get(i).ok_or("--currency requires a value")?;
                currency = Iso4217::parse(c).ok_or_else(|| format!("invalid currency: {c}"))?;
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            s => {
                if path.is_some() {
                    return Err("multiple input paths".into());
                }
                path = Some(PathBuf::from(s));
            }
        }
        i += 1;
    }

    let path = path.ok_or_else(|| {
        "usage: rradar process <path> [--explain] [--json] [--engine mock|onnx] [--qr PAYLOAD] [--currency TWD]"
            .to_string()
    })?;

    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let categories = CategoryEngine::with_seed();
    let opts = ProcessOptions {
        default_currency: currency,
        qr_payload: qr,
        ..Default::default()
    };

    let draft = process_path(&path, eng.as_ref(), &categories, opts).map_err(|e| e.to_string())?;
    print_draft(&draft, explain, json);
    Ok(())
}

fn print_draft(draft: &ReceiptDraft, explain: bool, json: bool) {
    if json {
        match serde_json::to_string_pretty(draft) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("json error: {e}"),
        }
        return;
    }

    println!("id:          {}", draft.id);
    println!("source:      {}", draft.source_path.as_str());
    println!(
        "merchant:    {}  (conf={:.2}, {:?})",
        draft.merchant.value, draft.merchant.confidence, draft.merchant.source
    );
    println!(
        "total:       {} {}  (minor={}, conf={:.2})",
        draft.total.value.currency,
        draft.total.value.display_major(),
        draft.total.value.amount_minor,
        draft.total.confidence
    );
    println!(
        "date:        {}  (conf={:.2})",
        draft.transacted_at.value, draft.transacted_at.confidence
    );
    if let Some(ref inv) = draft.invoice_id {
        println!("invoice:     {}  (conf={:.2})", inv.value, inv.confidence);
    }
    println!(
        "category:    {}  (conf={:.2})",
        draft.category.value, draft.category.confidence
    );
    println!("confidence:  {:.2}", draft.overall_confidence);

    if explain {
        println!("\n--- explain ---");
        print!("{}", draft.explain.format_pretty());
        println!("--- raw text ---");
        println!("{}", draft.raw_text);
    }
}

fn print_help() {
    println!(
        "\
rradar — ReceiptRadar CLI
{ident}

USAGE:
    rradar <COMMAND>

COMMANDS:
    version     Print core version
    process     Process a receipt image or text fixture
    help        Show this help

PROCESS:
    rradar process <path> [options]

    <path>        Image bytes, .txt OCR fixture, or file with .ocr.txt sidecar
    --explain     Print rule hits, amount candidates, engine path
    --json        Emit ReceiptDraft as JSON
    --engine      mock (default) | onnx (stub until models pinned)
    --qr          Offline TW e-invoice left-QR payload string
    --currency    Default currency fallback (TWD|USD|JPY|…)

EXAMPLES:
    rradar process fixtures/text/familymart_89.txt --explain
    rradar process fixtures/qr/tw_einvoice_sample_01.payload.txt --qr \"$(Get-Content -Raw ...)\" 
    rradar process photo.jpg --engine mock

Local-first. No cloud. No account.
",
        ident = rradar_core::identify()
    );
}
