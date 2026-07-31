//! `rradar` — complete local-first receipt ledger CLI.

mod serve;

use rradar_core::{
    apply_edits, apply_handoff_merge, category_engine_with_packs, create_backup, create_handoff,
    data_dir, default_db_path, ensure_data_dir, ensure_inbox_dir, ensure_rules_dir, inbox_dir,
    inspect_backup, inspect_handoff, install_rule_pack, list_rule_files, monthly_markdown,
    open_ledger_auto, process_path, restore_backup, rules_dir, save_sealed,
    transactions_from_backup, transactions_to_csv, transactions_to_json, verify_backup,
    write_handoff_file, write_restored_db, AppConfig, Iso4217, Money, ProcessOptions, ReceiptDraft,
    Transaction, TxUpdate, UserEdits, LEDGER_SCHEMA_VERSION, PRODUCT_ID, VERSION,
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
            if args.len() > 1 {
                print_topic_help(&args[1])
            } else {
                print_help();
                Ok(())
            }
        }
        "version" | "--version" | "-V" => cmd_version(&args[1..]),
        "init" => cmd_init(&args[1..]),
        "config" => cmd_config(&args[1..]),
        "doctor" => cmd_doctor(&args[1..]),
        "demo" => cmd_demo(&args[1..]),
        "process" | "add" => cmd_process(&args[1..]),
        "manual" | "entry" => cmd_manual(&args[1..]),
        "import" => cmd_import(&args[1..]),
        "list" | "ls" | "search" => cmd_list(&args[1..]),
        "count" => cmd_count(&args[1..]),
        "last" | "undo" => cmd_last_or_undo(&args[0], &args[1..]),
        "show" => cmd_show(&args[1..]),
        "delete" | "rm" => cmd_delete(&args[1..]),
        "edit" => cmd_edit(&args[1..]),
        "stats" => cmd_stats(&args[1..]),
        "top" => cmd_top(&args[1..]),
        "report" => cmd_report(&args[1..]),
        "watch" => cmd_watch(&args[1..]),
        "inbox" => cmd_inbox(&args[1..]),
        "serve" => cmd_serve(&args[1..]),
        "recategorize" => cmd_recategorize(&args[1..]),
        "clear" => cmd_clear(&args[1..]),
        "categories" | "cats" => cmd_categories(),
        "rules" => cmd_rules(&args[1..]),
        "handoff" => cmd_handoff(&args[1..]),
        "export" => cmd_export(&args[1..]),
        "backup" => cmd_backup(&args[1..]),
        "migrate" => cmd_migrate(&args[1..]),
        "models" => cmd_models(&args[1..]),
        "seal" => cmd_seal(&args[1..]),
        "unseal" => cmd_unseal(&args[1..]),
        "path" => {
            println!("home  | {}", data_dir().display());
            println!("db    | {}", default_db_path().display());
            println!("inbox | {}", inbox_dir().display());
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

// --- version / release probe -----------------------------------------------

fn cmd_version(args: &[String]) -> Result<(), String> {
    let long = args
        .iter()
        .any(|a| a == "--long" || a == "-l" || a == "--verbose");
    let json = args.iter().any(|a| a == "--json");
    if json {
        let onnx = rradar_ocr::onnx_feature_enabled();
        println!(
            "{{\n  \"product_id\": \"{PRODUCT_ID}\",\n  \"version\": \"{VERSION}\",\n  \"ledger_schema\": {LEDGER_SCHEMA_VERSION},\n  \"onnx_feature\": {onnx},\n  \"os\": \"{}\",\n  \"arch\": \"{}\"\n}}",
            env::consts::OS,
            env::consts::ARCH
        );
        return Ok(());
    }
    println!("{PRODUCT_ID} {VERSION}");
    if long {
        println!("ledger_schema | {LEDGER_SCHEMA_VERSION}");
        println!(
            "onnx_feature  | {}",
            if rradar_ocr::onnx_feature_enabled() {
                "enabled"
            } else {
                "disabled (build with --features onnx)"
            }
        );
        println!("target        | {}-{}", env::consts::OS, env::consts::ARCH);
        println!("policy        | local-first; no official cloud relay");
    }
    Ok(())
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
    let cfg = AppConfig::default();
    if !AppConfig::path().is_file() {
        cfg.save().map_err(|e| e.to_string())?;
    }
    println!("initialized");
    println!("  home:   {}", dir.display());
    println!("  db:     {}", db.display());
    println!("  config: {}", AppConfig::path().display());
    println!("next: rradar process <receipt.txt|image> --confirm");
    Ok(())
}

fn cmd_config(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args[0] == "show" {
        let c = AppConfig::load();
        println!("path | {}", AppConfig::path().display());
        println!("default_currency | {}", c.default_currency);
        println!("list_limit | {}", c.list_limit);
        return Ok(());
    }
    if args[0] == "set" {
        let mut c = AppConfig::load();
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "default_currency" | "--currency" => {
                    i += 1;
                    let v = args.get(i).ok_or("needs currency code")?.clone();
                    Iso4217::parse(&v).ok_or_else(|| format!("bad currency {v}"))?;
                    c.default_currency = v.to_uppercase();
                }
                "list_limit" | "--list-limit" => {
                    i += 1;
                    c.list_limit = args
                        .get(i)
                        .ok_or("needs number")?
                        .parse()
                        .map_err(|_| "bad list_limit")?;
                }
                other => return Err(format!("unknown config key `{other}`")),
            }
            i += 1;
        }
        let _ = ensure_data_dir();
        c.save().map_err(|e| e.to_string())?;
        println!("saved | {}", AppConfig::path().display());
        return Ok(());
    }
    Err("usage: rradar config [show|set default_currency TWD|set list_limit 50]".into())
}

