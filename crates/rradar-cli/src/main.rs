//! `rradar` CLI — local-first receipt → ledger (Track A).

use rradar_core::{
    create_backup, open_ledger_auto, process_path, restore_backup, save_sealed,
    transactions_to_csv, transactions_to_json, write_restored_db, CategoryEngine, Iso4217,
    ProcessOptions, ReceiptDraft,
};
use rradar_ocr::engine_by_name;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        return ExitCode::SUCCESS;
    }
    let result = match args[0].as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("{}", rradar_core::identify());
            Ok(())
        }
        "process" => cmd_process(&args[1..]),
        "list" => cmd_list(&args[1..]),
        "stats" => cmd_stats(&args[1..]),
        "export" => cmd_export(&args[1..]),
        "backup" => cmd_backup(&args[1..]),
        "seal" => cmd_seal(&args[1..]),
        other => Err(format!("unknown command `{other}`")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
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
    let mut confirm = false;
    let mut db: Option<PathBuf> = None;
    let mut passphrase: Option<String> = None;
    let mut force = false;
    let mut notes: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--explain" => explain = true,
            "--json" => json = true,
            "--confirm" => confirm = true,
            "--force" => force = true,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("--engine needs value")?.clone();
            }
            "--qr" => {
                i += 1;
                qr = Some(args.get(i).ok_or("--qr needs value")?.clone());
            }
            "--currency" => {
                i += 1;
                let c = args.get(i).ok_or("--currency needs value")?;
                currency = Iso4217::parse(c).ok_or_else(|| format!("bad currency {c}"))?;
            }
            "--db" => {
                i += 1;
                db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
            }
            "--passphrase" => {
                i += 1;
                passphrase = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
            }
            "--notes" => {
                i += 1;
                notes = Some(args.get(i).ok_or("--notes needs value")?.clone());
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

    let path = path.ok_or("usage: rradar process <path> [options]")?;
    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let categories = CategoryEngine::with_seed();
    let opts = ProcessOptions {
        default_currency: currency,
        qr_payload: qr,
        ..Default::default()
    };

    let draft = process_path(&path, eng.as_ref(), &categories, opts).map_err(|e| e.to_string())?;
    print_draft(&draft, explain, json && !confirm);

    if confirm {
        let db_path = db.ok_or("--confirm requires --db <path>")?;
        let hash = rradar_core::preprocess::content_hash(&std::fs::read(&path).unwrap_or_default());
        let (ledger, tmp) =
            open_ledger_auto(&db_path, passphrase.as_deref()).map_err(|e| e.to_string())?;
        let result = ledger
            .confirm_draft(&draft, Some(&hash), notes.as_deref(), force)
            .map_err(|e| e.to_string())?;
        if let Some(ref d) = result.dedupe {
            eprintln!("dedupe {:?}: {} ({})", d.level, d.message, d.existing_id);
        }
        if result.inserted {
            println!("confirmed id={}", result.transaction.id);
        } else {
            println!(
                "not inserted (hard dedupe); existing id={}",
                result.transaction.id
            );
        }
        // Re-seal if working from sealed DB or passphrase provided for .rrsealed target
        if db_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("rrsealed"))
            .unwrap_or(false)
        {
            let pass = passphrase.ok_or("sealed db needs --passphrase to save")?;
            save_sealed(&ledger, &db_path, &pass).map_err(|e| e.to_string())?;
        }
        if let Some(t) = tmp {
            let _ = std::fs::remove_file(t);
        }
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }
    }
    Ok(())
}

