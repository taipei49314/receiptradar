//! `rradar` — complete local-first receipt ledger CLI.

mod serve;

use rradar_core::{
    annual_markdown, apply_edits, apply_handoff_merge, attachments_root_for_db,
    budget_status_month, category_engine_with_packs, create_backup, create_handoff, data_dir,
    default_db_path, ensure_data_dir, ensure_inbox_dir, ensure_rules_dir, inbox_dir,
    inspect_backup, inspect_handoff, install_rule_pack, list_rule_files, monthly_markdown,
    monthly_markdown_with_budgets, normalize_tags, open_ledger_auto, process_path,
    remove_stored_attachment, resolve_attachment_path, restore_backup, rules_dir, save_sealed,
    store_attachment, transactions_from_backup, transactions_from_csv, transactions_to_csv,
    transactions_to_json, utc_now_iso, verify_backup, write_handoff_file, write_restored_aliases,
    write_restored_attachments, write_restored_budgets, write_restored_db, AliasBook, AppConfig,
    BudgetBook, Iso4217, Money, ProcessOptions, PurgeReport, ReceiptDraft, Transaction, TxFilter,
    TxUpdate, UserEdits, LEDGER_SCHEMA_VERSION, PRODUCT_ID, VERSION,
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
        "engines" => cmd_engines(&args[1..]),
        "licenses" | "notices" => cmd_licenses(&args[1..]),
        "release-check" | "self-check" => cmd_release_check(&args[1..]),
        "measure" | "probe" => cmd_measure(&args[1..]),
        "demo" => cmd_demo(&args[1..]),
        "day" => cmd_day(&args[1..]),
        "fixtures" => cmd_fixtures(&args[1..]),
        "process" => cmd_process(&args[1..], false),
        // Daily path: `add` writes to the ledger by default (no need for -c).
        "add" => cmd_process(&args[1..], true),
        "today" | "home" | "status" => cmd_today(&args[1..]),
        "ocr" => cmd_ocr(&args[1..]),
        "bench" => cmd_bench(&args[1..]),
        "manual" | "entry" => cmd_manual(&args[1..]),
        "import" => cmd_import(&args[1..]),
        "list" | "ls" | "search" => cmd_list(&args[1..]),
        "count" => cmd_count(&args[1..]),
        "tags" => cmd_tags(&args[1..]),
        "budget" => cmd_budget(&args[1..]),
        "aliases" | "alias" => cmd_aliases(&args[1..]),
        "last" | "undo" => cmd_last_or_undo(&args[0], &args[1..]),
        "show" => cmd_show(&args[1..]),
        "delete" | "rm" => cmd_delete(&args[1..]),
        "trash" => cmd_trash(&args[1..]),
        "restore" => cmd_restore(&args[1..]),
        "purge" => cmd_purge(&args[1..]),
        "edit" => cmd_edit(&args[1..]),
        "attach" => cmd_attach(&args[1..]),
        "detach" => cmd_detach(&args[1..]),
        "stats" => cmd_stats(&args[1..]),
        "top" => cmd_top(&args[1..]),
        "report" => cmd_report(&args[1..]),
        "month" | "close" | "monthly" => cmd_month(&args[1..]),
        "watch" => cmd_watch(&args[1..]),
        "scoop" | "catch" => cmd_scoop(&args[1..]),
        "inbox" => cmd_inbox(&args[1..]),
        "serve" => cmd_serve(&args[1..]),
        "api-smoke" => cmd_api_smoke(&args[1..]),
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
            let db = default_db_path();
            println!("home        | {}", data_dir().display());
            println!("db          | {}", db.display());
            println!("inbox       | {}", inbox_dir().display());
            println!("attachments | {}", attachments_root_for_db(&db).display());
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
    let models = rradar_ocr::default_models_dir();
    let ready = rradar_ocr::probe_onnx_readiness(&models);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "product_id": PRODUCT_ID,
                "version": VERSION,
                "ledger_schema": LEDGER_SCHEMA_VERSION,
                "soft_delete": LEDGER_SCHEMA_VERSION >= 4,
                "onnx_feature": ready.feature_enabled,
                "onnx_ready": ready.ready_for_inference,
                "auto_engine": rradar_ocr::resolve_auto_engine_name(),
                "models_dir": ready.models_dir,
                "os": env::consts::OS,
                "arch": env::consts::ARCH,
                "policy": "local-first; no official cloud relay",
                "release_features": [
                    "mock-ocr",
                    "soft-delete",
                    "backup",
                    "handoff",
                    "csv-import",
                    "local-http",
                ],
            })
        );
        return Ok(());
    }
    println!("{PRODUCT_ID} {VERSION}");
    if long {
        println!("ledger_schema | {LEDGER_SCHEMA_VERSION}");
        println!(
            "soft_delete   | {}",
            if LEDGER_SCHEMA_VERSION >= 4 {
                "yes (trash/restore/purge)"
            } else {
                "no"
            }
        );
        println!(
            "onnx_feature  | {}",
            if ready.feature_enabled {
                "enabled"
            } else {
                "disabled (build with --features onnx)"
            }
        );
        println!(
            "onnx_ready    | {} ({})",
            ready.ready_for_inference, ready.hint
        );
        println!("auto_engine   | {}", rradar_ocr::resolve_auto_engine_name());
        println!("target        | {}-{}", env::consts::OS, env::consts::ARCH);
        println!("policy        | local-first; no official cloud relay");
    }
    Ok(())
}

fn cmd_engines(args: &[String]) -> Result<(), String> {
    let json = args.iter().any(|a| a == "--json");
    if json {
        println!("{}", rradar_ocr::engines_catalog_json());
        return Ok(());
    }
    let ready = rradar_ocr::probe_onnx_readiness(rradar_ocr::default_models_dir());
    let auto = rradar_ocr::resolve_auto_engine_name();
    println!("rradar engines (local-first OCR; no cloud)");
    println!("  mock  | available | fixtures, CI, default");
    println!(
        "  onnx  | {} | feature={} models={} pins={}/{} ort={}",
        if ready.ready_for_inference {
            "ready"
        } else {
            "not ready"
        },
        ready.feature_enabled,
        ready.models_present,
        ready.pin_ok_count,
        ready.pin_total,
        ready.ort_found
    );
    println!("  auto  | resolves → {auto}");
    println!("  dir   | {}", ready.models_dir);
    println!("  hint  | {}", ready.hint);
    println!("  use   | rradar process FILE --engine mock|onnx|auto");
    println!("  docs  | models/README.md · scripts/smoke-onnx.ps1");
    Ok(())
}