fn cmd_doctor(_args: &[String]) -> Result<(), String> {
    let cfg = AppConfig::load();
    println!("receiptradar doctor");
    println!("  version:  {VERSION}");
    println!("  home:     {}", data_dir().display());
    println!("  db:       {}", default_db_path().display());
    println!("  inbox:    {}", inbox_dir().display());
    println!("  config:   {}", AppConfig::path().display());
    println!("  currency: {}", cfg.default_currency);
    let db = default_db_path();
    println!("  schema:   supports ledger v{LEDGER_SCHEMA_VERSION} (local-first; no cloud relay)");
    if db.is_file() {
        match rradar_core::Ledger::open(&db) {
            Ok(l) => {
                let ver = l.schema_version().unwrap_or_else(|_| "?".into());
                println!(
                    "  ledger:   ok (schema {ver}, {} transactions)",
                    l.count().unwrap_or(-1)
                );
            }
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
    let onnx_cfg = rradar_ocr::OnnxConfig::from_models_dir(&models);
    for line in onnx_cfg.status_lines() {
        println!("{line}");
    }
    println!(
        "  engines:  mock (default), onnx{}",
        if rradar_ocr::onnx_feature_enabled() {
            " [feature ON]"
        } else {
            " [rebuild with --features onnx for inference]"
        }
    );
    println!("  privacy:  local-first; no network required for core path");
    println!("  demo:     rradar demo   # isolated closed-loop from fixtures/");
    Ok(())
}

/// Recordable closed-loop demo: fixtures → parse → confirm → list/stats → export → backup.
/// Uses an isolated demo ledger (does not touch the default user ledger unless --db set).
fn cmd_demo(args: &[String]) -> Result<(), String> {
    let mut fixtures_root: Option<PathBuf> = None;
    let mut db_override: Option<PathBuf> = None;
    let mut skip_backup = false;
    let mut quiet = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fixtures" => {
                i += 1;
                fixtures_root = Some(PathBuf::from(args.get(i).ok_or("--fixtures needs path")?));
            }
            "--db" => {
                i += 1;
                db_override = Some(PathBuf::from(args.get(i).ok_or("--db needs path")?));
            }
            "--no-backup" => skip_backup = true,
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => {
                print_topic_help("demo")?;
                return Ok(());
            }
            other => {
                return Err(format!(
                    "unknown demo flag `{other}` — try `rradar help demo`"
                ))
            }
        }
        i += 1;
    }

    let fixtures = fixtures_root
        .or_else(|| env::var_os("RRADAR_FIXTURES").map(PathBuf::from))
        .unwrap_or_else(find_fixtures_dir);
    if !fixtures.is_dir() {
        return Err(format!(
            "fixtures not found at {} — run from repo root or pass --fixtures PATH",
            fixtures.display()
        ));
    }

    let demo_db = if let Some(p) = db_override {
        p
    } else if env::var_os("RRADAR_DB").is_some() {
        default_db_path()
    } else {
        let home = data_dir().join("demo");
        let _ = std::fs::create_dir_all(&home);
        home.join("ledger.db")
    };
    if let Some(parent) = demo_db.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Fresh demo ledger each run (isolated path by default).
    if demo_db.is_file() {
        let _ = std::fs::remove_file(&demo_db);
    }
    let ledger = rradar_core::Ledger::open(&demo_db).map_err(|e| e.to_string())?;
    let eng = engine_by_name("mock").map_err(|e| e.to_string())?;
    let categories = category_engine_with_packs();

    if !quiet {
        println!("══════════════════════════════════════════════");
        println!(" ReceiptRadar demo — local-first closed loop");
        println!(" No cloud. No account. Fixtures only.");
        println!("══════════════════════════════════════════════");
        println!("fixtures | {}", fixtures.display());
        println!("demo db  | {}", demo_db.display());
        println!("schema   | {}", ledger.schema_version().unwrap_or_default());
        println!();
    }

    // Collect demo inputs: all text fixtures + mock_ocr + one QR sample.
    let mut text_paths = collect_glob(&fixtures.join("text"), &["txt"]);
    text_paths.sort();
    let mut mock_paths = collect_glob(&fixtures.join("mock_ocr"), &["bin"]);
    mock_paths.sort();
    let qr_sample = fixtures.join("qr/tw_einvoice_sample_01.payload.txt");

    if text_paths.is_empty() {
        return Err(format!("no text fixtures under {}", fixtures.display()));
    }

    let step = |n: u32, title: &str| {
        if !quiet {
            println!("── step {n}: {title} ──");
        }
    };

    step(1, "parse + confirm text receipts");
    let mut confirmed = 0usize;
    for path in &text_paths {
        let currency = currency_hint_for_path(path);
        let draft = process_path(
            path,
            eng.as_ref(),
            &categories,
            ProcessOptions {
                default_currency: currency,
                ..Default::default()
            },
        )
        .map_err(|e| format!("{}: {e}", path.display()))?;
        let hash = rradar_core::preprocess::content_hash(&std::fs::read(path).unwrap_or_default());
        let res = ledger
            .confirm_draft(&draft, Some(&hash), None, false)
            .map_err(|e| e.to_string())?;
        if res.inserted {
            confirmed += 1;
        }
        if !quiet {
            println!(
                "  ✓ {:<28}  {:>3} {:>10.2}  {}",
                path.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                draft.total.value.currency,
                draft.total.value.amount_minor as f64
                    / 10f64.powi(draft.total.value.exponent as i32),
                draft.merchant.value
            );
        }
    }

    step(2, "mock image path (RRADAR_MOCK_OCR binaries)");
    for path in &mock_paths {
        let currency = currency_hint_for_path(path);
        let draft = process_path(
            path,
            eng.as_ref(),
            &categories,
            ProcessOptions {
                default_currency: currency,
                ..Default::default()
            },
        )
        .map_err(|e| format!("{}: {e}", path.display()))?;
        let hash = rradar_core::preprocess::content_hash(&std::fs::read(path).unwrap_or_default());
        let res = ledger
            .confirm_draft(&draft, Some(&hash), Some("demo mock_ocr"), false)
            .map_err(|e| e.to_string())?;
        if res.inserted {
            confirmed += 1;
        }
        if !quiet {
            println!(
                "  ✓ {:<28}  {} (source mock image)",
                path.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                draft.merchant.value
            );
        }
    }

    step(3, "pixel path image + .ocr.txt sidecar (CI-safe)");
    let img_sidecar = fixtures.join("images/familymart_photo.png");
    if img_sidecar.is_file() {
        let draft = process_path(
            &img_sidecar,
            eng.as_ref(),
            &categories,
            ProcessOptions {
                default_currency: Iso4217::TWD,
                ..Default::default()
            },
        )
        .map_err(|e| format!("{}: {e}", img_sidecar.display()))?;
        let hash =
            rradar_core::preprocess::content_hash(&std::fs::read(&img_sidecar).unwrap_or_default());
        let res = ledger
            .confirm_draft(&draft, Some(&hash), Some("demo image sidecar"), false)
            .map_err(|e| e.to_string())?;
        if res.inserted {
            confirmed += 1;
        }
        if !quiet {
            println!(
                "  ✓ {:<28}  {} (sidecar OCR text)",
                img_sidecar
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?"),
                draft.merchant.value
            );
        }
    } else if !quiet {
        println!("  (skip — fixtures/images not present)");
    }

    step(4, "TW e-invoice QR prefer path");
    if qr_sample.is_file() {
        let payload = std::fs::read_to_string(&qr_sample)
            .map_err(|e| e.to_string())?
            .trim()
            .to_string();
        // Any path works; QR payload drives structured fields.
        let carrier = &text_paths[0];
        let draft = process_path(
            carrier,
            eng.as_ref(),
            &categories,
            ProcessOptions {
                qr_payload: Some(payload),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
        let res = ledger
            .confirm_draft(&draft, None, Some("demo qr"), true)
            .map_err(|e| e.to_string())?;
        if res.inserted {
            confirmed += 1;
        }
        if !quiet {
            println!(
                "  ✓ QR invoice={} total={} {}",
                draft
                    .invoice_id
                    .as_ref()
                    .map(|f| f.value.as_str())
                    .unwrap_or("?"),
                draft.total.value.amount_minor,
                draft.total.value.currency
            );
        }
    } else if !quiet {
        println!("  (skip — qr sample missing)");
    }

    step(5, "browse ledger");
    let n = ledger.count().map_err(|e| e.to_string())?;
    let rows = ledger.list_transactions(8, 0).map_err(|e| e.to_string())?;
    if !quiet {
        println!("  count | {n}  (confirmed this run ≈ {confirmed})");
        for t in &rows {
            println!(
                "  · {}  {}  {}  {}",
                &t.id[..t.id.len().min(12)],
                t.currency,
                t.amount_minor,
                t.merchant
            );
        }
    }

    step(6, "stats + top merchants");
    let stats = ledger.stats_by_currency_all().map_err(|e| e.to_string())?;
    if !quiet {
        for s in &stats {
            println!(
                "  {} {:04}-{:02}  n={}  minor={}",
                s.currency, s.year, s.month, s.count, s.total_minor
            );
        }
        if let Ok(top) = ledger.top_merchants("TWD", 5) {
            for (m, minor, c) in top {
                println!("  top TWD | {m}  minor={minor}  n={c}");
            }
        }
    }

    step(7, "export CSV + JSON");
    let out_dir = demo_db
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let all = ledger
        .list_transactions(10_000, 0)
        .map_err(|e| e.to_string())?;
    let csv_path = out_dir.join("demo-export.csv");
    let json_path = out_dir.join("demo-export.json");
    let csv = transactions_to_csv(&all).map_err(|e| e.to_string())?;
    let json = transactions_to_json(&all).map_err(|e| e.to_string())?;
    std::fs::write(&csv_path, csv).map_err(|e| e.to_string())?;
    std::fs::write(&json_path, json).map_err(|e| e.to_string())?;
    if !quiet {
        println!("  csv  | {}", csv_path.display());
        println!("  json | {}", json_path.display());
    }

    if !skip_backup {
        step(8, "encrypted backup.rradar");
        // Demo passphrase is intentionally public; real users choose their own.
        let bak = out_dir.join("demo-backup.rradar");
        let bytes = create_backup(
            &ledger,
            "demo-passphrase",
            8, /* fast Argon2 for demo */
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(&bak, bytes).map_err(|e| e.to_string())?;
        if !quiet {
            println!(
                "  backup | {}  (passphrase: demo-passphrase)",
                bak.display()
            );
        }
    }

    step(9, "monthly markdown report");
    // Pick a month that appears in fixtures (2024-05 family mart).
    let md = monthly_markdown(&ledger, 2024, 5).map_err(|e| e.to_string())?;
    let report_path = out_dir.join("demo-report-2024-05.md");
    std::fs::write(&report_path, &md).map_err(|e| e.to_string())?;
    if !quiet {
        println!("  report | {}", report_path.display());
        for line in md.lines().take(6) {
            println!("  | {line}");
        }
    }

    step(10, "ONNX model pin status (weights optional)");
    let models_dir = rradar_ocr::default_models_dir();
    match rradar_ocr::verify_models_dir(&models_dir, false) {
        Ok(checks) if checks.is_empty() => {
            if !quiet {
                println!("  models | no pins loaded from {}", models_dir.display());
            }
        }
        Ok(checks) => {
            let ok = checks.iter().filter(|c| c.is_ok()).count();
            if !quiet {
                println!(
                    "  models | {ok}/{} pins ok under {} (rradar models verify)",
                    checks.len(),
                    models_dir.display()
                );
            }
        }
        Err(e) => {
            if !quiet {
                println!("  models | {e}");
            }
        }
    }

    if !quiet {
        println!();
        println!("DEMO_OK — closed loop finished ({n} rows in demo ledger).");
        println!("Next: rradar list --db {}", demo_db.display());
        println!(
            "      rradar report --year 2024 --month 5 --db {}",
            demo_db.display()
        );
        println!("      rradar inbox --ensure && rradar watch --once");
        println!(
            "      rradar serve --db {}   # loopback HTTP only",
            demo_db.display()
        );
        println!("      powershell -File scripts/smoke-onnx.ps1  # optional real ONNX e2e");
        println!("Record tip: capture this command output as a terminal GIF for README.");
    } else {
        println!("DEMO_OK n={n}");
    }
    Ok(())
}

fn find_fixtures_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("fixtures"),
        PathBuf::from("./fixtures"),
        // When cwd is crates/rradar-cli during some cargo invocations
        PathBuf::from("../../fixtures"),
        PathBuf::from("../fixtures"),
    ];
    for c in candidates {
        if c.is_dir() {
            return c;
        }
    }
    PathBuf::from("fixtures")
}