fn cmd_list(args: &[String]) -> Result<(), String> {
    let (db, pass, limit) = parse_db_args(args)?;
    let (ledger, tmp) = open_ledger_auto(&db, pass.as_deref()).map_err(|e| e.to_string())?;
    let rows = ledger
        .list_transactions(limit, 0)
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        transactions_to_json(&rows).map_err(|e| e.to_string())?
    );
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_stats(args: &[String]) -> Result<(), String> {
    let mut db: Option<PathBuf> = None;
    let mut pass: Option<String> = None;
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
            }
            "--passphrase" => {
                i += 1;
                pass = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
            }
            "--year" => {
                i += 1;
                year = Some(
                    args.get(i)
                        .ok_or("--year needs value")?
                        .parse()
                        .map_err(|_| "bad year")?,
                );
            }
            "--month" => {
                i += 1;
                month = Some(
                    args.get(i)
                        .ok_or("--month needs value")?
                        .parse()
                        .map_err(|_| "bad month")?,
                );
            }
            other => return Err(format!("unknown arg {other}")),
        }
        i += 1;
    }
    let db = db.ok_or("--db required")?;
    let year = year.ok_or("--year required")?;
    let month = month.ok_or("--month required")?;
    let (ledger, tmp) = open_ledger_auto(&db, pass.as_deref()).map_err(|e| e.to_string())?;
    let stats = ledger
        .stats_by_currency_month(year, month)
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_export(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: rradar export <csv|json> --db PATH [-o file]".into());
    }
    let kind = args[0].as_str();
    let mut db: Option<PathBuf> = None;
    let mut pass: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
            }
            "--passphrase" => {
                i += 1;
                pass = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
            }
            "-o" | "--output" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("-o needs value")?));
            }
            other => return Err(format!("unknown arg {other}")),
        }
        i += 1;
    }
    let db = db.ok_or("--db required")?;
    let (ledger, tmp) = open_ledger_auto(&db, pass.as_deref()).map_err(|e| e.to_string())?;
    let rows = ledger.export_all().map_err(|e| e.to_string())?;
    let body = match kind {
        "csv" => transactions_to_csv(&rows).map_err(|e| e.to_string())?,
        "json" => transactions_to_json(&rows).map_err(|e| e.to_string())?,
        _ => return Err("export kind must be csv or json".into()),
    };
    if let Some(p) = out {
        std::fs::write(p, body).map_err(|e| e.to_string())?;
    } else {
        print!("{body}");
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_backup(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: rradar backup <create|restore> ...".into());
    }
    match args[0].as_str() {
        "create" => {
            let mut db: Option<PathBuf> = None;
            let mut pass: Option<String> = None;
            let mut out: Option<PathBuf> = None;
            let mut db_pass: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--db" => {
                        i += 1;
                        db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
                    }
                    "--passphrase" => {
                        i += 1;
                        pass = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
                    }
                    "--db-passphrase" => {
                        i += 1;
                        db_pass = Some(args.get(i).ok_or("--db-passphrase needs value")?.clone());
                    }
                    "-o" | "--output" => {
                        i += 1;
                        out = Some(PathBuf::from(args.get(i).ok_or("-o needs value")?));
                    }
                    other => return Err(format!("unknown {other}")),
                }
                i += 1;
            }
            let db = db.ok_or("--db required")?;
            let pass = pass.ok_or("--passphrase required for backup")?;
            let out = out.ok_or("-o required")?;
            let (ledger, tmp) =
                open_ledger_auto(&db, db_pass.as_deref()).map_err(|e| e.to_string())?;
            // Use reduced Argon2 for CI-friendly default on create; design m=64MiB is default in create_backup_default
            let bytes = create_backup(&ledger, &pass, rradar_core::crypto::ARGON2_M_KIB)
                .map_err(|e| e.to_string())?;
            std::fs::write(out, bytes).map_err(|e| e.to_string())?;
            println!("backup written");
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            Ok(())
        }
        "restore" => {
            let mut input: Option<PathBuf> = None;
            let mut pass: Option<String> = None;
            let mut db: Option<PathBuf> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--in" | "-i" => {
                        i += 1;
                        input = Some(PathBuf::from(args.get(i).ok_or("--in needs value")?));
                    }
                    "--passphrase" => {
                        i += 1;
                        pass = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
                    }
                    "--db" => {
                        i += 1;
                        db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
                    }
                    other => return Err(format!("unknown {other}")),
                }
                i += 1;
            }
            let input = input.ok_or("--in required")?;
            let pass = pass.ok_or("--passphrase required")?;
            let db = db.ok_or("--db required (output sqlite path)")?;
            let sealed = std::fs::read(input).map_err(|e| e.to_string())?;
            let restored = restore_backup(&pass, &sealed).map_err(|e| e.to_string())?;
            let sqlite = restored
                .sqlite_bytes
                .ok_or("backup missing ledger.sqlite")?;
            write_restored_db(&db, &sqlite).map_err(|e| e.to_string())?;
            println!(
                "restored {} txs -> {}",
                restored.manifest.transaction_count,
                db.display()
            );
            Ok(())
        }
        other => Err(format!("unknown backup subcommand {other}")),
    }
}