/// Pre-flight gate for release / install verification (local-only; no network).
///
/// Checks identity, engines catalog, schema constant, optional process fixture,
/// optional demo closed-loop, optional local API smoke.
fn cmd_release_check(args: &[String]) -> Result<(), String> {
    let mut fixtures_root: Option<PathBuf> = None;
    let mut skip_demo = false;
    let mut skip_api = false;
    let mut quiet = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fixtures" => {
                i += 1;
                fixtures_root = Some(PathBuf::from(args.get(i).ok_or("--fixtures needs path")?));
            }
            "--skip-demo" => skip_demo = true,
            "--skip-api" => skip_api = true,
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => {
                println!(
                    "rradar release-check [--fixtures DIR] [--skip-demo] [--skip-api] [--quiet]\n  \
                     Local pre-flight for install/release (no network, no cloud).\n  \
                     Alias: self-check"
                );
                return Ok(());
            }
            other => return Err(format!("unknown release-check flag `{other}`")),
        }
        i += 1;
    }

    let mut failed = 0usize;
    let mut step = |name: &str, ok: bool, detail: &str| {
        if ok {
            if !quiet {
                println!("  OK  | {name} | {detail}");
            }
        } else {
            failed += 1;
            println!("  FAIL| {name} | {detail}");
        }
    };

    if !quiet {
        println!("rradar release-check (local-first; no cloud)");
    }

    // 1) Identity
    step(
        "version",
        !VERSION.is_empty() && !PRODUCT_ID.is_empty(),
        &format!("{PRODUCT_ID} {VERSION}"),
    );
    step(
        "ledger_schema",
        LEDGER_SCHEMA_VERSION >= 4,
        &format!("supports v{LEDGER_SCHEMA_VERSION} (soft-delete)"),
    );

    // 2) Engines catalog
    let eng_json = rradar_ocr::engines_catalog_json();
    step(
        "engines",
        eng_json.contains("auto_resolves_to") && eng_json.contains("mock"),
        &format!(
            "auto→{} onnx_ready={}",
            rradar_ocr::resolve_auto_engine_name(),
            rradar_ocr::probe_onnx_readiness(rradar_ocr::default_models_dir()).ready_for_inference
        ),
    );

    // 3) Policy constants (string presence in catalog)
    step(
        "policy",
        eng_json.contains("no cloud") || eng_json.contains("local-first"),
        "local-first OCR catalog",
    );

    // 3b) Supply-chain notices (files in package / source tree)
    let notices = find_repo_file("THIRD_PARTY_NOTICES");
    let license = find_repo_file("LICENSE");
    step(
        "license_file",
        license.as_ref().map(|p| p.is_file()).unwrap_or(false),
        &license
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "LICENSE not found".into()),
    );
    step(
        "third_party_notices",
        notices
            .as_ref()
            .map(|p| p.is_file() && p.metadata().map(|m| m.len() > 32).unwrap_or(false))
            .unwrap_or(false),
        &notices
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "THIRD_PARTY_NOTICES not found".into()),
    );

    let fixtures = fixtures_root
        .or_else(|| env::var_os("RRADAR_FIXTURES").map(PathBuf::from))
        .unwrap_or_else(find_fixtures_dir);

    // 4) Process one fixture (mock)
    let fam = fixtures.join("text/familymart_89.txt");
    if fam.is_file() {
        let eng = engine_by_name("mock").map_err(|e| e.to_string())?;
        let cats = category_engine_with_packs();
        match process_path(
            &fam,
            eng.as_ref(),
            &cats,
            ProcessOptions {
                default_currency: Iso4217::TWD,
                ..Default::default()
            },
        ) {
            Ok(d) => step(
                "process_mock",
                d.total.value.amount_minor == 8900,
                &format!(
                    "{} minor={} {}",
                    d.merchant.value, d.total.value.amount_minor, d.total.value.currency
                ),
            ),
            Err(e) => step("process_mock", false, &e.to_string()),
        }
    } else {
        step(
            "process_mock",
            false,
            &format!("fixture missing: {} (pass --fixtures)", fam.display()),
        );
    }

    // 4b) Soft-delete lifecycle + integrity (schema v4, local-only)
    {
        let home =
            std::env::temp_dir().join(format!("rradar-release-check-trash-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&home);
        let db = home.join("ledger.db");
        match rradar_core::Ledger::open(&db) {
            Ok(ledger) => {
                let eng = engine_by_name("mock").map_err(|e| e.to_string())?;
                let cats = category_engine_with_packs();
                let draft_ok = if fam.is_file() {
                    match process_path(
                        &fam,
                        eng.as_ref(),
                        &cats,
                        ProcessOptions {
                            default_currency: Iso4217::TWD,
                            ..Default::default()
                        },
                    ) {
                        Ok(d) => {
                            match ledger.confirm_draft(&d, None, Some("release-check"), false) {
                                Ok(r) if r.inserted => Some(r.transaction.id),
                                Ok(_) => None,
                                Err(e) => {
                                    step("soft_delete", false, &format!("confirm: {e}"));
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            step("soft_delete", false, &format!("process: {e}"));
                            None
                        }
                    }
                } else {
                    // Minimal synthetic draft path when fixtures missing.
                    None
                };
                if let Some(id) = draft_ok {
                    let soft = ledger.soft_delete_transaction(&id).unwrap_or(false);
                    let active0 = ledger.count().unwrap_or(-1) == 0;
                    let trash1 = ledger.count_trash().unwrap_or(0) == 1;
                    let restored = ledger.restore_transaction(&id).unwrap_or(false);
                    let active1 = ledger.count().unwrap_or(0) == 1;
                    let purged = ledger
                        .purge_transaction(&id)
                        .map(|report| report.purged_any())
                        .unwrap_or(false);
                    let integ = ledger.integrity_check().ok();
                    let integ_ok = integ.as_ref().map(|i| i.pragma_ok).unwrap_or(false);
                    let ok = soft && active0 && trash1 && restored && active1 && purged && integ_ok;
                    step(
                        "soft_delete",
                        ok,
                        &format!(
                            "trash→restore→purge integrity={}",
                            integ
                                .as_ref()
                                .map(|i| i.pragma_message.as_str())
                                .unwrap_or("?")
                        ),
                    );
                    step(
                        "integrity",
                        integ_ok,
                        &format!(
                            "schema={} active={}",
                            integ.as_ref().map(|i| i.schema_version).unwrap_or(0),
                            integ.as_ref().map(|i| i.active_count).unwrap_or(-1)
                        ),
                    );
                } else if fam.is_file() {
                    // already stepped fail
                } else {
                    step("soft_delete", false, "no fixture for trash smoke");
                    step("integrity", false, "skipped");
                }
            }
            Err(e) => {
                step("soft_delete", false, &e.to_string());
                step("integrity", false, "ledger open failed");
            }
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    // 5) Demo closed-loop
    if !skip_demo {
        if fixtures.is_dir() {
            let home = std::env::temp_dir()
                .join(format!("rradar-release-check-demo-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&home);
            let db = home.join("ledger.db");
            let demo_args = vec![
                "--fixtures".into(),
                fixtures.display().to_string(),
                "--db".into(),
                db.display().to_string(),
                "--quiet".into(),
            ];
            match cmd_demo(&demo_args) {
                Ok(()) => step("demo", true, &format!("db={}", db.display())),
                Err(e) => step("demo", false, &e),
            }
            let _ = std::fs::remove_dir_all(&home);
        } else {
            step("demo", false, "fixtures dir missing");
        }
    } else if !quiet {
        println!("  skip| demo");
    }

    // 6) Local API smoke
    if !skip_api {
        if fam.is_file() {
            let home = std::env::temp_dir()
                .join(format!("rradar-release-check-api-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&home);
            let db = home.join("ledger.db");
            let _ = rradar_core::Ledger::open(&db);
            match serve::smoke_local_api(&db, &fam, None) {
                Ok(rep) => step(
                    "api_smoke",
                    rep.health_ok && rep.process_confirmed && rep.transactions_n >= 1,
                    &format!("bind={} txs={}", rep.bind, rep.transactions_n),
                ),
                Err(e) => step("api_smoke", false, &e),
            }
            let _ = std::fs::remove_dir_all(&home);
        } else {
            step("api_smoke", false, "fixture missing for api-smoke");
        }
    } else if !quiet {
        println!("  skip| api_smoke");
    }

    if failed == 0 {
        println!("RELEASE_CHECK_OK schema={LEDGER_SCHEMA_VERSION} version={VERSION}");
        Ok(())
    } else {
        Err(format!(
            "release-check failed ({failed} check(s)); see FAIL lines above"
        ))
    }
}

/// Behavioral measurer for the daily-path surface (Phases 1–7).
/// Does **not** trust features until each probe passes. Prints explicit blind spots.
fn cmd_measure(args: &[String]) -> Result<(), String> {
    let mut fixtures_root: Option<PathBuf> = None;
    let mut quiet = false;
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fixtures" => {
                i += 1;
                fixtures_root = Some(PathBuf::from(args.get(i).ok_or("--fixtures needs path")?));
            }
            "--quiet" | "-q" => quiet = true,
            "--json" => json = true,
            "--help" | "-h" => {
                print_topic_help("measure")?;
                return Ok(());
            }
            other => return Err(format!("unknown measure flag `{other}`")),
        }
        i += 1;
    }

    let fixtures = fixtures_root
        .or_else(|| env::var_os("RRADAR_FIXTURES").map(PathBuf::from))
        .unwrap_or_else(find_fixtures_dir);
    if !fixtures.is_dir() {
        return Err(format!(
            "fixtures not found at {} — run from repo root or pass --fixtures",
            fixtures.display()
        ));
    }

    // Isolated sandbox — never touch the personal ledger.
    let stamp = utc_now_iso().replace([':', '-'], "");
    let home = std::env::temp_dir().join(format!("rradar-measure-{stamp}"));
    let db = home.join("ledger.db");
    let inbox = home.join("inbox");
    let _ = std::fs::create_dir_all(&inbox);
    let prev_home = env::var_os("RRADAR_HOME");
    let prev_db = env::var_os("RRADAR_DB");
    let prev_inbox = env::var_os("RRADAR_INBOX");
    env::set_var("RRADAR_HOME", &home);
    env::set_var("RRADAR_DB", &db);
    env::set_var("RRADAR_INBOX", &inbox);

    let mut probes: Vec<serde_json::Value> = Vec::new();
    let mut failed = 0usize;
    let mut record = |id: &str, ok: bool, detail: &str| {
        if !ok {
            failed += 1;
        }
        if !quiet && !json {
            let mark = if ok { "PASS" } else { "FAIL" };
            println!("  {mark} | {id} | {detail}");
        }
        probes.push(serde_json::json!({
            "id": id,
            "ok": ok,
            "detail": detail,
        }));
    };

    if !quiet && !json {
        println!("rradar measure — daily-path behavioral probes (isolated)");
        println!("sandbox | {}", home.display());
        println!("trust   | only PASS probes; FAIL = do not trust yet");
        println!();
    }

    // --- Probe matrix -------------------------------------------------------
    let fam = fixtures.join("text/familymart_89.txt");
    let tea = fixtures.join("text/bubbletea_50lan_tw.txt");
    let cht = fixtures.join("text/cht_bill_tw.txt");
    let ibon = fixtures.join("text/ibon_print_tw.txt");

    // P1: add confirms without -c
    if fam.is_file() {
        let r = cmd_process(
            &[
                fam.display().to_string(),
                "--quiet".into(),
                "--db".into(),
                db.display().to_string(),
            ],
            true,
        );
        let n = open_db(&DbFlags {
            db: db.clone(),
            passphrase: None,
        })
        .ok()
        .and_then(|(led, tmp)| {
            let c = led.count().ok();
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            c
        });
        record(
            "add_default_confirm",
            r.is_ok() && n == Some(1),
            &format!("status={r:?} count={n:?}"),
        );
    } else {
        record("add_default_confirm", false, "fixture missing");
    }

    // P1: preview does not write
    let before = open_db(&DbFlags {
        db: db.clone(),
        passphrase: None,
    })
    .ok()
    .and_then(|(led, tmp)| {
        let c = led.count().ok();
        if let Some(t) = tmp {
            let _ = std::fs::remove_file(t);
        }
        c
    })
    .unwrap_or(0);
    if fam.is_file() {
        let _ = cmd_process(
            &[
                fam.display().to_string(),
                "--preview".into(),
                "--quiet".into(),
                "--db".into(),
                db.display().to_string(),
            ],
            true,
        );
        let after = open_db(&DbFlags {
            db: db.clone(),
            passphrase: None,
        })
        .ok()
        .and_then(|(led, tmp)| {
            let c = led.count().ok();
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            c
        })
        .unwrap_or(0);
        record(
            "add_preview_no_write",
            after == before,
            &format!("before={before} after={after}"),
        );
    } else {
        record("add_preview_no_write", false, "fixture missing");
    }

    // Wipe for clean as-today / today probes
    wipe_sqlite(&db);

    // P2: --as-today lands in current UTC month
    if fam.is_file() {
        let _ = cmd_process(
            &[
                fam.display().to_string(),
                "--as-today".into(),
                "--quiet".into(),
                "--db".into(),
                db.display().to_string(),
            ],
            true,
        );
        let (y, m) = current_year_month();
        let period = format!("{y:04}-{m:02}");
        let ok = open_db(&DbFlags {
            db: db.clone(),
            passphrase: None,
        })
        .ok()
        .and_then(|(led, tmp)| {
            let rows = led.list_by_month(y, m, 10).ok();
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            rows
        })
        .map(|r| !r.is_empty() && r[0].transacted_at.starts_with(&period))
        .unwrap_or(false);
        record(
            "as_today_current_month",
            ok,
            &format!("expect period {period}"),
        );
    } else {
        record("as_today_current_month", false, "fixture missing");
    }

    // P1/P2: today glance + short merchant display
    {
        let mut aliases = AliasBook::default();
        let _ = aliases.ensure_tw_defaults();
        let _ = aliases.save();
        // Capture today human output via subprocess would be heavy; probe via APIs + display helper.
        let short = display_merchant_name("全家便利商店 臨江店");
        record(
            "merchant_display_short",
            short == "全家" || !short.contains("臨江"),
            &format!("display={short}"),
        );
        let (y, m) = current_year_month();
        let stats_ok = open_db(&DbFlags {
            db: db.clone(),
            passphrase: None,
        })
        .ok()
        .and_then(|(led, tmp)| {
            let s = led.stats_by_currency_month(y, m).ok();
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            s
        })
        .map(|s| !s.is_empty())
        .unwrap_or(false);
        record(
            "today_month_stats",
            stats_ok,
            "stats_by_currency_month non-empty after as-today add",
        );
    }

    // P2: amount ranking — 價稅合計 / 合計 over 應稅
    {
        use rradar_core::extract::extract_l1_fields;
        use rradar_core::{ExplainTrace, TextBlock};
        let blocks = |lines: &[&str]| -> Vec<TextBlock> {
            lines
                .iter()
                .map(|t| TextBlock {
                    text: (*t).into(),
                    confidence: 1.0,
                })
                .collect()
        };
        let mut ex = ExplainTrace::new("measure", "ocr");
        let f = extract_l1_fields(
            &blocks(&["店", "應稅銷售額 100", "營業稅 5", "價稅合計 105"]),
            Iso4217::TWD,
            &mut ex,
        );
        let ok = f
            .total
            .as_ref()
            .map(|t| t.value.amount_minor == 10500)
            .unwrap_or(false);
        record(
            "extract_prefer_price_tax_total",
            ok,
            &format!("minor={:?}", f.total.as_ref().map(|t| t.value.amount_minor)),
        );
    }

    // P2: ibon category longest-match
    {
        let eng = category_engine_with_packs();
        let mut ex = rradar_core::ExplainTrace::new("measure", "ocr");
        let cat = eng.categorize("7-ELEVEN ibon", "列印", &mut ex);
        record(
            "category_ibon_not_seven",
            cat.value == "shopping",
            &format!("category={}", cat.value),
        );
    }

    // P4/P5: scoop + archive + second scoop empty
    wipe_sqlite(&db);
    if fam.is_file() && tea.is_file() {
        let _ = std::fs::copy(&fam, inbox.join("familymart_89.txt"));
        let _ = std::fs::copy(&tea, inbox.join("bubbletea_50lan_tw.txt"));
        // call scoop quietly via argv
        let scoop1 = cmd_scoop(&["--quiet".into(), "--db".into(), db.display().to_string()]);
        let top_left: usize = std::fs::read_dir(&inbox)
            .map(|rd| rd.flatten().filter(|e| e.path().is_file()).count())
            .unwrap_or(99);
        let done_ok = inbox.join("done").is_dir();
        let scoop2 = cmd_scoop(&["--quiet".into(), "--db".into(), db.display().to_string()]);
        record(
            "scoop_archive_clears_inbox",
            scoop1.is_ok() && top_left == 0 && done_ok,
            &format!("scoop1={scoop1:?} top_files={top_left} done_dir={done_ok}"),
        );
        // Second scoop should be empty (n=0) — verify via inbox emptiness already;
        // also ensure count stayed at 2 (no double insert of archived files).
        let count = open_db(&DbFlags {
            db: db.clone(),
            passphrase: None,
        })
        .ok()
        .and_then(|(led, tmp)| {
            let c = led.count().ok();
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            c
        });
        record(
            "scoop_second_is_noop",
            scoop2.is_ok() && count == Some(2),
            &format!("scoop2={scoop2:?} count={count:?}"),
        );
    } else {
        record("scoop_archive_clears_inbox", false, "fixtures missing");
        record("scoop_second_is_noop", false, "fixtures missing");
    }

    // P6/P7: month --csv
    if fam.is_file() {
        let md = home.join("month.md");
        let csv = home.join("month.csv");
        let r = cmd_month(&[
            "--db".into(),
            db.display().to_string(),
            "-o".into(),
            md.display().to_string(),
            "--csv".into(),
            csv.display().to_string(),
            "--quiet".into(),
        ]);
        let csv_ok = csv.is_file()
            && std::fs::read(&csv)
                .map(|b| b.len() >= 3 && b[0..3] == [0xEF, 0xBB, 0xBF])
                .unwrap_or(false);
        record(
            "month_csv_bom",
            r.is_ok() && csv_ok && md.is_file(),
            &format!("month={r:?} csv_bom={csv_ok} md={}", md.is_file()),
        );
    } else {
        record("month_csv_bom", false, "fixture missing");
    }

    // P3: day closed loop (quiet)
    {
        let day_db = home.join("day-ledger.db");
        let r = cmd_day(&[
            "--fixtures".into(),
            fixtures.display().to_string(),
            "--db".into(),
            day_db.display().to_string(),
            "--quiet".into(),
        ]);
        record(
            "day_closed_loop",
            r.is_ok() && day_db.is_file(),
            &format!("day={r:?}"),
        );
    }

    // Optional: cht fixture extracts 699
    if cht.is_file() {
        let eng = engine_by_name("mock").map_err(|e| e.to_string())?;
        let cats = category_engine_with_packs();
        let draft = process_path(
            &cht,
            eng.as_ref(),
            &cats,
            ProcessOptions {
                default_currency: Iso4217::TWD,
                ..Default::default()
            },
        );
        let ok = draft
            .as_ref()
            .map(|d| d.total.value.amount_minor == 69900 && d.category.value == "utilities")
            .unwrap_or(false);
        record(
            "fixture_cht_bill",
            ok,
            &format!(
                "{:?}",
                draft.map(|d| (d.total.value.amount_minor, d.category.value))
            ),
        );
    } else {
        record("fixture_cht_bill", false, "fixture missing");
    }

    if ibon.is_file() {
        let eng = engine_by_name("mock").map_err(|e| e.to_string())?;
        let cats = category_engine_with_packs();
        let draft = process_path(
            &ibon,
            eng.as_ref(),
            &cats,
            ProcessOptions {
                default_currency: Iso4217::TWD,
                ..Default::default()
            },
        );
        let ok = draft
            .as_ref()
            .map(|d| d.total.value.amount_minor == 3500 && d.category.value == "shopping")
            .unwrap_or(false);
        record(
            "fixture_ibon_shopping",
            ok,
            &format!(
                "{:?}",
                draft.map(|d| (d.total.value.amount_minor, d.category.value))
            ),
        );
    } else {
        record("fixture_ibon_shopping", false, "fixture missing");
    }

    // --- Close former blinds with probes ------------------------------------
    let usd = fixtures.join("text/starbucks_usd.txt");
    let qr_file = fixtures.join("qr/tw_einvoice_sample_01.payload.txt");

    // watch --once --as-today
    {
        wipe_sqlite(&db);
        let watch_dir = home.join("watch-inbox");
        let _ = std::fs::create_dir_all(&watch_dir);
        if fam.is_file() {
            let _ = std::fs::copy(&fam, watch_dir.join("watch_fam.txt"));
            let r = cmd_watch(&[
                watch_dir.display().to_string(),
                "--once".into(),
                "--as-today".into(),
                "--db".into(),
                db.display().to_string(),
            ]);
            let (y, m) = current_year_month();
            let n = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let c = led.list_by_month(y, m, 10).ok().map(|r| r.len());
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                c
            })
            .unwrap_or(0);
            record(
                "watch_once_as_today",
                r.is_ok() && n >= 1,
                &format!("watch={r:?} month_rows={n}"),
            );
        } else {
            record("watch_once_as_today", false, "fixture missing");
        }
    }

    // scoop --attach
    {
        wipe_sqlite(&db);
        // clear inbox top-level
        if let Ok(rd) = std::fs::read_dir(&inbox) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        if fam.is_file() {
            let _ = std::fs::copy(&fam, inbox.join("attach_me.txt"));
            let r = cmd_scoop(&[
                "--quiet".into(),
                "--attach".into(),
                "--db".into(),
                db.display().to_string(),
            ]);
            let attached = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let rows = led.list_transactions(10, 0).ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                rows
            })
            .map(|rows| {
                rows.first()
                    .and_then(|t| t.attachment_path.as_ref())
                    .map(|p| !p.is_empty())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
            record(
                "scoop_attach",
                r.is_ok() && attached,
                &format!("scoop={r:?} attached={attached}"),
            );
        } else {
            record("scoop_attach", false, "fixture missing");
        }
    }

    // budget OVER after large add
    {
        wipe_sqlite(&db);
        let mut book = BudgetBook::default();
        let _ = book.set_major("TWD", "50", None);
        let _ = book.save();
        if fam.is_file() {
            let _ = cmd_process(
                &[
                    fam.display().to_string(),
                    "--as-today".into(),
                    "--quiet".into(),
                    "--db".into(),
                    db.display().to_string(),
                ],
                true,
            );
            let (y, m) = current_year_month();
            let over = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let st = budget_status_month(&led, &book, y, m).ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                st
            })
            .map(|st| st.iter().any(|s| s.over))
            .unwrap_or(false);
            record("budget_over_flag", over, "limit=50 spend≈89 should OVER");
        } else {
            record("budget_over_flag", false, "fixture missing");
        }
        // reset budgets for later probes
        let mut clear = BudgetBook::default();
        clear.clear_all();
        let _ = clear.save();
    }

    // QR + --as-today overrides QR date into current month
    {
        wipe_sqlite(&db);
        if fam.is_file() && qr_file.is_file() {
            let r = cmd_process(
                &[
                    fam.display().to_string(),
                    "--qr-file".into(),
                    qr_file.display().to_string(),
                    "--as-today".into(),
                    "--quiet".into(),
                    "--db".into(),
                    db.display().to_string(),
                ],
                true,
            );
            let (y, m) = current_year_month();
            let period = format!("{y:04}-{m:02}");
            let ok = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let rows = led.list_by_month(y, m, 5).ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                rows
            })
            .map(|rows| {
                rows.first()
                    .map(|t| t.transacted_at.starts_with(&period) && t.amount_minor == 8900)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
            record(
                "qr_as_today",
                r.is_ok() && ok,
                &format!("qr+as-today in {period}; {r:?}"),
            );
        } else {
            record("qr_as_today", false, "fixture/qr missing");
        }
    }

    // inbox done/ collision rename
    {
        wipe_sqlite(&db);
        if let Ok(rd) = std::fs::read_dir(&inbox) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        let _ = std::fs::remove_dir_all(inbox.join("done"));
        if fam.is_file() {
            let _ = std::fs::copy(&fam, inbox.join("same.txt"));
            let _ = cmd_scoop(&["--quiet".into(), "--db".into(), db.display().to_string()]);
            // Second drop: same filename, different content → collision rename in done/
            let _ = std::fs::write(inbox.join("same.txt"), "碰撞店\n合計 12\n2024-01-01\n");
            let _ = cmd_scoop(&["--quiet".into(), "--db".into(), db.display().to_string()]);
            let day = utc_today_date();
            let done = inbox.join("done").join(&day);
            let has_same = done.join("same.txt").is_file();
            let has_collision = done.join("same-2.txt").is_file();
            record(
                "inbox_done_collision",
                has_same && has_collision,
                &format!(
                    "same={has_same} same-2={has_collision} dir={}",
                    done.display()
                ),
            );
        } else {
            record("inbox_done_collision", false, "fixture missing");
        }
    }

    // undo after confirm
    {
        wipe_sqlite(&db);
        if fam.is_file() {
            let _ = cmd_process(
                &[
                    fam.display().to_string(),
                    "--as-today".into(),
                    "--quiet".into(),
                    "--db".into(),
                    db.display().to_string(),
                ],
                true,
            );
            let before = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let c = led.count().ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                c
            })
            .unwrap_or(0);
            let u = cmd_last_or_undo(
                "undo",
                &["--yes".into(), "--db".into(), db.display().to_string()],
            );
            let after = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let c = led.count().ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                c
            })
            .unwrap_or(99);
            record(
                "undo_after_confirm",
                u.is_ok() && before == 1 && after == 0,
                &format!("undo={u:?} before={before} after={after}"),
            );
        } else {
            record("undo_after_confirm", false, "fixture missing");
        }
    }

    // multi-currency month CSV
    {
        wipe_sqlite(&db);
        if fam.is_file() && usd.is_file() {
            let _ = cmd_process(
                &[
                    fam.display().to_string(),
                    "--as-today".into(),
                    "--quiet".into(),
                    "--db".into(),
                    db.display().to_string(),
                ],
                true,
            );
            let _ = cmd_process(
                &[
                    usd.display().to_string(),
                    "--as-today".into(),
                    "--quiet".into(),
                    "--db".into(),
                    db.display().to_string(),
                ],
                true,
            );
            let csv = home.join("multi.csv");
            let r = cmd_month(&[
                "--db".into(),
                db.display().to_string(),
                "--csv".into(),
                csv.display().to_string(),
                "--quiet".into(),
            ]);
            let body = std::fs::read_to_string(&csv).unwrap_or_default();
            let ok = r.is_ok()
                && body.contains("TWD")
                && body.contains("USD")
                && (body.contains("8900") || body.contains("全家"))
                && (body.contains("545") || body.contains("STARBUCKS"));
            record(
                "multi_currency_month_csv",
                ok,
                &format!(
                    "month={r:?} has_twd={} has_usd={}",
                    body.contains("TWD"),
                    body.contains("USD")
                ),
            );
        } else {
            record("multi_currency_month_csv", false, "fixtures missing");
        }
    }

    // scoop --attach against .rrsealed (needs passphrase forwarded)
    {
        wipe_sqlite(&db);
        let sealed = home.join("ledger.rrsealed");
        let _ = std::fs::remove_file(&sealed);
        // Ensure plain ledger exists then seal it.
        if let Ok((led, tmp)) = open_db(&DbFlags {
            db: db.clone(),
            passphrase: None,
        }) {
            drop(led);
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
        }
        let seal_r = cmd_seal(&[
            "--db".into(),
            db.display().to_string(),
            "--out".into(),
            sealed.display().to_string(),
            "-p".into(),
            "measure-pass".into(),
        ]);
        if let Ok(rd) = std::fs::read_dir(&inbox) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        if seal_r.is_ok() && fam.is_file() && sealed.is_file() {
            let _ = std::fs::copy(&fam, inbox.join("sealed_attach.txt"));
            let scoop_r = cmd_scoop(&[
                "--quiet".into(),
                "--attach".into(),
                "--db".into(),
                sealed.display().to_string(),
                "-p".into(),
                "measure-pass".into(),
            ]);
            let attached = open_db(&DbFlags {
                db: sealed.clone(),
                passphrase: Some("measure-pass".into()),
            })
            .ok()
            .and_then(|(led, tmp)| {
                let rows = led.list_transactions(5, 0).ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                rows
            })
            .map(|rows| {
                rows.first()
                    .map(|t| {
                        t.attachment_path
                            .as_ref()
                            .map(|p| !p.is_empty())
                            .unwrap_or(false)
                            && t.amount_minor == 8900
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);
            record(
                "scoop_attach_sealed",
                scoop_r.is_ok() && attached,
                &format!("seal={seal_r:?} scoop={scoop_r:?} attached={attached}"),
            );
        } else {
            record(
                "scoop_attach_sealed",
                false,
                &format!("seal={seal_r:?} sealed_exists={}", sealed.is_file()),
            );
        }
    }

    // watch process restart picks up a new drop (fresh seen set)
    {
        wipe_sqlite(&db);
        let watch_dir = home.join("watch-restart");
        let _ = std::fs::create_dir_all(&watch_dir);
        if let Ok(rd) = std::fs::read_dir(&watch_dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        let breakfast = fixtures.join("text/maymorning_breakfast_tw.txt");
        if fam.is_file() && breakfast.is_file() {
            let _ = std::fs::copy(&fam, watch_dir.join("a.txt"));
            let r1 = cmd_watch(&[
                watch_dir.display().to_string(),
                "--once".into(),
                "--as-today".into(),
                "--db".into(),
                db.display().to_string(),
            ]);
            let _ = std::fs::copy(&breakfast, watch_dir.join("b.txt"));
            let r2 = cmd_watch(&[
                watch_dir.display().to_string(),
                "--once".into(),
                "--as-today".into(),
                "--db".into(),
                db.display().to_string(),
            ]);
            let n = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let c = led.count().ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                c
            })
            .unwrap_or(0);
            record(
                "watch_restart_picks_new",
                r1.is_ok() && r2.is_ok() && n == 2,
                &format!("r1={r1:?} r2={r2:?} count={n}"),
            );
        } else {
            record("watch_restart_picks_new", false, "fixtures missing");
        }
    }

    // two OS processes scooping the same inbox (distinct receipts)
    {
        wipe_sqlite(&db);
        if let Ok(rd) = std::fs::read_dir(&inbox) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        let _ = std::fs::remove_dir_all(inbox.join("done"));
        let a = fixtures.join("text/maymorning_breakfast_tw.txt");
        let b = fixtures.join("text/bubbletea_50lan_tw.txt");
        if fam.is_file() && a.is_file() && b.is_file() {
            let _ = std::fs::copy(&a, inbox.join("conc_a.txt"));
            let _ = std::fs::copy(&b, inbox.join("conc_b.txt"));
            let exe = env::current_exe().map_err(|e| e.to_string())?;
            let spawn = |label: &str| {
                std::process::Command::new(&exe)
                    .args(["scoop", "--quiet", "--db"])
                    .arg(&db)
                    .env("RRADAR_HOME", &home)
                    .env("RRADAR_INBOX", &inbox)
                    .env("RRADAR_DB", &db)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| format!("spawn {label}: {e}"))
            };
            let mut c1 = spawn("scoop1")?;
            let mut c2 = spawn("scoop2")?;
            let s1 = c1.wait().map_err(|e| e.to_string())?;
            let s2 = c2.wait().map_err(|e| e.to_string())?;
            let n = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let c = led.count().ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                c
            })
            .unwrap_or(0);
            let leftover = std::fs::read_dir(&inbox)
                .map(|rd| rd.flatten().filter(|e| e.path().is_file()).count())
                .unwrap_or(99);
            record(
                "concurrent_scoop",
                s1.success() && s2.success() && n == 2 && leftover == 0,
                &format!(
                    "exit1={} exit2={} count={n} leftover={leftover}",
                    s1.code().unwrap_or(-1),
                    s2.code().unwrap_or(-1)
                ),
            );
        } else {
            record("concurrent_scoop", false, "fixtures missing");
        }
    }

    // onnx: only PASS when feature+models ready; otherwise remain BLIND below
    let onnx_ready = rradar_ocr::onnx_feature_enabled()
        && rradar_ocr::probe_onnx_readiness(rradar_ocr::default_models_dir()).ready_for_inference;
    if onnx_ready {
        let img = fixtures.join("images/familymart_photo.png");
        if img.is_file() {
            wipe_sqlite(&db);
            let r = cmd_process(
                &[
                    img.display().to_string(),
                    "--engine".into(),
                    "onnx".into(),
                    "--as-today".into(),
                    "--quiet".into(),
                    "--db".into(),
                    db.display().to_string(),
                ],
                true,
            );
            let n = open_db(&DbFlags {
                db: db.clone(),
                passphrase: None,
            })
            .ok()
            .and_then(|(led, tmp)| {
                let c = led.count().ok();
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                c
            })
            .unwrap_or(0);
            record(
                "onnx_real_photo",
                r.is_ok() && n >= 1,
                &format!("onnx process={r:?} count={n}"),
            );
        } else {
            record("onnx_real_photo", false, "image fixture missing");
        }
    }

    // --- Blind spots still unmeasured ---------------------------------------
    let mut blinds: Vec<(&str, &str)> = Vec::new();
    if !onnx_ready {
        blinds.push((
            "onnx_real_photo",
            "true RapidOCR on phone photos — needs --features onnx + models",
        ));
    }
    blinds.push((
        "watch_daemon_crash",
        "in-loop watch crash mid-process (restart-between-runs is measured)",
    ));
    blinds.push((
        "mobile_frb",
        "Flutter/FRB daily path out of CLI measure scope",
    ));

    if !quiet && !json {
        println!();
        println!("blind spots (UNMEASURED — do not trust yet):");
        for (id, why) in &blinds {
            println!("  BLIND | {id} | {why}");
        }
        println!();
    }

    // Restore env
    match prev_home {
        Some(v) => env::set_var("RRADAR_HOME", v),
        None => env::remove_var("RRADAR_HOME"),
    }
    match prev_db {
        Some(v) => env::set_var("RRADAR_DB", v),
        None => env::remove_var("RRADAR_DB"),
    }
    match prev_inbox {
        Some(v) => env::set_var("RRADAR_INBOX", v),
        None => env::remove_var("RRADAR_INBOX"),
    }

    let pass = probes
        .iter()
        .filter(|p| p["ok"].as_bool() == Some(true))
        .count();
    let report = serde_json::json!({
        "product_id": PRODUCT_ID,
        "version": VERSION,
        "sandbox": home.display().to_string(),
        "probes": probes,
        "pass": pass,
        "fail": failed,
        "blind_spots": blinds.iter().map(|(id, why)| serde_json::json!({"id": id, "why": why})).collect::<Vec<_>>(),
        "trust_policy": "only PASS probes are trusted; BLIND = unknown",
    });
    let report_path = home.join("measure-report.json");
    let _ = std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into()),
    );

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    }

    if failed == 0 {
        println!(
            "MEASURE_OK pass={pass} fail=0 blind={} report={}",
            blinds.len(),
            report_path.display()
        );
        Ok(())
    } else {
        println!(
            "MEASURE_FAIL pass={pass} fail={failed} blind={} report={}",
            blinds.len(),
            report_path.display()
        );
        Err(format!(
            "measure failed ({failed} probe(s)); do not trust failing behaviors yet"
        ))
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
    let cfg = AppConfig::default();
    if !AppConfig::path().is_file() {
        cfg.save().map_err(|e| e.to_string())?;
    }
    let mut aliases = AliasBook::load();
    if aliases.ensure_tw_defaults() {
        aliases.save().map_err(|e| e.to_string())?;
        println!(
            "aliases: seeded TW short names → {}",
            AliasBook::path().display()
        );
    }
    println!("initialized");
    println!("  home:   {}", dir.display());
    println!("  db:     {}", db.display());
    println!("  config: {}", AppConfig::path().display());
    println!("next: rradar add <receipt.txt|image> [--as-today]   # or: process … --confirm");
    println!("      rradar today                     # month + budget glance");
    println!("      rradar month                     # month-end close");
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
                let trash = l.count_trash().unwrap_or(0);
                println!(
                    "  ledger:   ok (schema {ver}, {} active, {trash} trash)",
                    l.count().unwrap_or(-1)
                );
                match l.integrity_check() {
                    Ok(i) => println!(
                        "  integrity: pragma={} schema={} active={} trash={}",
                        if i.pragma_ok { "ok" } else { &i.pragma_message },
                        i.schema_version,
                        i.active_count,
                        i.trash_count
                    ),
                    Err(e) => println!("  integrity: error ({e})"),
                }
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
    let ready = rradar_ocr::probe_onnx_readiness(&models);
    println!(
        "  engines:  mock (default), onnx, auto→{}",
        rradar_ocr::resolve_auto_engine_name()
    );
    println!(
        "  onnx:     ready={} feature={} models={} ort={} — {}",
        ready.ready_for_inference,
        ready.feature_enabled,
        ready.models_present,
        ready.ort_found,
        ready.hint
    );
    println!("  privacy:  local-first; no network required for core path");
    let book = BudgetBook::load();
    println!(
        "  budgets:  {} ({} line(s)) — rradar budget status",
        BudgetBook::path().display(),
        book.lines.len()
    );
    println!("  engines:  rradar engines [--json]");
    println!("  demo:     rradar demo   # isolated closed-loop from fixtures/");
    println!("  day:      rradar day    # 30s Taiwan daily path (add --as-today → today)");
    println!("  showcase: docs/demo-showcase.md");
    Ok(())
}

