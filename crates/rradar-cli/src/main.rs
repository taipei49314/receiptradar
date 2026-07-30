//! `rradar` — complete local-first receipt ledger CLI.

use rradar_core::{
    apply_edits, create_backup, data_dir, default_db_path, ensure_data_dir, open_ledger_auto,
    process_path, restore_backup, save_sealed, transactions_to_csv, transactions_to_json,
    write_restored_db, CategoryEngine, Iso4217, Money, ProcessOptions, ReceiptDraft, Transaction,
    TxUpdate, UserEdits, PRODUCT_ID, VERSION,
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
            println!("{PRODUCT_ID} {VERSION}");
            Ok(())
        }
        "init" => cmd_init(&args[1..]),
        "doctor" => cmd_doctor(&args[1..]),
        "process" | "add" => cmd_process(&args[1..]),
        "list" | "ls" => cmd_list(&args[1..]),
        "show" => cmd_show(&args[1..]),
        "delete" | "rm" => cmd_delete(&args[1..]),
        "edit" => cmd_edit(&args[1..]),
        "stats" => cmd_stats(&args[1..]),
        "categories" | "cats" => cmd_categories(),
        "export" => cmd_export(&args[1..]),
        "backup" => cmd_backup(&args[1..]),
        "seal" => cmd_seal(&args[1..]),
        "unseal" => cmd_unseal(&args[1..]),
        "path" => {
            println!("home\t{}", data_dir().display());
            println!("db\t{}", default_db_path().display());
            Ok(())
        }
        other => Err(format!("unknown command `{other}` — try `rradar help`")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

// --- shared flag helpers ---------------------------------------------------

struct DbFlags {
    db: PathBuf,
    passphrase: Option<String>,
}

fn open_db(flags: &DbFlags) -> Result<(rradar_core::Ledger, Option<PathBuf>), String> {
    if let Some(parent) = flags.db.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    open_ledger_auto(&flags.db, flags.passphrase.as_deref()).map_err(|e| e.to_string())
}

fn maybe_reseal(
    flags: &DbFlags,
    ledger: &rradar_core::Ledger,
    tmp: Option<PathBuf>,
) -> Result<(), String> {
    let sealed = flags
        .db
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("rrsealed"))
        .unwrap_or(false);
    if sealed {
        let pass = flags
            .passphrase
            .as_deref()
            .ok_or("sealed db requires --passphrase to save changes")?;
        save_sealed(ledger, &flags.db, pass).map_err(|e| e.to_string())?;
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

// --- commands --------------------------------------------------------------

fn cmd_init(_args: &[String]) -> Result<(), String> {
    let dir = ensure_data_dir().map_err(|e| e.to_string())?;
    let db = default_db_path();
    let _ledger = rradar_core::Ledger::open(&db).map_err(|e| e.to_string())?;
    println!("initialized");
    println!("  home: {}", dir.display());
    println!("  db:   {}", db.display());
    println!("next: rradar process <receipt.txt|image> --confirm");
    Ok(())
}

fn cmd_doctor(_args: &[String]) -> Result<(), String> {
    println!("receiptradar doctor");
    println!("  version:  {VERSION}");
    println!("  home:     {}", data_dir().display());
    println!("  db:       {}", default_db_path().display());
    let db = default_db_path();
    if db.is_file() {
        match rradar_core::Ledger::open(&db) {
            Ok(l) => println!("  ledger:   ok ({} transactions)", l.count().unwrap_or(-1)),
            Err(e) => println!("  ledger:   error ({e})"),
        }
    } else {
        println!("  ledger:   missing (run `rradar init`)");
    }
    let models =
        PathBuf::from(std::env::var("RRADAR_MODELS_DIR").unwrap_or_else(|_| "models".into()));
    println!(
        "  models:   {} ({})",
        models.display(),
        if models.is_dir() {
            "dir exists"
        } else {
            "not found — mock OCR is default"
        }
    );
    println!("  engines:  mock (default), onnx (needs pinned models + ORT)");
    println!("  privacy:  local-first; no network required for core path");
    Ok(())
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
    let mut merchant: Option<String> = None;
    let mut amount_major: Option<String> = None;
    let mut category: Option<String> = None;
    let mut date: Option<String> = None;
    let mut quiet = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--explain" => explain = true,
            "--json" => json = true,
            "--confirm" | "-c" => confirm = true,
            "--force" => force = true,
            "--quiet" | "-q" => quiet = true,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("--engine needs value")?.clone();
            }
            "--qr" => {
                i += 1;
                qr = Some(args.get(i).ok_or("--qr needs value")?.clone());
            }
            "--qr-file" => {
                i += 1;
                let p = args.get(i).ok_or("--qr-file needs value")?;
                qr = Some(
                    std::fs::read_to_string(p)
                        .map_err(|e| e.to_string())?
                        .trim()
                        .to_string(),
                );
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
            "--passphrase" | "-p" => {
                i += 1;
                passphrase = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
            }
            "--notes" => {
                i += 1;
                notes = Some(args.get(i).ok_or("--notes needs value")?.clone());
            }
            "--merchant" => {
                i += 1;
                merchant = Some(args.get(i).ok_or("--merchant needs value")?.clone());
            }
            "--amount" => {
                i += 1;
                amount_major = Some(args.get(i).ok_or("--amount needs value")?.clone());
            }
            "--category" => {
                i += 1;
                category = Some(args.get(i).ok_or("--category needs value")?.clone());
            }
            "--date" => {
                i += 1;
                date = Some(args.get(i).ok_or("--date needs value")?.clone());
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

    let path = path.ok_or(
        "usage: rradar process <path> [--confirm] [--explain] [--merchant …] [--amount 89.00]",
    )?;
    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let categories = CategoryEngine::with_seed();
    let opts = ProcessOptions {
        default_currency: currency,
        qr_payload: qr,
        ..Default::default()
    };

    let mut draft =
        process_path(&path, eng.as_ref(), &categories, opts).map_err(|e| e.to_string())?;

    // User overrides before display / confirm
    let mut edits = UserEdits {
        merchant,
        notes: notes.clone(),
        category,
        transacted_at: date,
        ..Default::default()
    };
    if let Some(ref a) = amount_major {
        let m = Money::from_major_str(a, currency).map_err(|e| e.to_string())?;
        edits.amount_minor = Some(m.amount_minor);
        edits.currency = Some(currency.to_string());
    }
    apply_edits(&mut draft, &edits);
    // re-categorize if merchant override and no explicit category
    if edits.merchant.is_some() && edits.category.is_none() {
        let mut ex = draft.explain.clone();
        draft.category = categories.categorize(&draft.merchant.value, &draft.raw_text, &mut ex);
        draft.explain = ex;
    }

    if !quiet {
        print_draft(&draft, explain, json && !confirm);
    } else if json && !confirm {
        print_draft(&draft, false, true);
    }

    if confirm {
        let db_path = db.unwrap_or_else(default_db_path);
        let _ = ensure_data_dir();
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let hash = rradar_core::preprocess::content_hash(&std::fs::read(&path).unwrap_or_default());
        let flags = DbFlags {
            db: db_path,
            passphrase,
        };
        let (ledger, tmp) = open_db(&flags)?;
        let result = ledger
            .confirm_draft(&draft, Some(&hash), notes.as_deref(), force)
            .map_err(|e| e.to_string())?;
        if let Some(ref d) = result.dedupe {
            eprintln!("dedupe {:?}: {} ({})", d.level, d.message, d.existing_id);
        }
        if result.inserted {
            println!("confirmed\t{}", result.transaction.id);
        } else {
            println!(
                "skipped\t{}\t(hard dedupe; use --force)",
                result.transaction.id
            );
        }
        maybe_reseal(&flags, &ledger, tmp)?;
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
    let mut json = false;
    let mut limit = 50usize;
    let mut offset = 0usize;
    let mut currency: Option<String> = None;
    let mut query: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--limit" => {
                i += 1;
                limit = args
                    .get(i)
                    .ok_or("--limit needs value")?
                    .parse()
                    .map_err(|_| "bad limit")?;
            }
            "--offset" => {
                i += 1;
                offset = args
                    .get(i)
                    .ok_or("--offset needs value")?
                    .parse()
                    .map_err(|_| "bad offset")?;
            }
            "--currency" => {
                i += 1;
                currency = Some(args.get(i).ok_or("--currency needs value")?.clone());
            }
            "--query" | "-q" => {
                i += 1;
                query = Some(args.get(i).ok_or("--query needs value")?.clone());
            }
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;

    let (ledger, tmp) = open_db(&flags)?;
    let rows = ledger
        .list_filtered(limit, offset, currency.as_deref(), query.as_deref())
        .map_err(|e| e.to_string())?;
    if json {
        println!(
            "{}",
            transactions_to_json(&rows).map_err(|e| e.to_string())?
        );
    } else {
        print_table(&rows);
        eprintln!("({} rows, db={})", rows.len(), flags.db.display());
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn extract_db_from_all(args: &[String]) -> Result<DbFlags, String> {
    let mut db = None;
    let mut passphrase = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                db = Some(PathBuf::from(args.get(i).ok_or("--db needs value")?));
            }
            "--passphrase" | "-p" => {
                i += 1;
                passphrase = Some(args.get(i).ok_or("--passphrase needs value")?.clone());
            }
            _ => {}
        }
        i += 1;
    }
    Ok(DbFlags {
        db: db.unwrap_or_else(default_db_path),
        passphrase,
    })
}

fn cmd_show(args: &[String]) -> Result<(), String> {
    let id = args.first().ok_or("usage: rradar show <id>")?;
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let tx = ledger.get_transaction(id).map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&tx).unwrap_or_default());
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_delete(args: &[String]) -> Result<(), String> {
    let id = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or("usage: rradar delete <id> --yes")?
        .clone();
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    if !yes {
        return Err("refusing to delete without --yes".into());
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let ok = ledger.delete_transaction(&id).map_err(|e| e.to_string())?;
    if !ok {
        return Err(format!("not found: {id}"));
    }
    println!("deleted\t{id}");
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

fn cmd_edit(args: &[String]) -> Result<(), String> {
    let id = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or(
            "usage: rradar edit <id> [--merchant M] [--amount X] [--currency C] [--category K] [--notes N] [--date YYYY-MM-DD]",
        )?
        .clone();
    let mut merchant = None;
    let mut amount = None;
    let mut currency = None;
    let mut category = None;
    let mut notes = None;
    let mut date = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--merchant" => {
                i += 1;
                merchant = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--amount" => {
                i += 1;
                amount = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--currency" => {
                i += 1;
                currency = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--category" => {
                i += 1;
                category = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--notes" => {
                i += 1;
                notes = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--date" => {
                i += 1;
                date = Some(args.get(i).ok_or("needs value")?.clone());
            }
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let existing = ledger.get_transaction(&id).map_err(|e| e.to_string())?;
    let cur = currency
        .as_deref()
        .and_then(Iso4217::parse)
        .or_else(|| Iso4217::parse(&existing.currency))
        .unwrap_or(Iso4217::TWD);
    let amount_minor = if let Some(ref a) = amount {
        Some(
            Money::from_major_str(a, cur)
                .map_err(|e| e.to_string())?
                .amount_minor,
        )
    } else {
        None
    };
    let tx = ledger
        .update_transaction(
            &id,
            &TxUpdate {
                merchant,
                amount_minor,
                currency,
                category,
                notes,
                transacted_at: date,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&tx).unwrap_or_default());
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

fn cmd_stats(args: &[String]) -> Result<(), String> {
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let mut all = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--all" => all = true,
            "--year" => {
                i += 1;
                year = Some(
                    args.get(i)
                        .ok_or("needs value")?
                        .parse()
                        .map_err(|_| "bad year")?,
                );
            }
            "--month" => {
                i += 1;
                month = Some(
                    args.get(i)
                        .ok_or("needs value")?
                        .parse()
                        .map_err(|_| "bad month")?,
                );
            }
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let stats = if all {
        ledger.stats_by_currency_all().map_err(|e| e.to_string())?
    } else {
        let (y, m) = match (year, month) {
            (Some(y), Some(m)) => (y, m),
            _ => current_year_month(),
        };
        println!("period\t{y:04}-{m:02}");
        ledger
            .stats_by_currency_month(y, m)
            .map_err(|e| e.to_string())?
    };
    if stats.is_empty() {
        println!("(no transactions)");
    } else {
        for s in &stats {
            let major = Money::new(
                s.total_minor,
                Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD),
            )
            .display_major();
            println!(
                "{}\t{}\tcount={}\tminor={}",
                s.currency, major, s.count, s.total_minor
            );
        }
        println!("note\tcurrencies are never summed together");
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn current_year_month() -> (i32, u32) {
    // reuse pipeline civil date from UTC now
    let iso = rradar_core::utc_now_iso();
    let y: i32 = iso.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let m: u32 = iso.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    (y, m)
}

fn cmd_categories() -> Result<(), String> {
    println!("id\tzh-TW\ten");
    let zh: std::collections::HashMap<_, _> = rradar_core::category::categories_zh_tw()
        .into_iter()
        .collect();
    for (id, en) in rradar_core::category::categories_en() {
        let z = zh.get(id).copied().unwrap_or("");
        println!("{id}\t{z}\t{en}");
    }
    Ok(())
}

fn cmd_export(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: rradar export <csv|json> [-o file]".into());
    }
    let kind = args[0].as_str();
    let mut out: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("-o needs value")?));
            }
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let rows = ledger.export_all().map_err(|e| e.to_string())?;
    let body = match kind {
        "csv" => transactions_to_csv(&rows).map_err(|e| e.to_string())?,
        "json" => transactions_to_json(&rows).map_err(|e| e.to_string())?,
        _ => return Err("export kind must be csv or json".into()),
    };
    if let Some(p) = out {
        std::fs::write(&p, body.as_bytes()).map_err(|e| e.to_string())?;
        println!("wrote\t{}", p.display());
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
            let mut pass = None;
            let mut out = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--passphrase" | "-p" => {
                        i += 1;
                        pass = Some(args.get(i).ok_or("needs value")?.clone());
                    }
                    "-o" | "--output" => {
                        i += 1;
                        out = Some(PathBuf::from(args.get(i).ok_or("needs value")?));
                    }
                    _ => {}
                }
                i += 1;
            }
            let pass = pass.ok_or("--passphrase required")?;
            let out = out.unwrap_or_else(|| {
                data_dir().join(format!(
                    "backup-{}.rradar",
                    rradar_core::utc_now_iso()
                        .replace(':', "")
                        .replace('T', "-")
                ))
            });
            let _ = ensure_data_dir();
            let flags = extract_db_from_all(args)?;
            let (ledger, tmp) = open_db(&flags)?;
            // Faster KDF for CLI convenience on small machines? Keep design 64MiB.
            let m = if std::env::var("RRADAR_FAST_BACKUP").is_ok() {
                8
            } else {
                rradar_core::crypto::ARGON2_M_KIB
            };
            let bytes = create_backup(&ledger, &pass, m).map_err(|e| e.to_string())?;
            if let Some(parent) = out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
            println!("backup\t{}", out.display());
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            Ok(())
        }
        "restore" => {
            let mut input = None;
            let mut pass = None;
            let mut db = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--in" | "-i" => {
                        i += 1;
                        input = Some(PathBuf::from(args.get(i).ok_or("needs value")?));
                    }
                    "--passphrase" | "-p" => {
                        i += 1;
                        pass = Some(args.get(i).ok_or("needs value")?.clone());
                    }
                    "--db" => {
                        i += 1;
                        db = Some(PathBuf::from(args.get(i).ok_or("needs value")?));
                    }
                    _ => {}
                }
                i += 1;
            }
            let input = input.ok_or("--in required")?;
            let pass = pass.ok_or("--passphrase required")?;
            let db = db.unwrap_or_else(default_db_path);
            let sealed = std::fs::read(input).map_err(|e| e.to_string())?;
            let restored = restore_backup(&pass, &sealed).map_err(|e| e.to_string())?;
            let sqlite = restored
                .sqlite_bytes
                .ok_or("backup missing ledger.sqlite")?;
            write_restored_db(&db, &sqlite).map_err(|e| e.to_string())?;
            println!(
                "restored\t{} txs\t-> {}",
                restored.manifest.transaction_count,
                db.display()
            );
            Ok(())
        }
        other => Err(format!("unknown backup subcommand {other}")),
    }
}