fn collect_glob(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let ok = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
            .unwrap_or(false);
        if ok {
            out.push(p);
        }
    }
    out
}

fn currency_hint_for_path(path: &Path) -> Iso4217 {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.contains("usd") || name.contains("starbucks") {
        Iso4217::USD
    } else {
        Iso4217::TWD
    }
}

fn cmd_process(args: &[String]) -> Result<(), String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut explain = false;
    let mut engine = "mock".to_string();
    let mut qr: Option<String> = None;
    let mut json = false;
    let mut currency = default_currency_from_env();
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
            s => paths.push(PathBuf::from(s)),
        }
        i += 1;
    }

    if paths.is_empty() {
        return Err(
            "usage: rradar process <path> [more paths…] [--confirm] [--explain] [--amount 89]"
                .into(),
        );
    }

    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let categories = category_engine_with_packs();
    let opts_base = ProcessOptions {
        default_currency: currency,
        qr_payload: qr,
        ..Default::default()
    };

    let db_path = db.unwrap_or_else(default_db_path);
    let flags = DbFlags {
        db: db_path,
        passphrase,
    };
    let mut ledger_open = if confirm {
        let _ = ensure_data_dir();
        if let Some(parent) = flags.db.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Some(open_db(&flags)?)
    } else {
        None
    };

    let mut confirmed_n = 0usize;
    for path in &paths {
        let mut draft = process_path(path, eng.as_ref(), &categories, opts_base.clone())
            .map_err(|e| format!("{}: {e}", path.display()))?;

        let mut edits = UserEdits {
            merchant: merchant.clone(),
            notes: notes.clone(),
            category: category.clone(),
            transacted_at: date.clone(),
            ..Default::default()
        };
        if let Some(ref a) = amount_major {
            let m = Money::from_major_str(a, currency).map_err(|e| e.to_string())?;
            edits.amount_minor = Some(m.amount_minor);
            edits.currency = Some(currency.to_string());
        }
        apply_edits(&mut draft, &edits);
        if edits.merchant.is_some() && edits.category.is_none() {
            let mut ex = draft.explain.clone();
            draft.category = categories.categorize(&draft.merchant.value, &draft.raw_text, &mut ex);
            draft.explain = ex;
        }

        if !quiet {
            if paths.len() > 1 {
                println!("=== {} ===", path.display());
            }
            print_draft(&draft, explain, json && !confirm);
        } else if json && !confirm {
            print_draft(&draft, false, true);
        }

        if let Some((ref ledger, _)) = ledger_open {
            let hash =
                rradar_core::preprocess::content_hash(&std::fs::read(path).unwrap_or_default());
            let result = ledger
                .confirm_draft(&draft, Some(&hash), notes.as_deref(), force)
                .map_err(|e| e.to_string())?;
            if let Some(ref d) = result.dedupe {
                eprintln!(
                    "dedupe {:?} | {} | existing={}",
                    d.level, d.message, d.existing_id
                );
            }
            if result.inserted {
                confirmed_n += 1;
                println!("confirmed | {}", result.transaction.id);
            } else {
                println!(
                    "skipped | {} | hard dedupe (use --force)",
                    result.transaction.id
                );
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
        }
    }

    if let Some((ledger, tmp)) = ledger_open.take() {
        maybe_reseal(&flags, &ledger, tmp)?;
        if paths.len() > 1 {
            println!("batch | confirmed={confirmed_n} files={}", paths.len());
        }
    }
    Ok(())
}