/// 30-second Taiwan daily path: curated fixtures → --as-today → today glance.
/// Isolated day ledger by default (does not touch the personal ledger unless --db set).
fn cmd_day(args: &[String]) -> Result<(), String> {
    let mut fixtures_root: Option<PathBuf> = None;
    let mut db_override: Option<PathBuf> = None;
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
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => {
                print_topic_help("day")?;
                return Ok(());
            }
            other => {
                return Err(format!(
                    "unknown day flag `{other}` — try `rradar help day`"
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

    let _ = ensure_data_dir().map_err(|e| e.to_string())?;
    let mut aliases = AliasBook::load();
    if aliases.ensure_tw_defaults() {
        aliases.save().map_err(|e| e.to_string())?;
    }
    let mut budgets = BudgetBook::load();
    if budgets.lines.is_empty() {
        budgets.set_major("TWD", "30000", None)?;
        budgets.save().map_err(|e| e.to_string())?;
    }

    let day_db = if let Some(p) = db_override {
        p
    } else if env::var_os("RRADAR_DB").is_some() {
        default_db_path()
    } else {
        let home = data_dir().join("day");
        let _ = std::fs::create_dir_all(&home);
        home.join("ledger.db")
    };
    if let Some(parent) = day_db.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if day_db.is_file() {
        let _ = std::fs::remove_file(&day_db);
    }
    let day_att = attachments_root_for_db(&day_db);
    if day_att.is_dir() {
        let _ = std::fs::remove_dir_all(&day_att);
    }
    let _ = rradar_core::Ledger::open(&day_db).map_err(|e| e.to_string())?;

    let curated = [
        "text/familymart_89.txt",
        "text/maymorning_breakfast_tw.txt",
        "text/bubbletea_50lan_tw.txt",
        "text/mrt_taipei.txt",
        "text/cht_bill_tw.txt",
    ];
    if !quiet {
        println!("══════════════════════════════════════════════");
        println!(" ReceiptRadar day — Taiwan daily path");
        println!(" add --as-today → today  |  No cloud.");
        println!("══════════════════════════════════════════════");
        println!("fixtures | {}", fixtures.display());
        println!("day db   | {}", day_db.display());
        println!();
    }

    let mut confirmed = 0usize;
    for rel in curated {
        let path = fixtures.join(rel);
        if !path.is_file() {
            return Err(format!("missing day fixture: {}", path.display()));
        }
        if !quiet {
            println!("── add --as-today | {rel} ──");
        }
        let mut proc_args = vec![
            path.display().to_string(),
            "--as-today".into(),
            "--db".into(),
            day_db.display().to_string(),
        ];
        if quiet {
            proc_args.push("--quiet".into());
        }
        cmd_process(&proc_args, true)?;
        confirmed += 1;
    }

    if !quiet {
        println!();
        println!("── today ──");
        cmd_today(&["--db".into(), day_db.display().to_string()])?;
    } else {
        // Quiet path: still open ledger to prove rows exist without printing the table.
        let (ledger, tmp) = open_db(&DbFlags {
            db: day_db.clone(),
            passphrase: None,
        })?;
        let n = ledger.count().map_err(|e| e.to_string())?;
        if let Some(t) = tmp {
            let _ = std::fs::remove_file(t);
        }
        if n == 0 {
            return Err("day quiet: ledger empty after curated adds".into());
        }
    }

    if quiet {
        println!("DAY_OK n={confirmed}");
    } else {
        println!();
        println!("DAY_OK — daily path finished ({confirmed} receipts, UTC today).");
        println!("Next: rradar today --db {}", day_db.display());
        println!("      rradar report --db {}", day_db.display());
        println!("Record tip: powershell -File scripts/day.ps1");
    }
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
    // Drop previous demo attachments so backup counts stay deterministic.
    let demo_att = attachments_root_for_db(&demo_db);
    if demo_att.is_dir() {
        let _ = std::fs::remove_dir_all(&demo_att);
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

    step(
        3,
        "synthetic photo + .ocr.txt sidecar (CI-safe; ONNX-capable pixels)",
    );
    let mut sidecar_tx_id: Option<String> = None;
    let mut img_paths = collect_glob(&fixtures.join("images"), &["png"]);
    img_paths.sort();
    // Prefer sidecar-backed photos (skip pure ONNX-only images without .ocr.txt).
    img_paths.retain(|p| {
        let side = PathBuf::from(format!("{}.ocr.txt", p.display()));
        side.is_file()
    });
    if img_paths.is_empty() {
        if !quiet {
            println!("  (skip — no image+sidecar fixtures)");
        }
    } else {
        for img_sidecar in &img_paths {
            let currency = currency_hint_for_path(img_sidecar);
            let draft = process_path(
                img_sidecar,
                eng.as_ref(),
                &categories,
                ProcessOptions {
                    default_currency: currency,
                    ..Default::default()
                },
            )
            .map_err(|e| format!("{}: {e}", img_sidecar.display()))?;
            let hash = rradar_core::content_hash(&std::fs::read(img_sidecar).unwrap_or_default());
            let res = ledger
                .confirm_draft(&draft, Some(&hash), Some("demo image sidecar"), false)
                .map_err(|e| e.to_string())?;
            if res.inserted {
                confirmed += 1;
                if sidecar_tx_id.is_none() {
                    sidecar_tx_id = Some(res.transaction.id.clone());
                }
            }
            if !quiet {
                println!(
                    "  ✓ {:<28}  {} {} (synthetic PNG + sidecar)",
                    img_sidecar
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?"),
                    draft.total.value.currency,
                    draft.merchant.value
                );
            }
        }
    }
    // Attach first sidecar image as receipt blob when available.
    let img_sidecar = img_paths.first().cloned().unwrap_or_default();

    step(4, "attach receipt blob + tags (schema v3 local store)");
    if let Some(ref tid) = sidecar_tx_id {
        if img_sidecar.is_file() {
            let rel =
                store_attachment(ledger.path(), tid, &img_sidecar).map_err(|e| e.to_string())?;
            let tx = ledger
                .update_transaction(
                    tid,
                    &TxUpdate {
                        attachment_path: Some(rel.clone()),
                        tags: Some("demo,receipt,sidecar".into()),
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?;
            if !quiet {
                println!("  ✓ attach | {rel}");
                println!("  ✓ tags   | {}", tx.tags.as_deref().unwrap_or("(none)"));
                println!(
                    "  ✓ store  | {}",
                    resolve_attachment_path(ledger.path(), &rel).display()
                );
            }
        }
    } else if !quiet {
        println!("  (skip — no image sidecar transaction)");
    }

    step(5, "TW e-invoice QR prefer path");
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

    step(6, "browse ledger");
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

    // Tag filter + local soft budgets (axis #2 product surface).
    let tag_hits = ledger
        .query_transactions(&TxFilter {
            limit: 20,
            tag: Some("demo".into()),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    let mut demo_budgets = BudgetBook::default();
    demo_budgets
        .set_major("TWD", "50", None)
        .map_err(|e| e.to_string())?;
    let budget_st =
        budget_status_month(&ledger, &demo_budgets, 2024, 5).map_err(|e| e.to_string())?;
    if !quiet {
        println!("── tags + budget (local soft limits) ──");
        println!("  tags list | {:?}", ledger.list_tags().unwrap_or_default());
        println!("  filter --tag demo | {} hit(s)", tag_hits.len());
        for s in &budget_st {
            let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
            println!(
                "  budget {} | spent={} limit={} {} ({:.0}%)",
                s.currency,
                Money::new(s.spent_minor, iso).display_major(),
                Money::new(s.limit_minor, iso).display_major(),
                if s.over { "OVER" } else { "ok" },
                s.ratio * 100.0
            );
        }
        println!("  hint | rradar budget set --currency TWD --monthly 30000");
        println!("  hint | rradar list --tag demo --min-amount 10");
    }

    step(7, "stats + top merchants");
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

    step(8, "export CSV + JSON");
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
    std::fs::write(&csv_path, &csv).map_err(|e| e.to_string())?;
    std::fs::write(&json_path, &json).map_err(|e| e.to_string())?;
    if !quiet {
        println!("  csv  | {}", csv_path.display());
        println!("  json | {}", json_path.display());
    }

    step(9, "multi-device file import (CSV + backup merge; no cloud)");
    // Round-trip: export CSV → empty ledger import; backup → second empty ledger merge.
    let import_db = out_dir.join("demo-import.db");
    if import_db.is_file() {
        let _ = std::fs::remove_file(&import_db);
    }
    let import_ledger = rradar_core::Ledger::open(&import_db).map_err(|e| e.to_string())?;
    let csv_rows = transactions_from_csv(&csv).map_err(|e| e.to_string())?;
    let (csv_ins, csv_skip) = import_ledger
        .import_transactions(&csv_rows)
        .map_err(|e| e.to_string())?;
    if !quiet {
        println!(
            "  ✓ import csv → {}  inserted={csv_ins} skipped={csv_skip} (local-only)",
            import_db.display()
        );
    }

    let bak = out_dir.join("demo-backup.rradar");
    if !skip_backup {
        let bytes = create_backup(
            &ledger,
            "demo-passphrase",
            8, /* fast Argon2 for demo */
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(&bak, &bytes).map_err(|e| e.to_string())?;
        if !quiet {
            match inspect_backup("demo-passphrase", &bytes) {
                Ok(info) => println!(
                    "  backup | {}  attachments={}  (passphrase: demo-passphrase)",
                    bak.display(),
                    info.attachment_file_count
                ),
                Err(_) => println!(
                    "  backup | {}  (passphrase: demo-passphrase)",
                    bak.display()
                ),
            }
        }
        let merge_db = out_dir.join("demo-merge.db");
        if merge_db.is_file() {
            let _ = std::fs::remove_file(&merge_db);
        }
        let merge_ledger = rradar_core::Ledger::open(&merge_db).map_err(|e| e.to_string())?;
        let restored = restore_backup("demo-passphrase", &bytes).map_err(|e| e.to_string())?;
        let bak_rows = transactions_from_backup(&restored).map_err(|e| e.to_string())?;
        let (m_ins, m_skip) = merge_ledger
            .import_transactions(&bak_rows)
            .map_err(|e| e.to_string())?;
        let att_n = write_restored_attachments(merge_ledger.path(), &restored)
            .map_err(|e| e.to_string())?;
        if !quiet {
            println!(
                "  ✓ backup merge → {}  inserted={m_ins} skipped={m_skip} attachments={att_n}",
                merge_db.display()
            );
            println!("  policy | multi-device = encrypted file you copy — no official relay");
        }
    } else if !quiet {
        println!("  (skip backup merge — --no-backup)");
    }

    step(10, "monthly markdown report (+ budgets section)");
    // Pick a month that appears in fixtures (2024-05 family mart).
    let md = monthly_markdown_with_budgets(&ledger, 2024, 5, &demo_budgets)
        .map_err(|e| e.to_string())?;
    let report_path = out_dir.join("demo-report-2024-05.md");
    std::fs::write(&report_path, &md).map_err(|e| e.to_string())?;
    if !quiet {
        println!("  report | {}", report_path.display());
        for line in md.lines().take(8) {
            println!("  | {line}");
        }
    }

    step(11, "annual report + merchant alias (local)");
    // Seed a display alias for demo narrative (does not rewrite ledger unless apply).
    let mut demo_aliases = AliasBook::default();
    demo_aliases.set("FAMILYMART", "全家");
    demo_aliases.set("全家", "全家");
    let annual =
        rradar_core::annual_markdown_with_books(&ledger, 2024, &demo_budgets, &demo_aliases)
            .map_err(|e| e.to_string())?;
    let annual_path = out_dir.join("demo-report-2024-annual.md");
    std::fs::write(&annual_path, &annual).map_err(|e| e.to_string())?;
    if !quiet {
        println!("  annual | {}", annual_path.display());
        println!("  alias  | FAMILYMART → 全家 (report display; rradar aliases apply to rewrite)");
        for line in annual.lines().take(8) {
            println!("  | {line}");
        }
    }

    step(12, "ONNX model pin status (weights optional)");
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

    step(13, "local HTTP API smoke (loopback only)");
    let api_fixture = fixtures.join("text/familymart_89.txt");
    // Use a dedicated smoke ledger so demo count stays stable for DEMO_OK messaging.
    let api_db = out_dir.join("demo-api-smoke.db");
    if api_db.is_file() {
        let _ = std::fs::remove_file(&api_db);
    }
    let api_att = attachments_root_for_db(&api_db);
    if api_att.is_dir() {
        let _ = std::fs::remove_dir_all(&api_att);
    }
    let _ = rradar_core::Ledger::open(&api_db).map_err(|e| e.to_string())?;
    match serve::smoke_local_api(&api_db, &api_fixture, None) {
        Ok(rep) => {
            if !quiet {
                println!(
                    "  ✓ bind={} health={} caps={} process={} attach={} txs={} stats={}",
                    rep.bind,
                    rep.health_ok,
                    rep.capabilities_ok,
                    rep.process_confirmed,
                    rep.attachment_set,
                    rep.transactions_n,
                    rep.stats_ok
                );
            }
        }
        Err(e) => return Err(format!("demo local API smoke failed: {e}")),
    }

    if !quiet {
        println!();
        println!("DEMO_OK — closed loop finished ({n} rows in demo ledger).");
        println!("Next: rradar list --db {}", demo_db.display());
        println!(
            "      rradar report --year 2024 --month 5 --db {}",
            demo_db.display()
        );
        println!("      rradar inbox --ensure && rradar watch --once --attach");
        println!(
            "      rradar serve --db {}   # loopback HTTP only",
            demo_db.display()
        );
        println!("      rradar api-smoke --fixtures fixtures");
        println!("      powershell -File scripts/smoke-onnx.ps1  # optional real ONNX e2e");
        println!("      cargo run -p gen-receipt-png -- fixtures/images  # regen synthetic PNGs");
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

/// List / verify the recordable fixture matrix from fixtures/manifest.json.
fn cmd_fixtures(args: &[String]) -> Result<(), String> {
    let mut verify = false;
    let mut json = false;
    let mut root: Option<PathBuf> = None;
    let mut engine = "mock".to_string();
    let mut include_onnx_smoke = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "verify" | "--verify" => verify = true,
            "list" => {}
            "--json" => json = true,
            "--onnx-smoke" => include_onnx_smoke = true,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("--engine needs mock|onnx|auto")?.clone();
            }
            "--fixtures" | "--root" => {
                i += 1;
                root = Some(PathBuf::from(args.get(i).ok_or("needs path")?));
            }
            "--help" | "-h" => {
                println!(
                    "rradar fixtures [list|verify] [--fixtures DIR] [--json]\n  \
                     [--engine mock|onnx|auto] [--onnx-smoke]\n  \
                     Index the demo matrix (text / mock_ocr / image sidecar / qr).\n  \
                     verify: process each entry and check expect_total_minor.\n  \
                     --engine onnx: force_ocr on images; requires --features onnx + models.\n  \
                     --onnx-smoke: also verify onnx_smoke_images (pixel path)."
                );
                return Ok(());
            }
            other if !other.starts_with('-') => {
                return Err(format!("unknown fixtures subcommand `{other}`"));
            }
            other => return Err(format!("unknown flag `{other}`")),
        }
        i += 1;
    }

    let fixtures = root.unwrap_or_else(find_fixtures_dir);
    let man_path = fixtures.join("manifest.json");
    if !man_path.is_file() {
        return Err(format!(
            "manifest not found at {} — run from repo root or pass --fixtures",
            man_path.display()
        ));
    }
    let raw = std::fs::read_to_string(&man_path).map_err(|e| e.to_string())?;
    let man: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    #[derive(Clone)]
    struct Row {
        kind: String,
        path: String,
        expect_minor: Option<i64>,
        currency: String,
        demo: bool,
    }
    let mut rows: Vec<Row> = Vec::new();
    let take = |kind: &str, arr: Option<&Vec<serde_json::Value>>| {
        let mut out = Vec::new();
        if let Some(a) = arr {
            for v in a {
                out.push(Row {
                    kind: kind.into(),
                    path: v
                        .get("path")
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_string(),
                    expect_minor: v.get("expect_total_minor").and_then(|x| x.as_i64()),
                    currency: v
                        .get("expect_currency")
                        .and_then(|c| c.as_str())
                        .unwrap_or("TWD")
                        .to_string(),
                    demo: v.get("demo").and_then(|d| d.as_bool()).unwrap_or(false),
                });
            }
        }
        out
    };
    rows.extend(take(
        "text",
        man.get("text_fixtures").and_then(|v| v.as_array()),
    ));
    rows.extend(take(
        "mock_ocr",
        man.get("mock_ocr_fixtures").and_then(|v| v.as_array()),
    ));
    rows.extend(take(
        "image_sidecar",
        man.get("image_sidecar_fixtures").and_then(|v| v.as_array()),
    ));
    if let Some(qr) = man.get("qr_fixtures").and_then(|v| v.as_array()) {
        for v in qr {
            rows.push(Row {
                kind: "qr".into(),
                path: v
                    .get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string(),
                expect_minor: None,
                currency: "TWD".into(),
                demo: v.get("demo").and_then(|d| d.as_bool()).unwrap_or(false),
            });
        }
    }
    // Optional ONNX pixel matrix (synthetic PNGs); list always, verify when --onnx-smoke or --engine onnx.
    let want_onnx_rows = include_onnx_smoke
        || engine.eq_ignore_ascii_case("onnx")
        || (engine.eq_ignore_ascii_case("auto")
            && rradar_ocr::probe_onnx_readiness(rradar_ocr::default_models_dir())
                .ready_for_inference);
    if want_onnx_rows || !verify {
        rows.extend(take(
            "onnx_smoke",
            man.get("onnx_smoke_images").and_then(|v| v.as_array()),
        ));
    }

    if json && !verify {
        println!(
            "{}",
            serde_json::json!({
                "root": fixtures.display().to_string(),
                "count": rows.len(),
                "engine": engine,
                "entries": rows.iter().map(|r| serde_json::json!({
                    "kind": r.kind,
                    "path": r.path,
                    "expect_total_minor": r.expect_minor,
                    "currency": r.currency,
                    "demo": r.demo,
                })).collect::<Vec<_>>()
            })
        );
        return Ok(());
    }

    if !verify {
        println!(
            "rradar fixtures | root={} | n={}",
            fixtures.display(),
            rows.len()
        );
        println!("kind          | demo | expect      | path");
        for r in &rows {
            let exp = r
                .expect_minor
                .map(|m| format!("{:>8} {}", m, r.currency))
                .unwrap_or_else(|| "         -".into());
            println!(
                "{:<13} | {:<4} | {} | {}",
                r.kind,
                if r.demo { "yes" } else { "no" },
                exp,
                r.path
            );
        }
        println!("hint | rradar fixtures verify                 # mock process + totals");
        println!("hint | rradar fixtures verify --engine onnx --onnx-smoke");
        println!("hint | rradar demo                            # full closed-loop");
        return Ok(());
    }

    // verify
    if engine.eq_ignore_ascii_case("onnx") {
        let ready = rradar_ocr::probe_onnx_readiness(rradar_ocr::default_models_dir());
        if !ready.ready_for_inference {
            return Err(format!(
                "onnx not ready for fixtures verify — {} (models/README.md)",
                ready.hint
            ));
        }
    }
    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let cats = category_engine_with_packs();
    let mut ok_n = 0usize;
    let mut fail_n = 0usize;
    let mut skip_n = 0usize;
    let force_pixel = eng.name().contains("onnx");
    println!(
        "rradar fixtures verify | root={} | engine={} ({}) force_ocr={}",
        fixtures.display(),
        engine,
        eng.name(),
        force_pixel
    );
    for r in &rows {
        if r.kind == "qr" {
            let p = fixtures.join(&r.path);
            if p.is_file() {
                ok_n += 1;
                println!("  OK   | qr payload present | {}", r.path);
            } else {
                fail_n += 1;
                println!("  FAIL | qr missing | {}", r.path);
            }
            continue;
        }
        // Skip text/mock under pure onnx pixel matrix mode when --onnx-smoke alone? No: full matrix still useful.
        // Skip onnx_smoke rows when engine is mock (sidecar-less totals would use broken OCR).
        if r.kind == "onnx_smoke" && !force_pixel {
            skip_n += 1;
            println!(
                "  skip | onnx_smoke needs --engine onnx|auto(ready) | {}",
                r.path
            );
            continue;
        }
        let Some(expect) = r.expect_minor else {
            skip_n += 1;
            println!("  skip | no expect | {}", r.path);
            continue;
        };
        let path = fixtures.join(&r.path);
        if !path.is_file() {
            fail_n += 1;
            println!("  FAIL | missing file | {}", r.path);
            continue;
        }
        let currency = Iso4217::parse(&r.currency).unwrap_or(Iso4217::TWD);
        // Pixel path: force OCR so sidecars do not mask engine quality.
        let force_ocr = force_pixel && (r.kind == "image_sidecar" || r.kind == "onnx_smoke");
        match process_path(
            &path,
            eng.as_ref(),
            &cats,
            ProcessOptions {
                default_currency: currency,
                force_ocr,
                ..Default::default()
            },
        ) {
            Ok(d) => {
                if d.total.value.amount_minor == expect
                    && d.total.value.currency.to_string() == r.currency
                {
                    ok_n += 1;
                    println!("  OK   | {:>8} {} | {}", expect, r.currency, r.path);
                } else {
                    fail_n += 1;
                    println!(
                        "  FAIL | got {} {} want {} {} | {}",
                        d.total.value.amount_minor,
                        d.total.value.currency,
                        expect,
                        r.currency,
                        r.path
                    );
                }
            }
            Err(e) => {
                fail_n += 1;
                println!("  FAIL | {e} | {}", r.path);
            }
        }
    }
    println!("FIXTURES_VERIFY ok={ok_n} fail={fail_n} skip={skip_n}");
    if fail_n > 0 {
        return Err(format!("fixtures verify failed ({fail_n})"));
    }
    Ok(())
}

/// Locate a repo-root file when cwd is repo root or a crate dir.
fn find_repo_file(name: &str) -> Option<PathBuf> {
    [
        PathBuf::from(name),
        PathBuf::from("..").join(name),
        PathBuf::from("../..").join(name),
        PathBuf::from("../../..").join(name),
    ]
    .into_iter()
    .find(|c| c.is_file())
}

/// Print third-party notices + project license policy (release trust surface).
fn cmd_licenses(args: &[String]) -> Result<(), String> {
    let json = args.iter().any(|a| a == "--json");
    let notices_path = find_repo_file("THIRD_PARTY_NOTICES");
    let license_path = find_repo_file("LICENSE");
    if json {
        let notices = notices_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let body = serde_json::json!({
            "product_id": PRODUCT_ID,
            "version": VERSION,
            "project_license": "Apache-2.0",
            "license_path": license_path.as_ref().map(|p| p.display().to_string()),
            "notices_path": notices_path.as_ref().map(|p| p.display().to_string()),
            "notices_bytes": notices.len(),
            "cloud_sync": false,
            "official_relay": false,
            "supply_chain_docs": ["docs/SUPPLY-CHAIN.md", "docs/licenses-checklist.md", "THIRD_PARTY_NOTICES"],
            "gate": "python tools/supply-chain/check_deps.py",
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return Ok(());
    }
    println!("ReceiptRadar licenses / notices (local-first)");
    println!("  product  | {PRODUCT_ID} {VERSION}");
    println!("  license  | Apache-2.0");
    if let Some(p) = &license_path {
        println!("  LICENSE  | {}", p.display());
    } else {
        println!("  LICENSE  | (not found — run from repo / release package root)");
    }
    if let Some(p) = &notices_path {
        println!("  notices  | {}", p.display());
        println!("────────────────────────────────────────");
        let text = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
        print!("{text}");
        if !text.ends_with('\n') {
            println!();
        }
    } else {
        println!("  notices  | THIRD_PARTY_NOTICES missing");
        println!("  hint     | ship notices with release archives (see docs/SUPPLY-CHAIN.md)");
    }
    println!("────────────────────────────────────────");
    println!("  gate     | python tools/supply-chain/check_deps.py");
    println!("  docs     | docs/SUPPLY-CHAIN.md");
    Ok(())
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

/// Raw OCR line dump (debug real photos before L1 extract).
fn cmd_ocr(args: &[String]) -> Result<(), String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut engine = "auto".to_string();
    let mut json = false;
    let mut max_edge: u32 = 1280;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("--engine needs value")?.clone();
            }
            "--max-edge" => {
                i += 1;
                let v = args.get(i).ok_or("--max-edge needs value")?;
                max_edge = v.parse().map_err(|_| format!("bad --max-edge {v}"))?;
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            s => paths.push(PathBuf::from(s)),
        }
        i += 1;
    }
    if paths.is_empty() {
        return Err(
            "usage: rradar ocr <image…> [--engine mock|onnx|auto] [--max-edge 1280] [--json]"
                .into(),
        );
    }
    if engine.eq_ignore_ascii_case("auto") {
        eprintln!(
            "engine auto → {} ({})",
            rradar_ocr::resolve_auto_engine_name(),
            rradar_ocr::probe_onnx_readiness(rradar_ocr::default_models_dir()).hint
        );
    }
    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let mut all = Vec::new();
    for path in &paths {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let pre = rradar_core::preprocess::preprocess(
            &bytes,
            rradar_core::preprocess::PreprocessConfig { max_edge },
        );
        let t0 = std::time::Instant::now();
        let lines = eng
            .recognize(&pre.bytes)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let ms = t0.elapsed().as_millis();
        if json {
            all.push(serde_json::json!({
                "path": path.display().to_string(),
                "engine": eng.name(),
                "ms": ms,
                "decoded": pre.decoded,
                "resized": pre.resized,
                "max_edge": pre.max_edge,
                "original": [pre.original_width, pre.original_height],
                "output": [pre.output_width, pre.output_height],
                "lines": lines.iter().map(|l| serde_json::json!({
                    "text": l.text,
                    "confidence": l.confidence,
                })).collect::<Vec<_>>(),
            }));
        } else {
            println!(
                "=== {} | engine={} | {}ms | decoded={} resized={} max_edge={}",
                path.display(),
                eng.name(),
                ms,
                pre.decoded,
                pre.resized,
                pre.max_edge
            );
            if let (Some(ow), Some(oh), Some(nw), Some(nh)) = (
                pre.original_width,
                pre.original_height,
                pre.output_width,
                pre.output_height,
            ) {
                println!("  size {ow}x{oh} → {nw}x{nh}");
            }
            for (idx, l) in lines.iter().enumerate() {
                println!("  [{idx:02}] conf={:.3}  {}", l.confidence, l.text);
            }
        }
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&all).unwrap_or_else(|_| "[]".into())
        );
    }
    Ok(())
}

/// A04 latency / accuracy harness over a directory or file list (product path).
fn cmd_bench(args: &[String]) -> Result<(), String> {
    let mut root = PathBuf::from("fixtures/text");
    let mut engine = "mock".to_string();
    let mut json = false;
    let mut warmup = true;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--no-warmup" => warmup = false,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("--engine needs value")?.clone();
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            s => root = PathBuf::from(s),
        }
        i += 1;
    }

    let eng = engine_by_name(&engine).map_err(|e| e.to_string())?;
    let cats = category_engine_with_packs();
    let paths = collect_bench_inputs(&root);
    if paths.is_empty() {
        return Err(format!("no inputs under {}", root.display()));
    }

    // Force pixel OCR for image roots so .ocr.txt sidecars do not zero-out A04 timings.
    let bench_opts = ProcessOptions {
        force_ocr: true,
        ..Default::default()
    };

    // Warm shared engine so first cold-load does not dominate p50 for ONNX.
    if warmup {
        if let Some(p) = paths.first() {
            let _ = process_path(p, eng.as_ref(), &cats, bench_opts.clone());
        }
    }

    let mut rows = Vec::new();
    let mut times = Vec::new();
    for path in &paths {
        let t0 = std::time::Instant::now();
        let res = process_path(path, eng.as_ref(), &cats, bench_opts.clone());
        let ms = t0.elapsed().as_millis();
        match res {
            Ok(d) => {
                times.push(ms);
                rows.push(serde_json::json!({
                    "path": path.display().to_string(),
                    "engine": eng.name(),
                    "ok": true,
                    "ms": ms,
                    "total_minor": d.total.value.amount_minor,
                    "currency": d.total.value.currency.to_string(),
                    "merchant": d.merchant.value,
                    "overall_confidence": d.overall_confidence,
                    "source_path": format!("{:?}", d.source_path),
                    "error": null,
                }));
            }
            Err(e) => {
                rows.push(serde_json::json!({
                    "path": path.display().to_string(),
                    "engine": eng.name(),
                    "ok": false,
                    "ms": ms,
                    "total_minor": null,
                    "currency": null,
                    "merchant": null,
                    "overall_confidence": null,
                    "source_path": null,
                    "error": e.to_string(),
                }));
            }
        }
    }
    times.sort_unstable();
    let success = rows
        .iter()
        .filter(|r| r["ok"].as_bool() == Some(true))
        .count();
    let fail = rows.len() - success;
    let p50 = percentile_ms(&times, 50);
    let p95 = percentile_ms(&times, 95);
    let report = serde_json::json!({
        "note": "A04 harness via rradar bench — paste into docs/spike-ocr-size.md",
        "engine": eng.name(),
        "engine_arg": engine,
        "root": root.display().to_string(),
        "warmup": warmup,
        "success": success,
        "fail": fail,
        "p50_ms": p50,
        "p95_ms": p95,
        "rows": rows,
    });
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "engine={} arg={} success={} fail={} p50_ms={:?} p95_ms={:?} warmup={}",
            eng.name(),
            engine,
            success,
            fail,
            p50,
            p95,
            warmup
        );
        for r in &rows {
            println!(
                "  {:>5}ms  ok={}  conf={}  {}  {:?}",
                r["ms"].as_u64().unwrap_or(0),
                r["ok"],
                r["overall_confidence"],
                r["path"].as_str().unwrap_or("?"),
                r["total_minor"]
            );
        }
    }
    if fail > 0 && engine.eq_ignore_ascii_case("onnx") {
        eprintln!(
            "note: onnx fails may be blank placeholder images (sidecar-only fixtures) \
             or missing ORT/weights — see models/README.md"
        );
    }
    Ok(())
}

fn collect_bench_inputs(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            // Skip OCR sidecars and non-input noise.
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with(".ocr.txt") || name.ends_with(".expected.json") {
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(
                ext.as_str(),
                "txt" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "bin" | "mock"
            ) || name.contains("mock")
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn percentile_ms(sorted: &[u128], p: u8) -> Option<u128> {
    if sorted.is_empty() {
        return None;
    }
    let idx = ((p as usize) * (sorted.len() - 1)) / 100;
    Some(sorted[idx])
}

fn cmd_process(args: &[String], default_confirm: bool) -> Result<(), String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut explain = false;
    let mut engine = "mock".to_string();
    let mut qr: Option<String> = None;
    let mut json = false;
    let mut currency = default_currency_from_env();
    let mut confirm = default_confirm;
    let mut db: Option<PathBuf> = None;
    let mut passphrase: Option<String> = None;
    let mut force = false;
    let mut notes: Option<String> = None;
    let mut merchant: Option<String> = None;
    let mut amount_major: Option<String> = None;
    let mut category: Option<String> = None;
    let mut date: Option<String> = None;
    let mut quiet = false;
    let mut attach = false;
    let mut tags: Option<String> = None;
    let mut as_today = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--explain" => explain = true,
            "--json" => json = true,
            "--confirm" | "-c" => confirm = true,
            // Opt out of `add`'s default confirm (preview only).
            "--preview" | "--no-confirm" => confirm = false,
            "--as-today" => as_today = true,
            "--force" => force = true,
            "--quiet" | "-q" => quiet = true,
            "--attach" => attach = true,
            "--tags" => {
                i += 1;
                tags = Some(args.get(i).ok_or("--tags needs value")?.clone());
            }
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
        return Err(if default_confirm {
            "usage: rradar add <path> [more…] [--attach] [--tags a,b] [--explain] (writes ledger; --preview to parse only)".into()
        } else {
            "usage: rradar process <path> [more paths…] [--confirm] [--explain] [--amount 89]"
                .into()
        });
    }

    if engine.eq_ignore_ascii_case("auto") && !quiet {
        eprintln!(
            "engine auto → {} ({})",
            rradar_ocr::resolve_auto_engine_name(),
            rradar_ocr::probe_onnx_readiness(rradar_ocr::default_models_dir()).hint
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
        if as_today && edits.transacted_at.is_none() {
            edits.transacted_at = Some(utc_today_date());
        }
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
                // Optional: copy source file into local attachments store + tags.
                let mut patch = TxUpdate::default();
                if attach && path.is_file() {
                    match store_attachment(ledger.path(), &result.transaction.id, path) {
                        Ok(rel) => {
                            patch.attachment_path = Some(rel.clone());
                            if !quiet {
                                println!("attached  | {rel}");
                            }
                        }
                        Err(e) => eprintln!("attach warn | {e}"),
                    }
                }
                if let Some(ref t) = tags {
                    patch.tags = Some(normalize_tags(t).unwrap_or_default());
                }
                if patch.attachment_path.is_some() || patch.tags.is_some() {
                    let _ = ledger
                        .update_transaction(&result.transaction.id, &patch)
                        .map_err(|e| e.to_string())?;
                }
            } else {
                println!(
                    "skipped | {} | hard dedupe (use --force)",
                    result.transaction.id
                );
            }
            if json {
                // Re-fetch after optional attach/tags patch for accurate JSON.
                let out = ledger
                    .get_transaction(&result.transaction.id)
                    .map(|tx| {
                        serde_json::json!({
                            "transaction": tx,
                            "dedupe": result.dedupe,
                            "inserted": result.inserted,
                        })
                    })
                    .unwrap_or_else(|_| serde_json::to_value(&result).unwrap_or_default());
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            }
        }
    }

    if let Some((ledger, tmp)) = ledger_open.take() {
        if confirmed_n > 0 && !quiet && !json {
            print_confirm_glance(&ledger);
        }
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
    // rradar import csv path.csv
    // rradar import backup --in file.rradar -p PASS [--db PATH]
    if args.is_empty() {
        return Err(
            "usage: rradar import json|csv <file> | rradar import backup --in file.rradar -p PASS"
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
            println!("import json | inserted={ins} skipped={skip} (existing ids skipped)");
            maybe_reseal(&flags, &ledger, tmp)?;
            Ok(())
        }
        "csv" => {
            let path = args.get(1).ok_or("usage: rradar import csv <file.csv>")?;
            let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let rows = transactions_from_csv(&raw).map_err(|e| e.to_string())?;
            let flags = extract_db_from_all(args)?;
            let _ = ensure_data_dir();
            let (ledger, tmp) = open_db(&flags)?;
            let (ins, skip) = ledger
                .import_transactions(&rows)
                .map_err(|e| e.to_string())?;
            println!(
                "import csv | inserted={ins} skipped={skip} from={} rows (existing ids skipped; local-only)",
                rows.len()
            );
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
            let att_n =
                write_restored_attachments(ledger.path(), &restored).map_err(|e| e.to_string())?;
            let bud =
                write_restored_budgets(ledger.path(), &restored).map_err(|e| e.to_string())?;
            let ali =
                write_restored_aliases(ledger.path(), &restored).map_err(|e| e.to_string())?;
            println!(
                "import backup | inserted={ins} skipped={skip} attachments={att_n} budgets={bud} aliases={ali} (from {} txs; multi-device via backup only)",
                rows.len()
            );
            maybe_reseal(&flags, &ledger, tmp)?;
            Ok(())
        }
        other => Err(format!(
            "unknown import type `{other}` — try: import json | import csv | import backup"
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
    let mut tag: Option<String> = None;
    let mut category: Option<String> = None;
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let mut from: Option<String> = None;
    let mut to: Option<String> = None;
    let mut min_amount: Option<String> = None;
    let mut max_amount: Option<String> = None;
    let mut has_attachment: Option<bool> = None;
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
            "--tag" => {
                i += 1;
                tag = Some(args.get(i).ok_or("--tag needs value")?.clone());
            }
            "--category" => {
                i += 1;
                category = Some(args.get(i).ok_or("--category needs value")?.clone());
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
            "--from" => {
                i += 1;
                from = Some(args.get(i).ok_or("--from needs date")?.clone());
            }
            "--to" => {
                i += 1;
                to = Some(args.get(i).ok_or("--to needs date")?.clone());
            }
            "--min-amount" => {
                i += 1;
                min_amount = Some(args.get(i).ok_or("--min-amount needs value")?.clone());
            }
            "--max-amount" => {
                i += 1;
                max_amount = Some(args.get(i).ok_or("--max-amount needs value")?.clone());
            }
            "--has-attachment" => has_attachment = Some(true),
            "--no-attachment" => has_attachment = Some(false),
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;
    let ccy_for_amt = currency
        .as_deref()
        .and_then(Iso4217::parse)
        .unwrap_or_else(default_currency_from_env);

    let mut filter = TxFilter {
        limit,
        offset,
        currency,
        query,
        tag,
        category,
        year_month: match (year, month) {
            (Some(y), Some(m)) => Some(format!("{y:04}-{m:02}")),
            _ => None,
        },
        from,
        to,
        min_minor: None,
        max_minor: None,
        has_attachment,
        ..Default::default()
    };
    if let Some(ref s) = min_amount {
        filter.min_minor = Some(
            Money::from_major_str(s, ccy_for_amt)
                .map_err(|e| e.to_string())?
                .amount_minor,
        );
    }
    if let Some(ref s) = max_amount {
        filter.max_minor = Some(
            Money::from_major_str(s, ccy_for_amt)
                .map_err(|e| e.to_string())?
                .amount_minor,
        );
    }

    let (ledger, tmp) = open_db(&flags)?;
    let rows = ledger
        .query_transactions(&filter)
        .map_err(|e| e.to_string())?;
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

fn cmd_tags(args: &[String]) -> Result<(), String> {
    let json = args.iter().any(|a| a == "--json");
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let tags = ledger.list_tags().map_err(|e| e.to_string())?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&tags).unwrap_or_default()
        );
    } else if tags.is_empty() {
        println!("(no tags)");
    } else {
        for t in &tags {
            println!("{t}");
        }
        eprintln!("({} tags)", tags.len());
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_budget(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err(
            "usage: rradar budget list|status|set|clear|path [--year Y --month M] [--json]".into(),
        );
    }
    let sub = args[0].as_str();
    let rest = &args[1..];
    match sub {
        "path" => {
            println!("{}", BudgetBook::path().display());
            Ok(())
        }
        "list" | "show" => {
            let book = BudgetBook::load();
            let json = rest.iter().any(|a| a == "--json");
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&book).unwrap_or_else(|_| "{}".into())
                );
            } else if book.lines.is_empty() {
                println!(
                    "(no budgets) — set with: rradar budget set --currency TWD --monthly 30000"
                );
                println!("path | {}", BudgetBook::path().display());
            } else {
                for line in &book.lines {
                    let iso = Iso4217::parse(&line.currency).unwrap_or(Iso4217::TWD);
                    let major = Money::new(line.limit_minor, iso).display_major();
                    match &line.category {
                        None => println!("overall | {} | limit={}", line.currency, major),
                        Some(c) => {
                            println!("category | {} | {} | limit={}", line.currency, c, major)
                        }
                    }
                }
                println!("path | {}", BudgetBook::path().display());
            }
            Ok(())
        }
        "set" => {
            let mut currency = default_currency_from_env().to_string();
            let mut monthly: Option<String> = None;
            let mut category: Option<String> = None;
            let mut amount: Option<String> = None;
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--currency" | "-c" => {
                        i += 1;
                        currency = rest.get(i).ok_or("needs currency")?.clone();
                    }
                    "--monthly" | "--limit" => {
                        i += 1;
                        monthly = Some(rest.get(i).ok_or("needs amount")?.clone());
                    }
                    "--category" => {
                        i += 1;
                        category = Some(rest.get(i).ok_or("needs category")?.clone());
                    }
                    "--amount" => {
                        i += 1;
                        amount = Some(rest.get(i).ok_or("needs amount")?.clone());
                    }
                    _ => {}
                }
                i += 1;
            }
            let major = monthly
                .or(amount)
                .ok_or("usage: rradar budget set --currency TWD --monthly 30000 [--category ID]")?;
            let mut book = BudgetBook::load();
            book.set_major(&currency, &major, category.as_deref())?;
            book.save().map_err(|e| e.to_string())?;
            println!(
                "saved | {} | {} | {} | path={}",
                currency,
                category.as_deref().unwrap_or("overall"),
                major,
                BudgetBook::path().display()
            );
            Ok(())
        }
        "clear" => {
            let mut currency: Option<String> = None;
            let mut category: Option<String> = None;
            let mut all = false;
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--all" => all = true,
                    "--currency" | "-c" => {
                        i += 1;
                        currency = Some(rest.get(i).ok_or("needs currency")?.clone());
                    }
                    "--category" => {
                        i += 1;
                        category = Some(rest.get(i).ok_or("needs category")?.clone());
                    }
                    _ => {}
                }
                i += 1;
            }
            let mut book = BudgetBook::load();
            if all {
                book.clear_all();
                book.save().map_err(|e| e.to_string())?;
                println!("cleared | all budgets");
                return Ok(());
            }
            let ccy = currency
                .ok_or("usage: rradar budget clear --currency TWD [--category ID] | --all")?;
            if book.clear_line(&ccy, category.as_deref()) {
                book.save().map_err(|e| e.to_string())?;
                println!(
                    "cleared | {} | {}",
                    ccy,
                    category.as_deref().unwrap_or("overall")
                );
            } else {
                return Err("no matching budget line".into());
            }
            Ok(())
        }
        "status" | "check" => {
            let mut year: Option<i32> = None;
            let mut month: Option<u32> = None;
            let json = rest.iter().any(|a| a == "--json");
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--year" => {
                        i += 1;
                        year = Some(
                            rest.get(i)
                                .ok_or("needs year")?
                                .parse()
                                .map_err(|_| "bad year")?,
                        );
                    }
                    "--month" => {
                        i += 1;
                        month = Some(
                            rest.get(i)
                                .ok_or("needs month")?
                                .parse()
                                .map_err(|_| "bad month")?,
                        );
                    }
                    _ => {}
                }
                i += 1;
            }
            let (y, m) = match (year, month) {
                (Some(y), Some(m)) => (y, m),
                _ => current_year_month(),
            };
            let book = BudgetBook::load();
            if book.lines.is_empty() {
                if json {
                    println!("[]");
                } else {
                    println!("(no budgets configured)");
                    println!("hint | rradar budget set --currency TWD --monthly 30000");
                }
                return Ok(());
            }
            let flags = extract_db_from_all(rest)?;
            let (ledger, tmp) = open_db(&flags)?;
            let statuses = budget_status_month(&ledger, &book, y, m).map_err(|e| e.to_string())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&statuses).unwrap_or_else(|_| "[]".into())
                );
            } else {
                println!("period | {y:04}-{m:02}");
                for s in &statuses {
                    let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
                    let spent = Money::new(s.spent_minor, iso).display_major();
                    let limit = Money::new(s.limit_minor, iso).display_major();
                    let rem = Money::new(s.remaining_minor, iso).display_major();
                    let scope = s.category.as_deref().unwrap_or("overall");
                    let flag = if s.over { "OVER" } else { "ok" };
                    println!(
                        "{flag} | {} | {scope} | spent={spent} limit={limit} remaining={rem} ({:.0}%)",
                        s.currency,
                        s.ratio * 100.0
                    );
                }
            }
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            Ok(())
        }
        other => Err(format!(
            "unknown budget subcommand `{other}` — list|status|set|clear|path"
        )),
    }
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
    let trash = ledger.count_trash().map_err(|e| e.to_string())?;
    println!("count | {n} active | {trash} trash");
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
        .ok_or("usage: rradar delete <id> --yes  (soft-delete → trash; purge for hard)")?
        .clone();
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    let hard = args.iter().any(|a| a == "--purge" || a == "--hard");
    let json = args.iter().any(|a| a == "--json");
    if !yes {
        return Err("refusing to delete without --yes".into());
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    if hard {
        let purge_result = ledger.purge_transaction(&id).map_err(|e| e.to_string());
        drop(ledger);
        if let Some(path) = tmp {
            let _ = std::fs::remove_file(path);
        }
        let report = purge_result?;
        if !report.purged_any() {
            return Err(format!("not found: {id}"));
        }
        print_purge_report(&report, json, &format!("purged\t{id}"))?;
        return Ok(());
    } else {
        let ok = ledger
            .soft_delete_transaction(&id)
            .map_err(|e| e.to_string())?;
        if !ok {
            return Err(format!("not found or already trashed: {id}"));
        }
        println!("trashed\t{id}  (rradar restore {id} | rradar purge {id} --yes)");
    }
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

fn cmd_trash(args: &[String]) -> Result<(), String> {
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let limit = args
        .windows(2)
        .find(|w| w[0] == "--limit")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(50usize);
    let rows = ledger
        .query_transactions(&TxFilter {
            limit,
            trash_only: true,
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    let n = ledger.count_trash().map_err(|e| e.to_string())?;
    if args.iter().any(|a| a == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).unwrap_or_else(|_| "[]".into())
        );
    } else {
        println!("trash | {n} row(s)  (schema v4 soft-delete; local-only)");
        for t in &rows {
            println!(
                "  {} | {} | {} {} | deleted_at={}",
                t.id,
                t.merchant,
                t.currency,
                t.amount_minor,
                t.deleted_at.as_deref().unwrap_or("?")
            );
        }
        if rows.is_empty() {
            println!("  (empty)");
        }
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_restore(args: &[String]) -> Result<(), String> {
    let id = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or("usage: rradar restore <id>")?
        .clone();
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let ok = ledger.restore_transaction(&id).map_err(|e| e.to_string())?;
    if !ok {
        return Err(format!("not in trash: {id}"));
    }
    println!("restored\t{id}");
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

fn cmd_purge(args: &[String]) -> Result<(), String> {
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    if !yes {
        return Err("refusing to purge without --yes (hard delete)".into());
    }
    let all = args.iter().any(|a| a == "--all" || a == "--trash");
    let json = args.iter().any(|a| a == "--json");
    let id = if all {
        None
    } else {
        Some(
            args.iter()
                .find(|a| !a.starts_with('-') && *a != "purge")
                .ok_or("usage: rradar purge <id> --yes | rradar purge --all --yes")?
                .clone(),
        )
    };
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let purge_result = if all {
        ledger
            .purge_trash()
            .map_err(|e| e.to_string())
            .map(|report| (report, "purged trash".to_owned(), false))
    } else {
        let id = id.as_deref().expect("purge id was parsed above");
        ledger
            .purge_transaction(id)
            .map_err(|e| e.to_string())
            .map(|report| (report, format!("purged\t{id}"), true))
    };
    drop(ledger);
    if let Some(path) = tmp {
        let _ = std::fs::remove_file(path);
    }
    let (report, label, require_match) = purge_result?;
    if require_match && !report.purged_any() {
        let id = id.as_deref().expect("purge id was parsed above");
        return Err(format!("not found: {id}"));
    }
    print_purge_report(&report, json, &label)?;
    Ok(())
}

fn print_purge_report(report: &PurgeReport, json: bool, label: &str) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).map_err(|error| error.to_string())?
        );
    } else {
        println!(
            "{label} | {} transaction(s) | attachments deleted={} missing={} shared={} duplicates={} unsafe={} errors={} dirs_removed={}",
            report.purged_transactions,
            report.attachments.deleted.len(),
            report.attachments.already_missing.len(),
            report.attachments.shared_references_skipped.len(),
            report.attachments.duplicate_candidates_skipped.len(),
            report.attachments.unsafe_paths_skipped.len(),
            report.attachments.cleanup_errors.len(),
            report.attachments.empty_dirs_removed.len(),
        );
    }
    Ok(())
}