fn cmd_seal(args: &[String]) -> Result<(), String> {
    // rradar seal --db ledger.db --out ledger.rrsealed --passphrase X
    let mut db: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut pass: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
            }
            "--out" | "-o" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("--out needs value")?));
            }
            "--passphrase" => {
                i += 1;
                pass = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
            }
            other => return Err(format!("unknown {other}")),
        }
        i += 1;
    }
    let db = db.ok_or("--db required")?;
    let out = out.ok_or("--out required")?;
    let pass = pass.ok_or("--passphrase required")?;
    rradar_core::seal_db_file(&db, &out, &pass).map_err(|e| e.to_string())?;
    println!("sealed {}", out.display());
    Ok(())
}

fn parse_db_args(args: &[String]) -> Result<(PathBuf, Option<String>, usize), String> {
    let mut db: Option<PathBuf> = None;
    let mut pass: Option<String> = None;
    let mut limit = 100usize;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
            }
            "--passphrase" => {
                i += 1;
                pass = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
            }
            "--limit" => {
                i += 1;
                limit = args
                    .get(i)
                    .ok_or("--limit needs value")?
                    .parse()
                    .map_err(|_| "bad limit")?;
            }
            other => return Err(format!("unknown arg {other}")),
        }
        i += 1;
    }
    Ok((db.ok_or("--db required")?, pass, limit))
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
        "merchant:    {}  (conf={:.2})",
        draft.merchant.value, draft.merchant.confidence
    );
    println!(
        "total:       {} {}  (minor={})",
        draft.total.value.currency,
        draft.total.value.display_major(),
        draft.total.value.amount_minor
    );
    println!("date:        {}", draft.transacted_at.value);
    if let Some(ref inv) = draft.invoice_id {
        println!("invoice:     {}", inv.value);
    }
    println!("category:    {}", draft.category.value);
    println!("confidence:  {:.2}", draft.overall_confidence);
    if explain {
        println!("\n--- explain ---");
        print!("{}", draft.explain.format_pretty());
    }
}

fn print_help() {
    println!(
        "\
rradar — ReceiptRadar CLI
{ident}

COMMANDS:
  version
  process <path> [--explain] [--json] [--engine mock|onnx]
                 [--qr PAYLOAD] [--currency TWD]
                 [--confirm --db PATH] [--force] [--passphrase P] [--notes N]
  list   --db PATH [--limit N] [--passphrase P]
  stats  --db PATH --year Y --month M [--passphrase P]
  export csv|json --db PATH [-o file] [--passphrase P]
  backup create  --db PATH --passphrase P -o backup.rradar [--db-passphrase P]
  backup restore --in backup.rradar --passphrase P --db out.db
  seal   --db ledger.db --out ledger.rrsealed --passphrase P

Local-first. No cloud. No account.
",
        ident = rradar_core::identify()
    );
}

#[allow(dead_code)]
fn _keep_path_import(_: &Path) {}