fn cmd_manual(args: &[String]) -> Result<(), String> {
    let mut merchant = None;
    let mut amount = None;
    let mut currency = default_currency_from_env();
    let mut category = "other".to_string();
    let mut date = None;
    let mut notes = None;
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
                let c = args.get(i).ok_or("needs value")?;
                currency = Iso4217::parse(c).ok_or_else(|| format!("bad currency {c}"))?;
            }
            "--category" => {
                i += 1;
                category = args.get(i).ok_or("needs value")?.clone();
            }
            "--date" => {
                i += 1;
                date = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--notes" => {
                i += 1;
                notes = Some(args.get(i).ok_or("needs value")?.clone());
            }
            s if s.starts_with('-') => return Err(format!("unknown {s}")),
            _ => {}
        }
        i += 1;
    }
    let merchant =
        merchant.ok_or("usage: rradar manual --merchant NAME --amount 89 [--date YYYY-MM-DD]")?;
    let amount = amount.ok_or("--amount required")?;
    let money = Money::from_major_str(&amount, currency).map_err(|e| e.to_string())?;
    let day = date.unwrap_or_else(|| {
        let iso = rradar_core::utc_now_iso();
        iso.get(..10).unwrap_or("1970-01-01").to_string()
    });
    let cats = category_engine_with_packs();
    let mut ex = rradar_core::ExplainTrace::new("manual", "manual");
    let cat_field = if category == "other" || category.is_empty() {
        cats.categorize(&merchant, "", &mut ex)
    } else {
        rradar_core::Field::new(category, 1.0, rradar_core::FieldSource::User)
    };
    let draft = ReceiptDraft {
        id: ReceiptDraft::new_id(),
        captured_at: rradar_core::utc_now_iso(),
        merchant: rradar_core::Field::new(merchant, 1.0, rradar_core::FieldSource::User),
        total: rradar_core::Field::new(money, 1.0, rradar_core::FieldSource::User),
        transacted_at: rradar_core::Field::new(day, 1.0, rradar_core::FieldSource::User),
        tax: None,
        invoice_id: None,
        category: cat_field,
        raw_text: String::new(),
        ocr_blocks: vec![],
        overall_confidence: 1.0,
        explain: ex,
        source_path: rradar_core::SourcePath::Manual,
    };
    let flags = extract_db_from_all(args)?;
    let _ = ensure_data_dir();
    let (ledger, tmp) = open_db(&flags)?;
    let result = ledger
        .confirm_draft(&draft, None, notes.as_deref(), true)
        .map_err(|e| e.to_string())?;
    println!("confirmed | {}", result.transaction.id);
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

fn cmd_import(args: &[String]) -> Result<(), String> {
    // rradar import json path.json
    // rradar import backup --in file.rradar -p PASS [--db PATH]
    if args.is_empty() {
        return Err(
            "usage: rradar import json <file.json> | rradar import backup --in file.rradar -p PASS"
                .into(),
        );
    }
    match args[0].as_str() {
        "json" => {
            let path = args.get(1).ok_or("usage: rradar import json <file.json>")?;
            let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let rows: Vec<Transaction> = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let flags = extract_db_from_all(args)?;
            let _ = ensure_data_dir();
            let (ledger, tmp) = open_db(&flags)?;
            let (ins, skip) = ledger
                .import_transactions(&rows)
                .map_err(|e| e.to_string())?;
            println!("import | inserted={ins} skipped={skip}");
            maybe_reseal(&flags, &ledger, tmp)?;
            Ok(())
        }
        "backup" | "rradar" => {
            // Merge transactions from encrypted backup into current ledger (skip existing ids).
            let mut input = None;
            let mut pass = None;
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
                    _ => {}
                }
                i += 1;
            }
            let input = input.ok_or("--in required")?;
            let pass = pass.ok_or("--passphrase required")?;
            let sealed = std::fs::read(&input).map_err(|e| e.to_string())?;
            let restored = restore_backup(&pass, &sealed).map_err(|e| e.to_string())?;
            let rows = transactions_from_backup(&restored).map_err(|e| e.to_string())?;
            let flags = extract_db_from_all(args)?;
            let _ = ensure_data_dir();
            let (ledger, tmp) = open_db(&flags)?;
            let (ins, skip) = ledger
                .import_transactions(&rows)
                .map_err(|e| e.to_string())?;
            println!(
                "import backup | inserted={ins} skipped={skip} (from {} txs; multi-device via backup only)",
                rows.len()
            );
            maybe_reseal(&flags, &ledger, tmp)?;
            Ok(())
        }
        other => Err(format!(
            "unknown import type `{other}` — try: import json | import backup"
        )),
    }
}

/// ONNX model pack status / hash verification (`models/manifest.sha256`).
fn cmd_models(args: &[String]) -> Result<(), String> {
    let mut sub = "status";
    let mut dir = rradar_ocr::default_models_dir();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "status" | "verify" | "pins" => sub = args[i].as_str(),
            "--dir" => {
                i += 1;
                dir = PathBuf::from(args.get(i).ok_or("--dir needs path")?);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown models flag `{other}`"));
            }
            other => {
                return Err(format!(
                    "unknown models subcommand `{other}` — try status|verify|pins"
                ));
            }
        }
        i += 1;
    }

    println!("models | dir={}", dir.display());
    let cfg = rradar_ocr::OnnxConfig::from_models_dir(&dir);
    // Path / feature / ORT lines only (pins printed from verify_models_dir).
    for line in cfg.status_lines() {
        if line.contains("model pins:")
            || line.contains("pin ok")
            || line.contains("pin MISS")
            || line.contains("pin BAD")
        {
            continue;
        }
        println!("{line}");
    }

    let require = sub == "verify";
    let checks = rradar_ocr::verify_models_dir(&dir, require).map_err(|e| e.to_string())?;
    if checks.is_empty() {
        println!("models | no pins in manifest.sha256");
        if require {
            return Err("verify requires pin lines in models/manifest.sha256".into());
        }
        return Ok(());
    }
    for c in &checks {
        println!("{}", c.summary_line());
    }
    let ok_n = checks.iter().filter(|c| c.is_ok()).count();
    if sub == "verify" {
        if rradar_ocr::all_pins_ok(&checks) {
            println!("models verify | OK ({ok_n} files)");
            Ok(())
        } else {
            Err("models verify failed — run tools/fetch-models.ps1 or tools/fetch-models.sh".into())
        }
    } else {
        println!(
            "models | {ok_n}/{} pins ok  (rradar models verify)",
            checks.len()
        );
        Ok(())
    }
}