fn cmd_edit(args: &[String]) -> Result<(), String> {
    let id = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or(
            "usage: rradar edit <id> [--merchant M] [--amount X] [--currency C] [--category K] [--notes N] [--date YYYY-MM-DD] [--tags T] [--clear-tags]",
        )?
        .clone();
    let mut merchant = None;
    let mut amount = None;
    let mut currency = None;
    let mut category = None;
    let mut notes = None;
    let mut date = None;
    let mut tags: Option<String> = None;
    let mut clear_tags = false;
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
            "--tags" => {
                i += 1;
                tags = Some(args.get(i).ok_or("needs value")?.clone());
            }
            "--clear-tags" => clear_tags = true,
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
    let tags_patch = if clear_tags {
        Some(String::new())
    } else {
        tags.map(|t| normalize_tags(&t).unwrap_or_default())
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
                tags: tags_patch,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&tx).unwrap_or_default());
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

/// Copy a receipt file into the local attachments store and set `attachment_path`.
fn cmd_attach(args: &[String]) -> Result<(), String> {
    let mut id: Option<String> = None;
    let mut file: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--file" | "-f" => {
                i += 1;
                file = Some(PathBuf::from(args.get(i).ok_or("--file needs path")?));
            }
            "--help" | "-h" => {
                println!(
                    "rradar attach <id> <file> | attach <id> --file PATH\n  \
                     Copy receipt into {{db_dir}}/attachments/{{id}}/ and set attachment_path."
                );
                return Ok(());
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            s => {
                if id.is_none() {
                    id = Some(s.to_string());
                } else if file.is_none() {
                    file = Some(PathBuf::from(s));
                } else {
                    return Err("usage: rradar attach <id> <file>".into());
                }
            }
        }
        i += 1;
    }
    let id = id.ok_or("usage: rradar attach <id> <file>")?;
    let file = file.ok_or("usage: rradar attach <id> <file>")?;
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let _ = ledger.get_transaction(&id).map_err(|e| e.to_string())?;
    let rel = store_attachment(ledger.path(), &id, &file).map_err(|e| e.to_string())?;
    let tx = ledger
        .update_transaction(
            &id,
            &TxUpdate {
                attachment_path: Some(rel.clone()),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
    println!("attached | {rel}");
    println!("{}", serde_json::to_string_pretty(&tx).unwrap_or_default());
    maybe_reseal(&flags, &ledger, tmp)?;
    Ok(())
}

/// Clear attachment_path; optionally delete the stored file.
fn cmd_detach(args: &[String]) -> Result<(), String> {
    let mut id: Option<String> = None;
    let mut delete_file = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--delete-file" => delete_file = true,
            "--help" | "-h" => {
                println!(
                    "rradar detach <id> [--delete-file]\n  \
                     Clear attachment_path; with --delete-file remove local blob too."
                );
                return Ok(());
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            s => {
                if id.is_none() {
                    id = Some(s.to_string());
                } else {
                    return Err("usage: rradar detach <id> [--delete-file]".into());
                }
            }
        }
        i += 1;
    }
    let id = id.ok_or("usage: rradar detach <id> [--delete-file]")?;
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let existing = ledger.get_transaction(&id).map_err(|e| e.to_string())?;
    if delete_file {
        if let Some(ref stored) = existing.attachment_path {
            remove_stored_attachment(ledger.path(), stored).map_err(|e| e.to_string())?;
        }
    }
    let tx = ledger
        .update_transaction(
            &id,
            &TxUpdate {
                attachment_path: Some(String::new()),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
    println!("detached | {id}");
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
    } else if let (Some(y), None) = (year, month) {
        // Full calendar year (no month)
        println!("period | year {y:04}");
        let year_tot = ledger
            .stats_by_currency_year(y)
            .map_err(|e| e.to_string())?;
        let months = ledger
            .stats_by_currency_year_months(y)
            .map_err(|e| e.to_string())?;
        if year_tot.is_empty() {
            println!("(no transactions)");
        } else {
            for s in &year_tot {
                let major = Money::new(
                    s.total_minor,
                    Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD),
                )
                .display_major();
                println!(
                    "{} year | {} | count={} | minor={}",
                    s.currency, major, s.count, s.total_minor
                );
            }
            for s in &months {
                let major = Money::new(
                    s.total_minor,
                    Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD),
                )
                .display_major();
                println!(
                    "{} {:04}-{:02} | {} | count={} | minor={}",
                    s.currency, s.year, s.month, major, s.count, s.total_minor
                );
            }
            println!("note | currencies are never summed together");
        }
        if let Some(t) = tmp {
            let _ = std::fs::remove_file(t);
        }
        return Ok(());
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
    let mut annual = false;
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
            "--annual" | "--year-only" => annual = true,
            "-o" | "--output" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("needs path")?));
            }
            _ => {}
        }
        i += 1;
    }
    // Annual when --annual, or --year without --month.
    let want_annual = annual || (year.is_some() && month.is_none());
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let md = if want_annual {
        let y = year.unwrap_or_else(|| current_year_month().0);
        annual_markdown(&ledger, y).map_err(|e| e.to_string())?
    } else {
        let (y, m) = match (year, month) {
            (Some(y), Some(m)) => (y, m),
            _ => current_year_month(),
        };
        monthly_markdown(&ledger, y, m).map_err(|e| e.to_string())?
    };
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