fn cmd_seal(args: &[String]) -> Result<(), String> {
    let mut db = None;
    let mut out = None;
    let mut pass = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                db = Some(PathBuf::from(args.get(i).ok_or("needs value")?));
            }
            "--out" | "-o" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("needs value")?));
            }
            "--passphrase" | "-p" => {
                i += 1;
                pass = Some(args.get(i).ok_or("needs value")?.clone());
            }
            _ => {}
        }
        i += 1;
    }
    let db = db.unwrap_or_else(default_db_path);
    let out = out.unwrap_or_else(|| db.with_extension("rrsealed"));
    let pass = pass.ok_or("--passphrase required")?;
    rradar_core::seal_db_file(&db, &out, &pass).map_err(|e| e.to_string())?;
    println!("sealed\t{}", out.display());
    Ok(())
}

fn cmd_unseal(args: &[String]) -> Result<(), String> {
    let mut sealed = None;
    let mut out = None;
    let mut pass = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--in" | "-i" => {
                i += 1;
                sealed = Some(PathBuf::from(args.get(i).ok_or("needs value")?));
            }
            "--out" | "-o" | "--db" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("needs value")?));
            }
            "--passphrase" | "-p" => {
                i += 1;
                pass = Some(args.get(i).ok_or("needs value")?.clone());
            }
            s if !s.starts_with('-') && sealed.is_none() => {
                sealed = Some(PathBuf::from(s));
            }
            _ => {}
        }
        i += 1;
    }
    let sealed = sealed.ok_or("usage: rradar unseal --in x.rrsealed -p PASS -o ledger.db")?;
    let pass = pass.ok_or("--passphrase required")?;
    let out = out.unwrap_or_else(default_db_path);
    let bytes = std::fs::read(&sealed).map_err(|e| e.to_string())?;
    let plain = rradar_core::crypto::unseal_bytes(&pass, &bytes, rradar_core::crypto::ARGON2_M_KIB)
        .map_err(|e| e.to_string())?;
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out, plain).map_err(|e| e.to_string())?;
    println!("unsealed\t{}", out.display());
    Ok(())
}