/// Open ledger (runs migrations) and print schema status.
fn cmd_migrate(args: &[String]) -> Result<(), String> {
    let flags = extract_db_from_all(args)?;
    let _ = ensure_data_dir();
    if let Some(parent) = flags.db.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let (ledger, tmp) = open_db(&flags)?;
    let ver = ledger.schema_version().map_err(|e| e.to_string())?;
    let n = ledger.count().map_err(|e| e.to_string())?;
    println!("migrate | db={}", flags.db.display());
    println!("migrate | schema={ver} (binary supports {LEDGER_SCHEMA_VERSION})");
    println!("migrate | transactions={n}");
    if let Some(app) = ledger.meta_get("app_version").ok().flatten() {
        println!("migrate | app_version_meta={app}");
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_list(args: &[String]) -> Result<(), String> {
    let mut json = false;
    let mut limit = AppConfig::load().list_limit;
    let mut offset = 0usize;
    let mut currency: Option<String> = None;
    let mut query: Option<String> = None;
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
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
            "--year" => {
                i += 1;
                year = Some(
                    args.get(i)
                        .ok_or("needs year")?
                        .parse()
                        .map_err(|_| "bad year")?,
                );
            }
            "--month" => {
                i += 1;
                month = Some(
                    args.get(i)
                        .ok_or("needs month")?
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
    let rows = if let (Some(y), Some(m)) = (year, month) {
        ledger
            .list_by_month(y, m, limit)
            .map_err(|e| e.to_string())?
    } else {
        ledger
            .list_filtered(limit, offset, currency.as_deref(), query.as_deref())
            .map_err(|e| e.to_string())?
    };
    if json {
        println!(
            "{}",
            transactions_to_json(&rows).map_err(|e| e.to_string())?
        );
    } else {
        print_table(&rows);
        eprintln!("({} rows | db={})", rows.len(), flags.db.display());
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_last_or_undo(cmd: &str, args: &[String]) -> Result<(), String> {
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let last = ledger
        .last_transaction()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "ledger is empty".to_string())?;
    if cmd == "last" {
        println!(
            "{}",
            serde_json::to_string_pretty(&last).unwrap_or_default()
        );
        if let Some(t) = tmp {
            let _ = std::fs::remove_file(t);
        }
        return Ok(());
    }
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    if !yes {
        println!(
            "would undo | {} | {} | {} {}",
            last.id,
            last.merchant,
            last.currency,
            Money::new(
                last.amount_minor,
                Iso4217::parse(&last.currency).unwrap_or(Iso4217::TWD)
            )
            .display_major()
        );
        return Err("refusing to undo without --yes (re-run: rradar undo --yes)".into());
    }
    ledger
        .delete_transaction(&last.id)
        .map_err(|e| e.to_string())?;
    println!("undone | {}", last.id);
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

fn cmd_recategorize(args: &[String]) -> Result<(), String> {
    let only_other = !args.iter().any(|a| a == "--all");
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let eng = category_engine_with_packs();
    let n = ledger
        .recategorize_all(&eng, only_other)
        .map_err(|e| e.to_string())?;
    println!(
        "recategorized | {} rows (scope={})",
        n,
        if only_other { "other-only" } else { "all" }
    );
    maybe_reseal(&flags, &ledger, tmp)?;
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

fn cmd_count(args: &[String]) -> Result<(), String> {
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let n = ledger.count().map_err(|e| e.to_string())?;
    println!("count | {n}");
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
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
    let mut from: Option<String> = None;
    let mut to: Option<String> = None;
    let mut by_category = false;
    let mut currency = default_currency_from_env().to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--all" => all = true,
            "--by-category" => by_category = true,
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
            "--from" => {
                i += 1;
                from = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--to" => {
                i += 1;
                to = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--currency" => {
                i += 1;
                currency = args.get(i).ok_or("needs value")?.clone();
            }
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;

    if by_category {
        let ym = match (year, month) {
            (Some(y), Some(m)) => Some(format!("{y:04}-{m:02}")),
            _ if all => None,
            _ => {
                let (y, m) = current_year_month();
                Some(format!("{y:04}-{m:02}"))
            }
        };
        println!(
            "categories | currency={currency} | period={}",
            ym.as_deref().unwrap_or("all-time")
        );
        let rows = ledger
            .stats_by_category(&currency, ym.as_deref())
            .map_err(|e| e.to_string())?;
        if rows.is_empty() {
            println!("(no transactions)");
        } else {
            for c in &rows {
                let major = Money::new(
                    c.total_minor,
                    Iso4217::parse(&c.currency).unwrap_or(Iso4217::TWD),
                )
                .display_major();
                println!(
                    "{} | {} | count={} | minor={}",
                    c.category, major, c.count, c.total_minor
                );
            }
        }
        if let Some(t) = tmp {
            let _ = std::fs::remove_file(t);
        }
        return Ok(());
    }

    let stats = if all {
        println!("period | all-time");
        ledger.stats_by_currency_all().map_err(|e| e.to_string())?
    } else if let (Some(f), Some(t)) = (from.as_deref(), to.as_deref()) {
        println!("period | {f} .. {t}");
        ledger
            .stats_by_currency_range(f, t)
            .map_err(|e| e.to_string())?
    } else {
        let (y, m) = match (year, month) {
            (Some(y), Some(m)) => (y, m),
            _ => current_year_month(),
        };
        println!("period | {y:04}-{m:02}");
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
                "{} | {} | count={} | minor={}",
                s.currency, major, s.count, s.total_minor
            );
        }
        println!("note | currencies are never summed together");
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_report(args: &[String]) -> Result<(), String> {
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--year" => {
                i += 1;
                year = Some(
                    args.get(i)
                        .ok_or("needs year")?
                        .parse()
                        .map_err(|_| "bad year")?,
                );
            }
            "--month" => {
                i += 1;
                month = Some(
                    args.get(i)
                        .ok_or("needs month")?
                        .parse()
                        .map_err(|_| "bad month")?,
                );
            }
            "-o" | "--output" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("needs path")?));
            }
            _ => {}
        }
        i += 1;
    }
    let (y, m) = match (year, month) {
        (Some(y), Some(m)) => (y, m),
        _ => current_year_month(),
    };
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let md = monthly_markdown(&ledger, y, m).map_err(|e| e.to_string())?;
    if let Some(p) = out {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&p, md.as_bytes()).map_err(|e| e.to_string())?;
        println!("wrote | {}", p.display());
    } else {
        print!("{md}");
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_inbox(args: &[String]) -> Result<(), String> {
    let open = args.iter().any(|a| a == "--ensure" || a == "ensure");
    let path = if open {
        ensure_inbox_dir().map_err(|e| e.to_string())?
    } else {
        inbox_dir()
    };
    println!("inbox | {}", path.display());
    if open {
        println!("hint | drop receipt .txt/.jpg here then: rradar watch");
    } else if !path.is_dir() {
        println!("hint | run: rradar inbox --ensure");
    }
    Ok(())
}

fn cmd_serve(args: &[String]) -> Result<(), String> {
    let mut bind = serve::default_bind();
    let mut i = 0;
    while i < args.len() {
        if args[i].as_str() == "--bind" {
            i += 1;
            bind = args.get(i).ok_or("needs bind host:port")?.clone();
        }
        i += 1;
    }
    // Defense in depth: serve::run also rejects non-loopback.
    if !serve::is_loopback_bind(&bind) {
        return Err("refuse non-loopback bind (local-only API; use 127.0.0.1:PORT)".into());
    }
    let flags = extract_db_from_all(args)?;
    let _ = ensure_data_dir();
    serve::run(serve::ServeOpts {
        bind,
        db: flags.db,
        passphrase: flags.passphrase,
    })
}

fn cmd_watch(args: &[String]) -> Result<(), String> {
    // rradar watch [dir] [--interval 2] [--once]  — default dir = inbox
    let mut dir: Option<PathBuf> = None;
    let mut interval_secs: u64 = 2;
    let mut confirm = true;
    let mut once = false;
    let mut engine = "mock".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--interval" => {
                i += 1;
                interval_secs = args
                    .get(i)
                    .ok_or("needs secs")?
                    .parse()
                    .map_err(|_| "bad interval")?;
            }
            "--no-confirm" => confirm = false,
            "--once" => once = true,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("needs engine")?.clone();
            }
            s if !s.starts_with('-') => dir = Some(PathBuf::from(s)),
            other => return Err(format!("unknown flag {other}")),
        }
        i += 1;
    }
    let dir = if let Some(d) = dir {
        d
    } else {
        ensure_inbox_dir().map_err(|e| e.to_string())?
    };
    if !dir.is_dir() {
        return Err(format!("not a directory: {}", dir.display()));
    }
    let flags = extract_db_from_all(args)?;
    let _ = ensure_data_dir();
    let mut seen = std::collections::HashSet::<String>::new();
    // seed seen with existing files so first pass only picks new ones unless --once on empty seen means process all
    if !once {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    seen.insert(p.display().to_string());
                }
            }
        }
        println!(
            "watch | {} | interval={interval_secs}s | seeded {} existing files",
            dir.display(),
            seen.len()
        );
    }
    loop {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let mut files: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect();
            files.sort();
            for path in files {
                let key = path.display().to_string();
                if seen.contains(&key) && !once {
                    continue;
                }
                if once && seen.contains(&key) {
                    continue;
                }
                seen.insert(key.clone());
                let mut proc_args = vec![path.display().to_string()];
                if confirm {
                    proc_args.push("--confirm".into());
                    proc_args.push("-q".into());
                }
                proc_args.push("--engine".into());
                proc_args.push(engine.clone());
                if let Some(ref db) = flags.db.to_str() {
                    // always pass db for confirm path
                    if confirm {
                        proc_args.push("--db".into());
                        proc_args.push(db.to_string());
                    }
                }
                println!("watch | processing {}", path.display());
                // call process logic by reconstructing argv
                if let Err(e) = cmd_process(&proc_args) {
                    eprintln!("watch | error: {e}");
                }
            }
        }
        if once {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(interval_secs));
    }
    Ok(())
}