/// Month-end glance: spend + budgets + categories + top merchants (+ optional markdown).
fn cmd_month(args: &[String]) -> Result<(), String> {
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let mut out: Option<PathBuf> = None;
    let mut csv_out: Option<PathBuf> = None;
    let mut json = false;
    let mut quiet = false;
    let mut currency = default_currency_from_env().to_string();
    let mut top_n: usize = 5;
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
            "--csv" => {
                i += 1;
                csv_out = Some(PathBuf::from(args.get(i).ok_or("--csv needs path")?));
            }
            "--quiet" | "-q" => quiet = true,
            "--json" => json = true,
            "--currency" => {
                i += 1;
                currency = args.get(i).ok_or("needs currency")?.clone();
            }
            "--top" => {
                i += 1;
                top_n = args
                    .get(i)
                    .ok_or("needs N")?
                    .parse()
                    .map_err(|_| "bad --top")?;
            }
            "--help" | "-h" => {
                print_topic_help("month")?;
                return Ok(());
            }
            "--db" | "--passphrase" | "-p" => {
                i += 1;
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            _ => {}
        }
        i += 1;
    }
    let (y, m) = match (year, month) {
        (Some(y), Some(m)) => (y, m),
        (Some(y), None) => (y, current_year_month().1),
        (None, Some(m)) => (current_year_month().0, m),
        _ => current_year_month(),
    };
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let stats = ledger
        .stats_by_currency_month(y, m)
        .map_err(|e| e.to_string())?;
    let book = BudgetBook::load();
    let budgets = if book.lines.is_empty() {
        Vec::new()
    } else {
        budget_status_month(&ledger, &book, y, m).map_err(|e| e.to_string())?
    };
    let ym = format!("{y:04}-{m:02}");
    let cats = ledger
        .stats_by_category(&currency, Some(&ym))
        .map_err(|e| e.to_string())?;
    let month_txs = ledger
        .list_by_month(y, m, 100_000)
        .map_err(|e| e.to_string())?;
    let mut merchant_totals: std::collections::BTreeMap<String, (i64, i64)> =
        std::collections::BTreeMap::new();
    for tx in &month_txs {
        if tx.currency != currency {
            continue;
        }
        let e = merchant_totals.entry(tx.merchant.clone()).or_insert((0, 0));
        e.0 += tx.amount_minor;
        e.1 += 1;
    }
    let mut top: Vec<(String, i64, i64)> = merchant_totals
        .into_iter()
        .map(|(k, (minor, cnt))| (k, minor, cnt))
        .collect();
    top.sort_by_key(|b| std::cmp::Reverse(b.1));
    top.truncate(top_n);

    let md = monthly_markdown_with_budgets(&ledger, y, m, &book).map_err(|e| e.to_string())?;
    if let Some(ref p) = out {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(p, md.as_bytes()).map_err(|e| e.to_string())?;
    }
    if let Some(ref p) = csv_out {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let csv = transactions_to_csv(&month_txs).map_err(|e| e.to_string())?;
        std::fs::write(p, csv.as_bytes()).map_err(|e| e.to_string())?;
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "period": ym,
                "stats": stats,
                "budgets": budgets,
                "categories": cats,
                "top_merchants": top.iter().map(|(m, minor, cnt)| serde_json::json!({
                    "merchant": m,
                    "amount_minor": minor,
                    "count": cnt,
                    "currency": currency,
                })).collect::<Vec<_>>(),
                "markdown_path": out.as_ref().map(|p| p.display().to_string()),
                "csv_path": csv_out.as_ref().map(|p| p.display().to_string()),
                "csv_rows": month_txs.len(),
            })
        );
    } else if quiet {
        println!("MONTH_OK | {ym}");
    } else {
        println!("month | {ym}");
        if stats.is_empty() {
            println!("spend | (none this month)");
        } else {
            for s in &stats {
                let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
                let total = Money::new(s.total_minor, iso).display_major();
                println!("spend | {} | total={total} | n={}", s.currency, s.count);
            }
        }
        if budgets.is_empty() {
            println!("budget | (none) — rradar budget set --currency TWD --monthly 30000");
        } else {
            for s in &budgets {
                let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
                let spent = Money::new(s.spent_minor, iso).display_major();
                let limit_s = Money::new(s.limit_minor, iso).display_major();
                let rem = Money::new(s.remaining_minor, iso).display_major();
                let scope = s.category.as_deref().unwrap_or("overall");
                let flag = if s.over { "OVER" } else { "ok" };
                println!(
                    "budget | {flag} | {} | {scope} | spent={spent} limit={limit_s} remaining={rem} ({:.0}%)",
                    s.currency,
                    s.ratio * 100.0
                );
            }
        }
        println!("categories | currency={currency}");
        if cats.is_empty() {
            println!("  (none)");
        } else {
            for c in &cats {
                let iso = Iso4217::parse(&c.currency).unwrap_or(Iso4217::TWD);
                let major = Money::new(c.total_minor, iso).display_major();
                println!("  {} | {major} | n={}", c.category, c.count);
            }
        }
        println!("top | currency={currency} | limit={top_n}");
        if top.is_empty() {
            println!("  (none)");
        } else {
            let iso = Iso4217::parse(&currency).unwrap_or(Iso4217::TWD);
            for (i, (merch, minor, cnt)) in top.iter().enumerate() {
                let major = Money::new(*minor, iso).display_major();
                let name = display_merchant_name(merch);
                println!("  {:>2} | {name} | {major} | n={cnt}", i + 1);
            }
        }
        if let Some(p) = &out {
            println!("wrote | md | {}", p.display());
        }
        if let Some(p) = &csv_out {
            println!("wrote | csv | {} | rows={}", p.display(), month_txs.len());
        }
        if csv_out.is_none() {
            println!("hint | rradar month --csv {ym}.csv");
        }
        println!("MONTH_OK | {ym}");
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

fn cmd_aliases(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err(
            "usage: rradar aliases list|set|rm|apply|path [--from X --to Y] [--rewrite]".into(),
        );
    }
    match args[0].as_str() {
        "path" => {
            println!("{}", AliasBook::path().display());
            Ok(())
        }
        "list" => {
            let book = AliasBook::load();
            if book.map.is_empty() {
                println!("(no aliases — rradar aliases set --from '全家便利商店' --to '全家')");
            } else {
                for (k, v) in &book.map {
                    println!("{k}  →  {v}");
                }
            }
            println!("path | {}", AliasBook::path().display());
            Ok(())
        }
        "set" => {
            let mut from = None;
            let mut to = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--from" => {
                        i += 1;
                        from = Some(args.get(i).ok_or("needs value")?.clone());
                    }
                    "--to" => {
                        i += 1;
                        to = Some(args.get(i).ok_or("needs value")?.clone());
                    }
                    _ => {}
                }
                i += 1;
            }
            let from = from.ok_or("--from required")?;
            let to = to.ok_or("--to required")?;
            let mut book = AliasBook::load();
            book.set(&from, &to);
            let _ = ensure_data_dir();
            book.save().map_err(|e| e.to_string())?;
            println!("alias | {from} → {to}");
            println!("path  | {}", AliasBook::path().display());
            Ok(())
        }
        "rm" | "remove" => {
            let mut from = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--from" => {
                        i += 1;
                        from = Some(args.get(i).ok_or("needs value")?.clone());
                    }
                    s if !s.starts_with('-') && from.is_none() => from = Some(s.to_string()),
                    _ => {}
                }
                i += 1;
            }
            let from = from.ok_or("usage: rradar aliases rm --from NAME")?;
            let mut book = AliasBook::load();
            if book.remove(&from) {
                book.save().map_err(|e| e.to_string())?;
                println!("removed | {from}");
            } else {
                println!("not found | {from}");
            }
            Ok(())
        }
        "apply" | "rewrite" => {
            // Rewrite ledger merchant fields using aliases (exact match).
            let flags = extract_db_from_all(args)?;
            let (ledger, tmp) = open_db(&flags)?;
            let book = AliasBook::load();
            let mut n = 0usize;
            for (from, to) in &book.map {
                n += ledger
                    .rewrite_merchant(from, to)
                    .map_err(|e| e.to_string())?;
            }
            println!("rewrote | {n} row(s)");
            maybe_reseal(&flags, &ledger, tmp)?;
            Ok(())
        }
        other => Err(format!(
            "unknown aliases subcommand `{other}` — try list|set|rm|apply|path"
        )),
    }
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
        println!("hint | drop receipt .txt/.jpg here then: rradar scoop");
    } else if !path.is_dir() {
        println!("hint | run: rradar inbox --ensure");
    } else {
        println!("hint | rradar scoop   # process inbox → today (as-today)");
    }
    Ok(())
}