// --- display ---------------------------------------------------------------

fn print_draft(draft: &ReceiptDraft, explain: bool, json: bool) {
    if json {
        match serde_json::to_string_pretty(draft) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("json error: {e}"),
        }
        return;
    }
    println!("id\t{}", draft.id);
    println!("source\t{}", draft.source_path.as_str());
    println!(
        "merchant\t{}\tconf={:.2}",
        draft.merchant.value, draft.merchant.confidence
    );
    println!(
        "total\t{} {}\tminor={}",
        draft.total.value.currency,
        draft.total.value.display_major(),
        draft.total.value.amount_minor
    );
    println!("date\t{}", draft.transacted_at.value);
    if let Some(ref inv) = draft.invoice_id {
        println!("invoice\t{}", inv.value);
    }
    println!("category\t{}", draft.category.value);
    println!("confidence\t{:.2}", draft.overall_confidence);
    if explain {
        println!("--- explain ---");
        print!("{}", draft.explain.format_pretty());
    }
}

fn print_table(rows: &[Transaction]) {
    if rows.is_empty() {
        println!("(empty)");
        return;
    }
    println!("date\t\tcurrency\tamount\t\tcategory\tmerchant\tid");
    for t in rows {
        let m = Money::new(
            t.amount_minor,
            Iso4217::parse(&t.currency).unwrap_or(Iso4217::TWD),
        );
        let merch: String = t.merchant.chars().take(20).collect();
        let date = t.transacted_at.get(..10).unwrap_or(&t.transacted_at);
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            date,
            t.currency,
            m.display_major(),
            t.category,
            merch,
            t.id
        );
    }
}