fn cmd_top(args: &[String]) -> Result<(), String> {
    let mut currency = default_currency_from_env().to_string();
    let mut limit = 10usize;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--currency" => {
                i += 1;
                currency = args.get(i).ok_or("needs value")?.clone();
            }
            "--limit" => {
                i += 1;
                limit = args
                    .get(i)
                    .ok_or("needs value")?
                    .parse()
                    .map_err(|_| "bad limit")?;
            }
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let rows = ledger
        .top_merchants(&currency, limit)
        .map_err(|e| e.to_string())?;
    if rows.is_empty() {
        println!("(no rows for {currency})");
    } else {
        println!("rank | merchant | total | count | currency={currency}");
        for (i, (m, minor, cnt)) in rows.iter().enumerate() {
            let major = Money::new(*minor, Iso4217::parse(&currency).unwrap_or(Iso4217::TWD))
                .display_major();
            println!("{:>4} | {m} | {major} | {cnt}", i + 1);
        }
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_clear(args: &[String]) -> Result<(), String> {
    if !args.iter().any(|a| a == "--yes") {
        return Err("refusing to clear ledger without --yes".into());
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let n = ledger.clear_all().map_err(|e| e.to_string())?;
    println!("cleared | {n} transactions");
    maybe_reseal(&flags, &ledger, tmp)?;
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

fn cmd_rules(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args[0] == "list" {
        let _ = ensure_rules_dir();
        println!("rules_dir | {}", rules_dir().display());
        for p in list_rule_files() {
            println!("file | {}", p.display());
        }
        let eng = category_engine_with_packs();
        println!("merchants_loaded | {}", eng.merchant_count());
        return Ok(());
    }
    if args[0] == "install" {
        let src = args
            .get(1)
            .ok_or("usage: rradar rules install <file.yml>")?;
        let dest = install_rule_pack(Path::new(src), None).map_err(|e| e.to_string())?;
        println!("installed | {}", dest.display());
        return Ok(());
    }
    if args[0] == "ensure" {
        let d = ensure_rules_dir().map_err(|e| e.to_string())?;
        println!("rules_dir | {}", d.display());
        return Ok(());
    }
    Err("usage: rradar rules [list|ensure|install <file>]".into())
}

fn cmd_handoff(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err(
            "usage: rradar handoff create -p PASS -o file.rrhandoff [--device LABEL]\n       rradar handoff info -p PASS --in file\n       rradar handoff apply -p PASS --in file [--merge]"
                .into(),
        );
    }
    match args[0].as_str() {
        "create" => {
            let mut pass = None;
            let mut out = None;
            let mut device = "device".to_string();
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--passphrase" | "-p" => {
                        i += 1;
                        pass = Some(args.get(i).ok_or("needs pass")?.clone());
                    }
                    "-o" | "--output" => {
                        i += 1;
                        out = Some(PathBuf::from(args.get(i).ok_or("needs path")?));
                    }
                    "--device" => {
                        i += 1;
                        device = args.get(i).ok_or("needs label")?.clone();
                    }
                    _ => {}
                }
                i += 1;
            }
            let pass = pass.ok_or("--passphrase required")?;
            let out = out.unwrap_or_else(|| {
                data_dir().join(format!(
                    "handoff-{}.rrhandoff",
                    rradar_core::utc_now_iso()
                        .replace(':', "")
                        .replace('T', "-")
                ))
            });
            let flags = extract_db_from_all(args)?;
            let (ledger, tmp) = open_db(&flags)?;
            let bytes = create_handoff(&ledger, &pass, &device).map_err(|e| e.to_string())?;
            write_handoff_file(&out, &bytes).map_err(|e| e.to_string())?;
            println!("handoff | {}", out.display());
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            Ok(())
        }
        "info" => {
            let mut pass = None;
            let mut input = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--passphrase" | "-p" => {
                        i += 1;
                        pass = Some(args.get(i).ok_or("needs pass")?.clone());
                    }
                    "--in" | "-i" => {
                        i += 1;
                        input = Some(PathBuf::from(args.get(i).ok_or("needs path")?));
                    }
                    _ => {}
                }
                i += 1;
            }
            let pass = pass.ok_or("--passphrase required")?;
            let input = input.ok_or("--in required")?;
            let sealed = std::fs::read(input).map_err(|e| e.to_string())?;
            let man = inspect_handoff(&pass, &sealed).map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&man).unwrap_or_default());
            Ok(())
        }
        "apply" => {
            let mut pass = None;
            let mut input = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--passphrase" | "-p" => {
                        i += 1;
                        pass = Some(args.get(i).ok_or("needs pass")?.clone());
                    }
                    "--in" | "-i" => {
                        i += 1;
                        input = Some(PathBuf::from(args.get(i).ok_or("needs path")?));
                    }
                    _ => {}
                }
                i += 1;
            }
            let pass = pass.ok_or("--passphrase required")?;
            let input = input.ok_or("--in required")?;
            let sealed = std::fs::read(input).map_err(|e| e.to_string())?;
            let flags = extract_db_from_all(args)?;
            let (ledger, tmp) = open_db(&flags)?;
            let (ins, skip, man) =
                apply_handoff_merge(&pass, &sealed, &ledger).map_err(|e| e.to_string())?;
            println!(
                "handoff_apply | from={} | inserted={ins} skipped={skip} | schema={}",
                man.device_label, man.schema_version
            );
            maybe_reseal(&flags, &ledger, tmp)?;
            Ok(())
        }
        other => Err(format!("unknown handoff subcommand {other}")),
    }
}