/// One-shot daily capture: ensure inbox → confirm --as-today → archive → today glance.
fn cmd_scoop(args: &[String]) -> Result<(), String> {
    let mut quiet = false;
    let mut attach = false;
    let mut keep_date = false;
    let mut archive = true;
    let mut engine = "mock".to_string();
    let mut dir: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--quiet" | "-q" => quiet = true,
            "--attach" => attach = true,
            "--keep-date" => keep_date = true,
            "--no-archive" => archive = false,
            "--archive" => archive = true,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("--engine needs value")?.clone();
            }
            "--help" | "-h" => {
                print_topic_help("scoop")?;
                return Ok(());
            }
            "--db" | "--passphrase" | "-p" => {
                i += 1;
            }
            s if !s.starts_with('-') => dir = Some(PathBuf::from(s)),
            other => {
                return Err(format!(
                    "unknown scoop flag `{other}` — try `rradar help scoop`"
                ))
            }
        }
        i += 1;
    }

    let inbox = if let Some(d) = dir {
        if !d.is_dir() {
            std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        }
        d
    } else {
        ensure_inbox_dir().map_err(|e| e.to_string())?
    };

    let flags = extract_db_from_all(args)?;
    let _ = ensure_data_dir();
    let mut aliases = AliasBook::load();
    if aliases.ensure_tw_defaults() {
        let _ = aliases.save();
    }

    // Top-level files only (skip done/ and other subdirs).
    let mut files: Vec<PathBuf> = std::fs::read_dir(&inbox)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();

    if !quiet {
        println!("══════════════════════════════════════════════");
        println!(" ReceiptRadar scoop — inbox → today");
        println!(" No cloud. Drop files, one command.");
        println!("══════════════════════════════════════════════");
        println!("inbox   | {}", inbox.display());
        println!("db      | {}", flags.db.display());
        println!("files   | {}", files.len());
        println!(
            "archive | {}",
            if archive {
                "done/YYYY-MM-DD/ (default)"
            } else {
                "off (--no-archive)"
            }
        );
        println!();
    }

    if files.is_empty() {
        if quiet {
            println!("SCOOP_OK n=0");
        } else {
            println!("(inbox empty)");
            println!("hint | copy a receipt into {}", inbox.display());
            println!("SCOOP_OK n=0");
        }
        return Ok(());
    }

    let mut confirmed = 0usize;
    let mut archived = 0usize;
    // Claim directory: concurrent scoops race on the same inbox listing, and the
    // ledger dedupe is an advisory SELECT-then-INSERT, so two processes handling
    // the same file can both insert it. Renaming into `.scooping/` first is
    // atomic on one filesystem: exactly one scoop wins each file, the loser
    // skips it. The claimed file keeps its original name so `add` provenance
    // and the done/-archive name stay stable.
    let claim_dir = inbox.join(".scooping");
    let _ = std::fs::create_dir_all(&claim_dir);
    for path in &files {
        let claim_target = match path.file_name() {
            Some(name) => claim_dir.join(name),
            None => continue,
        };
        let use_path = match std::fs::rename(path, &claim_target) {
            Ok(()) => claim_target.clone(),
            Err(_) if !path.exists() => {
                // Another scoop claimed (or archived) it between listing and now.
                if !quiet {
                    println!(
                        "── scoop | {} ── claimed by another scoop; skipping",
                        path.display()
                    );
                }
                continue;
            }
            // Stale same-named claim is blocking the rename (e.g. a crashed
            // run); fall back to processing in place so the file is not stuck.
            Err(_) => path.clone(),
        };
        if !quiet {
            println!("── scoop | {} ──", path.display());
        }
        let mut proc_args = vec![use_path.display().to_string()];
        if !keep_date {
            proc_args.push("--as-today".into());
        }
        if attach {
            proc_args.push("--attach".into());
        }
        proc_args.push("--engine".into());
        proc_args.push(engine.clone());
        if let Some(db) = flags.db.to_str() {
            proc_args.push("--db".into());
            proc_args.push(db.to_string());
        }
        if let Some(ref pass) = flags.passphrase {
            proc_args.push("--passphrase".into());
            proc_args.push(pass.clone());
        }
        if quiet {
            proc_args.push("--quiet".into());
        }
        match cmd_process(&proc_args, true) {
            Ok(()) => {
                confirmed += 1;
                if archive {
                    match archive_inbox_file(&inbox, &use_path) {
                        Ok(dest) => {
                            archived += 1;
                            if !quiet {
                                println!("archived | {}", dest.display());
                            }
                        }
                        Err(e) => eprintln!("archive warn | {} | {e}", use_path.display()),
                    }
                } else if use_path != *path {
                    // --no-archive keeps files in the inbox; release the claim.
                    let _ = std::fs::rename(&use_path, path);
                }
            }
            Err(e) => {
                eprintln!("scoop | error: {e}");
                if use_path != *path {
                    // Return the file to the inbox so a later scoop can retry.
                    let _ = std::fs::rename(&use_path, path);
                }
            }
        }
    }
    // Best-effort: only removes the claim dir when every claim was resolved.
    let _ = std::fs::remove_dir(&claim_dir);

    if !quiet {
        println!();
        println!("── today ──");
        cmd_today(&["--db".into(), flags.db.display().to_string()])?;
        println!();
        println!(
            "SCOOP_OK — processed {confirmed}/{} inbox file(s); archived={archived}.",
            files.len()
        );
        if archive {
            println!(
                "hint | processed files live under {}/done/{}/",
                inbox.display(),
                utc_today_date()
            );
        }
    } else {
        println!("SCOOP_OK n={confirmed} archived={archived}");
    }
    Ok(())
}