fn print_help() {
    println!(
        "\
rradar — ReceiptRadar CLI (local-first ledger)
{PRODUCT_ID} {VERSION}

Quick start:
  rradar init
  rradar process fixtures/text/familymart_89.txt --confirm --explain
  rradar list
  rradar stats
  rradar export csv -o out.csv

Commands:
  init                 Create data dir + empty ledger
  doctor               Environment / db check
  path                 Print default home & db paths
  process <file>       Parse receipt (alias: add)
  list                 List transactions (alias: ls)
  show <id>            Show one transaction (JSON)
  edit <id>            Edit merchant/amount/category/notes/date
  delete <id> --yes    Delete transaction (alias: rm)
  stats                Per-currency totals (default: this month)
  categories           List category ids
  export csv|json      Export ledger
  backup create|restore
  seal / unseal        Whole-file encryption (.rrsealed)

process options:
  --confirm, -c        Write to ledger (default db if --db omitted)
  --explain            Show rules / amount candidates
  --json               JSON output
  --engine mock|onnx   OCR backend (default mock)
  --currency TWD|USD|… Default currency fallback
  --qr STR / --qr-file Path to TW e-invoice left QR
  --merchant --amount --category --date --notes
  --force              Override hard dedupe
  --db PATH -p PASS    Ledger path / sealed passphrase

Global data:
  Default db:  %APPDATA%\\receiptradar\\ledger.db  (or $XDG_DATA_HOME/receiptradar)
  Override:    RRADAR_HOME, RRADAR_DB
  Fast backup: RRADAR_FAST_BACKUP=1  (weaker Argon2 for tests)

No cloud. No account. Core path works offline.
"
    );
}

#[allow(dead_code)]
fn _keep(p: &Path) {
    let _ = p;
}