fn cmd_export(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: rradar export <csv|json> [-o file] [--year Y --month M]".into());
    }
    let kind = args[0].as_str();
    let mut out: Option<PathBuf> = None;
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("-o needs value")?));
            }
            "--year" => {
                i += 1;
                year = Some(
                    args.get(i)
                        .ok_or("needs year")?
                        .parse()
                        .map_err(|_| "bad year")?,
                );
            }
            "--month" => {
                i += 1;
                month = Some(
                    args.get(i)
                        .ok_or("needs month")?
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
    let rows = if let (Some(y), Some(m)) = (year, month) {
        ledger
            .list_by_month(y, m, 100_000)
            .map_err(|e| e.to_string())?
    } else {
        ledger.export_all().map_err(|e| e.to_string())?
    };
    let body = match kind {
        "csv" => transactions_to_csv(&rows).map_err(|e| e.to_string())?,
        "json" => transactions_to_json(&rows).map_err(|e| e.to_string())?,
        _ => return Err("export kind must be csv or json".into()),
    };
    if let Some(p) = out {
        std::fs::write(&p, body.as_bytes()).map_err(|e| e.to_string())?;
        println!("wrote | {} | rows={}", p.display(), rows.len());
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
        return Err(
            "usage: rradar backup <create|restore|info|verify> ... (local-only; no cloud)".into(),
        );
    }
    match args[0].as_str() {
        "info" | "inspect" => {
            let mut input = None;
            let mut pass = None;
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
                    _ => {}
                }
                i += 1;
            }
            let input = input.ok_or("--in required")?;
            let pass = pass.ok_or("--passphrase required")?;
            let sealed = std::fs::read(&input).map_err(|e| e.to_string())?;
            let info = inspect_backup(&pass, &sealed).map_err(|e| e.to_string())?;
            println!("backup info | {}", input.display());
            println!(
                "  package_schema={}  ledger_schema={}  txs={}  app={}  created={}",
                info.manifest.schema_version,
                info.manifest.ledger_schema_version,
                info.manifest.transaction_count,
                info.manifest.app_version,
                info.manifest.created_at
            );
            println!(
                "  has_sqlite={}  has_transactions_json={}",
                info.has_sqlite, info.has_transactions_json
            );
            for f in &info.files {
                println!("  file | {:>8} B  {}", f.bytes, f.name);
            }
            println!("  policy | local-first; multi-device = copy this file (no official relay)");
            Ok(())
        }
        "verify" => {
            let mut input = None;
            let mut pass = None;
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
                    _ => {}
                }
                i += 1;
            }
            let input = input.ok_or("--in required")?;
            let pass = pass.ok_or("--passphrase required")?;
            let sealed = std::fs::read(&input).map_err(|e| e.to_string())?;
            let m = verify_backup(&pass, &sealed).map_err(|e| e.to_string())?;
            println!(
                "backup verify | OK  txs={}  ledger_schema={}  app={}",
                m.transaction_count, m.ledger_schema_version, m.app_version
            );
            Ok(())
        }
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
            let mut merge = false;
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
                    "--merge" => merge = true,
                    _ => {}
                }
                i += 1;
            }
            let input = input.ok_or("--in required")?;
            let pass = pass.ok_or("--passphrase required")?;
            let db = db.unwrap_or_else(default_db_path);
            let sealed = std::fs::read(&input).map_err(|e| e.to_string())?;
            let restored = restore_backup(&pass, &sealed).map_err(|e| e.to_string())?;
            if merge {
                let rows = transactions_from_backup(&restored).map_err(|e| e.to_string())?;
                let flags = DbFlags {
                    db: db.clone(),
                    passphrase: None,
                };
                let _ = ensure_data_dir();
                if let Some(parent) = flags.db.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let (ledger, tmp) = open_db(&flags)?;
                let (ins, skip) = ledger
                    .import_transactions(&rows)
                    .map_err(|e| e.to_string())?;
                println!(
                    "restored(merge)\tinserted={ins}\tskipped={skip}\t-> {}",
                    db.display()
                );
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
            } else {
                let sqlite = restored
                    .sqlite_bytes
                    .ok_or("backup missing ledger.sqlite")?;
                write_restored_db(&db, &sqlite).map_err(|e| e.to_string())?;
                // Open once so migrations apply if restoring older schema snapshot.
                let ledger = rradar_core::Ledger::open(&db).map_err(|e| e.to_string())?;
                println!(
                    "restored\t{} txs\tschema={}\t-> {}",
                    restored.manifest.transaction_count,
                    ledger.schema_version().unwrap_or_default(),
                    db.display()
                );
            }
            Ok(())
        }
        other => Err(format!(
            "unknown backup subcommand {other} — try create|restore|info|verify"
        )),
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
    println!("date       | cur | amount     | category              | merchant             | id");
    println!("-----------+-----+------------+-----------------------+----------------------+----");
    for t in rows {
        let m = Money::new(
            t.amount_minor,
            Iso4217::parse(&t.currency).unwrap_or(Iso4217::TWD),
        );
        let merch: String = t.merchant.chars().take(20).collect();
        let cat: String = t.category.chars().take(21).collect();
        let date = t.transacted_at.get(..10).unwrap_or(&t.transacted_at);
        let amt = format!("{:>10}", m.display_major());
        println!(
            "{date} | {cur:<3} | {amt} | {cat:<21} | {merch:<20} | {id}",
            cur = t.currency,
            id = t.id,
        );
    }
}