/// Move a scooped file to `{inbox}/done/YYYY-MM-DD/{filename}` (collision-safe).
fn archive_inbox_file(inbox: &Path, src: &Path) -> Result<PathBuf, String> {
    let day = utc_today_date();
    let dest_dir = inbox.join("done").join(&day);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let name = src
        .file_name()
        .ok_or_else(|| format!("no file name: {}", src.display()))?;
    let mut dest = dest_dir.join(name);
    if dest.exists() {
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("receipt");
        let ext = src
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        for n in 2..1000 {
            let candidate = dest_dir.join(format!("{stem}-{n}{ext}"));
            if !candidate.exists() {
                dest = candidate;
                break;
            }
        }
    }
    match std::fs::rename(src, &dest) {
        Ok(()) => Ok(dest),
        Err(_) => {
            std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
            std::fs::remove_file(src).map_err(|e| e.to_string())?;
            Ok(dest)
        }
    }
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

/// Ephemeral loopback API smoke (health → process+attach → list → stats).
fn cmd_api_smoke(args: &[String]) -> Result<(), String> {
    let mut fixtures_root: Option<PathBuf> = None;
    let mut db_override: Option<PathBuf> = None;
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
            "--help" | "-h" => {
                println!(
                    "rradar api-smoke [--fixtures DIR] [--db PATH]\n  \
                     Spawns ephemeral 127.0.0.1 server; exercises product HTTP surface.\n  \
                     Local-only; no cloud. See docs/local-api.md."
                );
                return Ok(());
            }
            other => return Err(format!("unknown api-smoke flag `{other}`")),
        }
        i += 1;
    }
    let fixtures = fixtures_root
        .or_else(|| env::var_os("RRADAR_FIXTURES").map(PathBuf::from))
        .unwrap_or_else(find_fixtures_dir);
    let fixture = fixtures.join("text/familymart_89.txt");
    let db = db_override.unwrap_or_else(|| {
        let home = data_dir().join("api-smoke");
        let _ = std::fs::create_dir_all(&home);
        home.join("ledger.db")
    });
    if let Some(parent) = db.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if db.is_file() {
        let _ = std::fs::remove_file(&db);
    }
    let att = attachments_root_for_db(&db);
    if att.is_dir() {
        let _ = std::fs::remove_dir_all(&att);
    }
    let _ = rradar_core::Ledger::open(&db).map_err(|e| e.to_string())?;
    let rep = serve::smoke_local_api(&db, &fixture, None)?;
    println!("API_SMOKE_OK bind={}", rep.bind);
    println!(
        "  health={} capabilities={} process={} attach={} txs={} stats={}",
        rep.health_ok,
        rep.capabilities_ok,
        rep.process_confirmed,
        rep.attachment_set,
        rep.transactions_n,
        rep.stats_ok
    );
    println!("  db | {}", db.display());
    Ok(())
}

fn cmd_watch(args: &[String]) -> Result<(), String> {
    // rradar watch [dir] [--interval 2] [--once] [--attach] [--as-today|--keep-date]
    let mut dir: Option<PathBuf> = None;
    let mut interval_secs: u64 = 2;
    let mut confirm = true;
    let mut once = false;
    let mut attach = false;
    let mut as_today = true; // daily path default
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
            "--attach" => attach = true,
            "--as-today" => as_today = true,
            "--keep-date" => as_today = false,
            "--engine" => {
                i += 1;
                engine = args.get(i).ok_or("needs engine")?.clone();
            }
            // Parsed by extract_db_from_all; skip value here.
            "--db" | "--passphrase" | "-p" => {
                i += 1;
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
            "watch | {} | interval={interval_secs}s | as_today={} | seeded {} existing files",
            dir.display(),
            as_today,
            seen.len()
        );
    }
    let mut processed = 0usize;
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
                    if attach {
                        proc_args.push("--attach".into());
                    }
                    if as_today {
                        proc_args.push("--as-today".into());
                    }
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
                if confirm {
                    if let Some(ref pass) = flags.passphrase {
                        proc_args.push("--passphrase".into());
                        proc_args.push(pass.clone());
                    }
                }
                println!(
                    "watch | processing {}{}{}",
                    path.display(),
                    if attach { " (+attach)" } else { "" },
                    if as_today { " (+as-today)" } else { "" }
                );
                // call process logic by reconstructing argv
                if let Err(e) = cmd_process(&proc_args, false) {
                    eprintln!("watch | error: {e}");
                } else {
                    processed += 1;
                }
            }
        }
        if once {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(interval_secs));
    }
    if once && processed > 0 {
        println!("watch | done n={processed} — next: rradar today");
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
        return Err(
            "usage: rradar export <csv|json> [-o file] [--tag T] [--category C] [--year Y --month M]"
                .into(),
        );
    }
    let kind = args[0].as_str();
    let mut out: Option<PathBuf> = None;
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let mut tag: Option<String> = None;
    let mut category: Option<String> = None;
    let mut currency: Option<String> = None;
    let mut query: Option<String> = None;
    let mut from: Option<String> = None;
    let mut to: Option<String> = None;
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
            "--tag" => {
                i += 1;
                tag = Some(args.get(i).ok_or("needs tag")?.clone());
            }
            "--category" => {
                i += 1;
                category = Some(args.get(i).ok_or("needs category")?.clone());
            }
            "--currency" => {
                i += 1;
                currency = Some(args.get(i).ok_or("needs currency")?.clone());
            }
            "--query" | "-q" => {
                i += 1;
                query = Some(args.get(i).ok_or("needs query")?.clone());
            }
            "--from" => {
                i += 1;
                from = Some(args.get(i).ok_or("needs date")?.clone());
            }
            "--to" => {
                i += 1;
                to = Some(args.get(i).ok_or("needs date")?.clone());
            }
            _ => {}
        }
        i += 1;
    }
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let filtered = tag.is_some()
        || category.is_some()
        || currency.is_some()
        || query.is_some()
        || from.is_some()
        || to.is_some()
        || year.is_some()
        || month.is_some();
    let rows = if filtered {
        ledger
            .query_transactions(&TxFilter {
                limit: 100_000,
                offset: 0,
                currency,
                query,
                tag,
                category,
                year_month: match (year, month) {
                    (Some(y), Some(m)) => Some(format!("{y:04}-{m:02}")),
                    _ => None,
                },
                from,
                to,
                ..Default::default()
            })
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
                "  package_schema={}  ledger_schema={}  txs={}  attachments={}  app={}  created={}",
                info.manifest.schema_version,
                info.manifest.ledger_schema_version,
                info.manifest.transaction_count,
                info.manifest
                    .attachment_count
                    .max(info.attachment_file_count as u32),
                info.manifest.app_version,
                info.manifest.created_at
            );
            println!(
                "  has_sqlite={}  has_transactions_json={}  attachment_files={}",
                info.has_sqlite, info.has_transactions_json, info.attachment_file_count
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
                "backup verify | OK  txs={}  attachments={}  ledger_schema={}  app={}",
                m.transaction_count, m.attachment_count, m.ledger_schema_version, m.app_version
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
            std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
            let att_n = inspect_backup(&pass, &bytes)
                .map(|i| i.attachment_file_count)
                .unwrap_or(0);
            println!("backup\t{}\tattachments={att_n}", out.display());
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
                let att_n = write_restored_attachments(ledger.path(), &restored)
                    .map_err(|e| e.to_string())?;
                let bud =
                    write_restored_budgets(ledger.path(), &restored).map_err(|e| e.to_string())?;
                let ali =
                    write_restored_aliases(ledger.path(), &restored).map_err(|e| e.to_string())?;
                println!(
                    "restored(merge)\tinserted={ins}\tskipped={skip}\tattachments={att_n}\tbudgets={bud}\taliases={ali}\t-> {}",
                    db.display()
                );
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
            } else {
                let sqlite = restored
                    .sqlite_bytes
                    .as_ref()
                    .ok_or("backup missing ledger.sqlite")?;
                write_restored_db(&db, sqlite).map_err(|e| e.to_string())?;
                let att_n =
                    write_restored_attachments(&db, &restored).map_err(|e| e.to_string())?;
                let bud = write_restored_budgets(&db, &restored).map_err(|e| e.to_string())?;
                let ali = write_restored_aliases(&db, &restored).map_err(|e| e.to_string())?;
                // Open once so migrations apply if restoring older schema snapshot.
                let ledger = rradar_core::Ledger::open(&db).map_err(|e| e.to_string())?;
                println!(
                    "restored\t{} txs\tattachments={att_n}\tbudgets={bud}\taliases={ali}\tschema={}\t-> {}",
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

/// Short post-confirm glance: spend + budget for the month of the last row.
fn print_confirm_glance(ledger: &rradar_core::Ledger) {
    let (y, m) = ledger
        .last_transaction()
        .ok()
        .flatten()
        .and_then(|tx| {
            let d = tx.transacted_at.get(..7)?;
            let (ys, ms) = d.split_once('-')?;
            Some((ys.parse().ok()?, ms.parse().ok()?))
        })
        .unwrap_or_else(current_year_month);
    match ledger.stats_by_currency_month(y, m) {
        Ok(stats) if !stats.is_empty() => {
            for s in stats {
                let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
                let total = Money::new(s.total_minor, iso).display_major();
                println!(
                    "month | {y:04}-{m:02} | {} | spent={total} | n={}",
                    s.currency, s.count
                );
            }
        }
        Ok(_) => println!("month | {y:04}-{m:02} | (no spend yet)"),
        Err(e) => eprintln!("month warn | {e}"),
    }
    let book = BudgetBook::load();
    if book.lines.is_empty() {
        println!("budget | (none) — rradar budget set --currency TWD --monthly 30000");
    } else if let Ok(statuses) = budget_status_month(ledger, &book, y, m) {
        for s in statuses {
            let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
            let rem = Money::new(s.remaining_minor, iso).display_major();
            let scope = s.category.as_deref().unwrap_or("overall");
            let flag = if s.over { "OVER" } else { "ok" };
            println!(
                "budget | {flag} | {} | {scope} | remaining={rem} ({:.0}%)",
                s.currency,
                s.ratio * 100.0
            );
        }
    }
    println!("hint | rradar today");
}

/// Daily home screen: this month + budgets + recent rows.
fn cmd_today(args: &[String]) -> Result<(), String> {
    let mut limit: usize = 8;
    let mut year: Option<i32> = None;
    let mut month: Option<u32> = None;
    let json = args.iter().any(|a| a == "--json");
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                i += 1;
                limit = args
                    .get(i)
                    .ok_or("--limit needs N")?
                    .parse()
                    .map_err(|_| "bad --limit")?;
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
            "--json" => {}
            "--db" | "--passphrase" | "-p" => {
                // consumed by extract_db_from_all; skip value
                i += 1;
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            _ => {}
        }
        i += 1;
    }
    let (y, m) = match (year, month) {
        (Some(y), Some(m)) => (y, m),
        (Some(y), None) => (y, current_year_month().1),
        (None, Some(m)) => (current_year_month().0, m),
        _ => current_year_month(),
    };
    let flags = extract_db_from_all(args)?;
    let (ledger, tmp) = open_db(&flags)?;
    let stats = ledger
        .stats_by_currency_month(y, m)
        .map_err(|e| e.to_string())?;
    let book = BudgetBook::load();
    let budgets = if book.lines.is_empty() {
        Vec::new()
    } else {
        budget_status_month(&ledger, &book, y, m).map_err(|e| e.to_string())?
    };
    let recent = ledger
        .list_by_month(y, m, limit)
        .map_err(|e| e.to_string())?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "period": format!("{y:04}-{m:02}"),
                "stats": stats,
                "budgets": budgets,
                "recent": recent,
            })
        );
    } else {
        println!("today | {y:04}-{m:02}");
        if stats.is_empty() {
            println!("spend | (none this month)");
        } else {
            for s in &stats {
                let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
                let total = Money::new(s.total_minor, iso).display_major();
                println!("spend | {} | total={total} | n={}", s.currency, s.count);
            }
        }
        if budgets.is_empty() {
            println!("budget | (none) — rradar budget set --currency TWD --monthly 30000");
        } else {
            for s in &budgets {
                let iso = Iso4217::parse(&s.currency).unwrap_or(Iso4217::TWD);
                let spent = Money::new(s.spent_minor, iso).display_major();
                let limit_s = Money::new(s.limit_minor, iso).display_major();
                let rem = Money::new(s.remaining_minor, iso).display_major();
                let scope = s.category.as_deref().unwrap_or("overall");
                let flag = if s.over { "OVER" } else { "ok" };
                println!(
                    "budget | {flag} | {} | {scope} | spent={spent} limit={limit_s} remaining={rem} ({:.0}%)",
                    s.currency,
                    s.ratio * 100.0
                );
            }
        }
        println!("recent | up to {limit}");
        print_table(&recent);
        println!("next | rradar add <receipt>   # or: list / report / budget status");
    }
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
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
        let display = display_merchant_name(&t.merchant);
        let merch: String = display.chars().take(20).collect();
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

fn wipe_sqlite(db: &Path) {
    let _ = std::fs::remove_file(db);
    let wal = PathBuf::from(format!("{}-wal", db.display()));
    let shm = PathBuf::from(format!("{}-shm", db.display()));
    let _ = std::fs::remove_file(&wal);
    let _ = std::fs::remove_file(&shm);
}

/// Prefer local aliases, then seed dictionary short display.
fn display_merchant_name(raw: &str) -> String {
    let mut book = AliasBook::load();
    if book.ensure_tw_defaults() {
        let _ = book.save();
    }
    let aliased = book.display_for(raw);
    if aliased != raw {
        return aliased;
    }
    category_engine_with_packs()
        .suggest_display(raw)
        .unwrap_or(aliased)
}

fn utc_today_date() -> String {
    utc_now_iso().chars().take(10).collect()
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
  Images: decode + downscale longest edge to 1280 (retry 1600 on low conf).
  Alias `add` confirms into the ledger by default (daily path).
  --confirm, -c     write to ledger (default db); implied by `add`
  --preview         with `add`, parse only (do not write)
  --as-today        stamp transaction date to UTC today (daily path)
  --attach          with confirm, copy source into {db_dir}/attachments/
  --tags a,b,c      with confirm, set free-form tags (schema v3)
  --explain         show amount candidates / rules
  --json --quiet -q
  --engine mock|onnx|auto   (auto = onnx if feature+models ready)
  --currency CODE   (or RRADAR_DEFAULT_CURRENCY)
  --qr STR | --qr-file PATH
  --merchant --amount --category --date --notes
  --force           override hard dedupe
  --db PATH -p PASS",
        "today" | "home" | "status" => "\
today [--year Y --month M] [--limit N] [--json] [--db PATH]
  Daily glance: this month spend + budgets + recent rows.
  Aliases: home, status. Default period = current UTC month.",
        "month" | "close" | "monthly" => "\
month [--year Y --month M] [--currency TWD] [--top N] [-o report.md] [--csv out.csv] [--json] [--db PATH]
  Month-end close: spend + budgets + categories + top merchants (month-scoped).
  Aliases: close, monthly. -o writes markdown; --csv writes this month's ledger rows (Excel BOM).
  Default period = current UTC month.",
        "ocr" => "\
ocr <image…> [--engine mock|onnx|auto] [--max-edge 1280] [--json]
  Dump raw OCR lines (preprocess + engine) without L1 extract / ledger.
  Useful to debug real photos before process.",
        "bench" => "\
bench [DIR|FILE] [--engine mock|onnx|auto] [--json] [--no-warmup]
  A04 latency harness (shared engine; default fixtures/text).
  Prints p50/p95 ms + per-file total. Paste --json into docs/spike-ocr-size.md.
  ONNX: cargo build -p rradar-cli --features onnx --release",
        "manual" | "entry" => "\
manual --merchant NAME --amount MAJOR [--currency TWD] [--category ID] [--date YYYY-MM-DD] [--notes N]
  Insert a transaction without OCR. Alias: entry",
        "list" | "ls" | "search" => "\
list [--json] [--limit N] [--offset N] [--currency C] [--query Q]
     [--tag T] [--category ID] [--year Y --month M] [--from DATE --to DATE]
     [--min-amount N] [--max-amount N] [--has-attachment|--no-attachment] [--db PATH]
  Aliases: ls, search. Table output uses | separators.
  Tag match is whole token in comma-separated tags (schema v3).",
        "tags" => "\
tags [--json] [--db PATH]
  List distinct free-form tag tokens in the ledger.",
        "budget" => "\
budget list|status|set|clear|path
  Local soft monthly limits (data_dir/budgets.toml). Never mixed across currencies.
  set --currency TWD --monthly 30000 [--category food_dining]
  status [--year Y --month M] [--json] [--db PATH]
  clear --currency TWD [--category ID] | --all",
        "stats" => "\
stats [--year Y --month M | --from DATE --to DATE | --all] [--db PATH]
  Per-currency totals only (never mixes currencies). Default: current UTC month.
  Pair with: rradar budget status",
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
import csv <file.csv> [--db PATH]
  CSV matches `export csv` header (UTF-8 BOM OK). Empty id → new ULID; existing ids skipped.
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
        "engines" => "\
engines [--json]
  Show OCR engine availability: mock, onnx readiness, auto resolution.
  process --engine auto uses onnx when feature+models ready, else mock.",
        "licenses" | "notices" => "\
licenses [--json]
  Print project Apache-2.0 policy and THIRD_PARTY_NOTICES (alias: notices).
  Supply-chain gate: python tools/supply-chain/check_deps.py
  Docs: docs/SUPPLY-CHAIN.md",
        "release-check" | "self-check" => "\
release-check [--fixtures DIR] [--skip-demo] [--skip-api] [--quiet]
  Local pre-flight for release/install (no network):
  version, schema, engines, LICENSE/notices, process fixture, demo, api-smoke.
  Alias: self-check. Exit non-zero on any FAIL.",
        "backup" => "\
backup create -p PASS [-o file] [--db PATH]
backup restore --in file -p PASS [--db PATH] [--merge]
backup info|verify --in file -p PASS
  Argon2id + XChaCha20-Poly1305. Local-only multi-device via file copy.
  Packs ledger + transactions.json + optional attachments/** blobs.
  --merge inserts missing txs and rehydrates attachment files.
  RRADAR_FAST_BACKUP=1 for tests.",
        "attach" | "detach" => "\
attach <id> <file>     copy receipt into {db_dir}/attachments/{id}/ and set path
detach <id> [--delete-file]
  attachment_path is relative (attachments/…); portable with the data dir.",
        "seal" | "unseal" => "\
seal [--db ledger.db] -o file.rrsealed -p PASS
unseal --in file.rrsealed -o ledger.db -p PASS
  Whole-file at-rest encryption (P2).",
        "measure" | "probe" => "\
measure [--fixtures DIR] [--json] [--quiet]
  Isolated daily-path behavioral probes (Phases 1–7). Alias: probe.
  PASS = measured & trusted for this run; FAIL = do not trust yet.
  Always prints BLIND spots (explicitly unmeasured). Writes measure-report.json.
  Does not touch the personal ledger (uses a temp sandbox).",
        "day" => "\
day [--fixtures DIR] [--db PATH] [--quiet]
  30-second Taiwan daily path (recordable):
  curated fixtures → add --as-today → today glance + soft budget.
  Default day db: %APPDATA%/receiptradar/day/ledger.db (fresh each run).
  Does not touch the default user ledger unless --db or RRADAR_DB is set.
  Windows: powershell -File scripts/day.ps1",
        "scoop" | "catch" => "\
scoop [DIR] [--attach] [--keep-date] [--no-archive] [--engine mock|onnx|auto] [--quiet] [--db PATH]
  One-shot daily capture: ensure/use inbox → confirm each file (--as-today) → today.
  Alias: catch. Default dir = inbox (rradar inbox --ensure).
  --keep-date keeps OCR receipt dates instead of stamping UTC today.
  Successful files move to inbox/done/YYYY-MM-DD/ (use --no-archive to leave them).
  After drop: copy receipts into inbox, then `rradar scoop`.",
        "demo" => "\
demo [--fixtures DIR] [--db PATH] [--no-backup] [--quiet]
  Isolated closed-loop demo for recording / CI:
  text + mock_ocr + attach/tags → export → backup → report → local API smoke.
  Default demo db: %APPDATA%/receiptradar/demo/ledger.db (fresh each run).
  Does not touch the default user ledger unless --db or RRADAR_DB is set.
  RRADAR_FIXTURES overrides fixtures root discovery.
  Prefer `rradar day` for the short daily-path clip.",
        "serve" | "api-smoke" => "\
serve [--bind 127.0.0.1:7432] [--db PATH]
  Loopback-only HTTP API (no cloud). See docs/local-api.md.
api-smoke [--fixtures DIR] [--db PATH]
  Spawns ephemeral 127.0.0.1 server; process+attach+list+stats.",
        "init" | "doctor" | "path" | "config" => "\
init     create data dir + empty ledger + default config.toml
config [show|set default_currency TWD|set list_limit 50]
doctor   health check (schema version, engines, models)
path     print home + db paths
demo     one-command closed-loop from fixtures/ (recordable)
  RRADAR_HOME / RRADAR_DB / RRADAR_DEFAULT_CURRENCY override defaults.",
        "edit" | "delete" | "show" | "rm" | "trash" | "restore" | "purge" => "\
show <id>
edit <id> [--merchant --amount --currency --category --notes --date] [--tags T] [--clear-tags]
delete <id> --yes           soft-delete (schema v4 trash)
  delete <id> --yes --purge [--json]   hard-delete one row + orphan attachment cleanup
trash [--json] [--limit N]  list soft-deleted
restore <id>                undelete
  purge <id> --yes [--json] | purge --all --yes [--json]
    Database rows commit first; the report lists every attachment cleanup outcome.
  edit/delete require --db for non-default ledgers.",
        other => {
            return Err(format!(
                "no help topic `{other}` — try: process manual list stats export import backup attach seal"
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
  rradar day                 # 30s Taiwan daily path (recordable)
  rradar demo
  rradar init
  rradar add fixtures/text/familymart_89.txt --as-today --explain
  rradar today
  rradar list
  rradar report

Commands:
  init                 Create data dir + empty ledger + config
  config               Show/set local config.toml
  doctor               Environment / db check
  today                Daily glance: month spend + budget + recent (alias: home, status)
  month                Month-end close: spend + budget + categories + top (alias: close)
  day                  30s Taiwan daily closed-loop (add --as-today → today)
  scoop                Inbox → ledger today (alias: catch)
  engines              OCR engines readiness (mock|onnx|auto)
  licenses             THIRD_PARTY_NOTICES + Apache-2.0 policy (alias: notices)
  release-check        Pre-flight install/release gate (alias: self-check)
  measure              Daily-path behavioral probes + blind spots (alias: probe)
  demo                 One-command closed-loop demo (fixtures → ledger)
  fixtures             List/verify demo fixture matrix
  path                 Print default home & db paths
  process <files…>     Parse receipt(s); add --confirm to write
  add <files…>         Daily path: parse + confirm (alias of process -c)
  ocr <image…>         Raw OCR line dump (debug photos)
  bench [dir]          A04 latency harness (p50/p95; --engine onnx)
  manual               Manual entry without OCR (alias: entry)
  import json|csv      Import JSON array or CSV (export format)
  list                 List/search transactions (alias: ls, search)
  tags                 Distinct free-form tags in ledger
  budget               Local monthly soft limits (set|status|list)
  count                Transaction count
  last                 Show most recently confirmed row (JSON)
  undo --yes           Delete most recently confirmed row
  show <id>            Show one transaction (JSON)
  edit <id>            Edit merchant/amount/category/notes/date/tags
  attach <id> <file>   Store receipt blob next to ledger (schema v3)
  detach <id>          Clear attachment_path [--delete-file]
  delete <id> --yes    Soft-delete → trash (alias: rm; --purge hard)
  trash                List soft-deleted rows
  restore <id>         Undelete from trash
  purge <id>|--all --yes Hard-delete + safe attachment GC [--json report]
  stats                Per-currency totals; --by-category for breakdown
  top                  Top merchants by spend (one currency)
  month                Month-end glance + optional markdown (-o) (alias: close, monthly)
  report               Markdown monthly or annual report (-o file.md)
  aliases              Merchant display aliases (local; in backup)
  inbox [--ensure]     Show default drop folder (RRADAR_INBOX)
  scoop [dir]          Process inbox into today (alias: catch; --attach|--keep-date)
  watch [dir]          Auto-process new files (default: inbox; --as-today; --attach)
  serve [--bind 127.0.0.1:7432]  Local-only HTTP API
  api-smoke            Ephemeral loopback product API closed-loop
  recategorize         Re-run category rules (default: only `other`)
  clear --yes          Wipe all transactions
  categories           List category ids
  rules                Merchant rule packs (list|install|ensure)
  handoff              Multi-device encrypted package (create|info|apply)
  export csv|json      Export ledger
  backup create|restore|info|verify
  import json|csv|backup  Import JSON/CSV or merge from .rradar
  migrate              Apply/report ledger schema migrations
  models               ONNX pack status / SHA-256 pin verify
  engines              OCR engines readiness (mock|onnx|auto)
  seal / unseal        Whole-file encryption (.rrsealed)

process options:
  --confirm, -c        Write to ledger (default db if --db omitted); implied by `add`
  --preview            With `add`, parse only (do not write)
  --as-today           Stamp date to UTC today (so `rradar today` shows it)
  --attach             Copy source file into attachments/ on confirm
  --tags a,b           Free-form tags on confirm
  --explain            Show rules / amount candidates
  --json               JSON output
  --engine mock|onnx|auto  OCR backend (auto = onnx when ready, else mock)
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