fn default_currency_from_env() -> Iso4217 {
    if let Ok(s) = std::env::var("RRADAR_DEFAULT_CURRENCY") {
        if let Some(c) = Iso4217::parse(&s) {
            return c;
        }
    }
    AppConfig::load().currency()
}

fn print_topic_help(topic: &str) -> Result<(), String> {
    let text = match topic {
        "process" | "add" => "\
process <files…> [options]
  Parse receipt text/image (mock OCR by default). Multiple files = batch.
  --confirm, -c     write to ledger (default db)
  --explain         show amount candidates / rules
  --json --quiet -q
  --engine mock|onnx
  --currency CODE   (or RRADAR_DEFAULT_CURRENCY)
  --qr STR | --qr-file PATH
  --merchant --amount --category --date --notes
  --force           override hard dedupe
  --db PATH -p PASS",
        "manual" | "entry" => "\
manual --merchant NAME --amount MAJOR [--currency TWD] [--category ID] [--date YYYY-MM-DD] [--notes N]
  Insert a transaction without OCR. Alias: entry",
        "list" | "ls" | "search" => "\
list [--json] [--limit N] [--offset N] [--currency C] [--query Q] [--db PATH]
  Aliases: ls, search. Table output uses | separators.",
        "stats" => "\
stats [--year Y --month M | --from DATE --to DATE | --all] [--db PATH]
  Per-currency totals only (never mixes currencies). Default: current UTC month.",
        "top" => "\
top [--currency TWD] [--limit 10] [--db PATH]
  Top merchants by spend within one currency.",
        "clear" => "\
clear --yes [--db PATH]
  Delete ALL transactions (irreversible without backup).",
        "export" => "\
export csv|json [-o file] [--db PATH]
  CSV includes UTF-8 BOM for Excel.",
        "import" => "\
import json <file.json> [--db PATH]
import backup --in file.rradar -p PASS [--db PATH]
  JSON array or merge transactions from encrypted backup (skip existing ids).",
        "migrate" => "\
migrate [--db PATH]
  Open ledger, apply schema migrations, print version and count.",
        "models" => "\
models [status|verify|pins] [--dir models]
  status/pins  show det/rec/cls paths + SHA-256 pin checks (manifest.sha256)
  verify       exit 1 unless every pin file is present and hashes match
  Fetch weights: tools/fetch-models.ps1 | tools/fetch-models.sh
  Pins are committed; .onnx weights are not (see models/README.md).",
        "backup" => "\
backup create -p PASS [-o file] [--db PATH]
backup restore --in file -p PASS [--db PATH] [--merge]
backup info|verify --in file -p PASS
  Argon2id + XChaCha20-Poly1305. Local-only multi-device via file copy.
  --merge inserts missing txs into existing ledger. RRADAR_FAST_BACKUP=1 for tests.",
        "seal" | "unseal" => "\
seal [--db ledger.db] -o file.rrsealed -p PASS
unseal --in file.rrsealed -o ledger.db -p PASS
  Whole-file at-rest encryption (P2).",
        "demo" => "\
demo [--fixtures DIR] [--db PATH] [--no-backup] [--quiet]
  Isolated closed-loop demo for recording / CI:
  text + mock_ocr fixtures → confirm → list/stats/top → export → backup.
  Default demo db: %APPDATA%/receiptradar/demo/ledger.db (fresh each run).
  Does not touch the default user ledger unless --db or RRADAR_DB is set.
  RRADAR_FIXTURES overrides fixtures root discovery.",
        "init" | "doctor" | "path" | "config" => "\
init     create data dir + empty ledger + default config.toml
config [show|set default_currency TWD|set list_limit 50]
doctor   health check (schema version, engines, models)
path     print home + db paths
demo     one-command closed-loop from fixtures/ (recordable)
  RRADAR_HOME / RRADAR_DB / RRADAR_DEFAULT_CURRENCY override defaults.",
        "edit" | "delete" | "show" | "rm" => "\
show <id>
edit <id> [--merchant --amount --currency --category --notes --date]
delete <id> --yes
  edit/delete require --db for non-default ledgers.",
        other => {
            return Err(format!(
                "no help topic `{other}` — try: process manual list stats export import backup seal"
            ));
        }
    };
    println!("rradar {topic}\n{text}");
    Ok(())
}

fn print_help() {
    println!(
        "\
rradar — ReceiptRadar CLI (local-first ledger)
{PRODUCT_ID} {VERSION}

Quick start:
  rradar demo
  rradar init
  rradar process fixtures/text/familymart_89.txt --confirm --explain
  rradar list
  rradar stats
  rradar export csv -o out.csv

Commands:
  init                 Create data dir + empty ledger + config
  config               Show/set local config.toml
  doctor               Environment / db check
  demo                 One-command closed-loop demo (fixtures → ledger)
  path                 Print default home & db paths
  process <files…>     Parse receipt(s) (alias: add); batch OK
  manual               Manual entry without OCR (alias: entry)
  import json <file>   Import transactions JSON array
  list                 List transactions (alias: ls, search)
  count                Transaction count
  last                 Show most recently confirmed row (JSON)
  undo --yes           Delete most recently confirmed row
  show <id>            Show one transaction (JSON)
  edit <id>            Edit merchant/amount/category/notes/date
  delete <id> --yes    Delete transaction (alias: rm)
  stats                Per-currency totals; --by-category for breakdown
  top                  Top merchants by spend (one currency)
  report               Markdown monthly report (-o file.md)
  inbox [--ensure]     Show default drop folder (RRADAR_INBOX)
  watch [dir]          Auto-process new files (default: inbox)
  serve [--bind 127.0.0.1:7432]  Local-only HTTP API
  recategorize         Re-run category rules (default: only `other`)
  clear --yes          Wipe all transactions
  categories           List category ids
  rules                Merchant rule packs (list|install|ensure)
  handoff              Multi-device encrypted package (create|info|apply)
  export csv|json      Export ledger
  backup create|restore|info|verify
  import json|backup   Import JSON array or merge from .rradar
  migrate              Apply/report ledger schema migrations
  models               ONNX pack status / SHA-256 pin verify
  seal / unseal        Whole-file encryption (.rrsealed)

process options:
  --confirm, -c        Write to ledger (default db if --db omitted)
  --explain            Show rules / amount candidates
  --json               JSON output
  --engine mock|onnx   OCR backend (default mock; onnx needs --features onnx + models)
  --currency TWD|USD|… Default currency fallback
  --qr STR / --qr-file Path to TW e-invoice left QR
  --merchant --amount --category --date --notes
  --force              Override hard dedupe
  --db PATH -p PASS    Ledger path / sealed passphrase

Global data:
  Default db:  %APPDATA%\\receiptradar\\ledger.db  (or $XDG_DATA_HOME/receiptradar)
  Override:    RRADAR_HOME, RRADAR_DB, RRADAR_DEFAULT_CURRENCY
  Fast backup: RRADAR_FAST_BACKUP=1  (weaker Argon2 for tests)

  rradar help <command>   detailed topic help

No cloud. No account. Core path works offline.
"
    );
}

#[allow(dead_code)]
fn _keep(p: &Path) {
    let _ = p;
}
