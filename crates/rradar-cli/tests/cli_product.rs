//! End-to-end CLI product smoke (invokes binary).

use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rradar"));
    let home = std::env::temp_dir().join(format!("rradar-cli-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    c.env("RRADAR_HOME", &home);
    c.env("RRADAR_DB", home.join("ledger.db"));
    c.env("RRADAR_FAST_BACKUP", "1");
    c
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/familymart_89.txt")
}

fn temp_case(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("rradar-cli-{label}-{}-{nonce}", std::process::id()))
}

fn isolated_bin(home: &std::path::Path, db: &std::path::Path) -> Command {
    let mut command = bin();
    command.env("RRADAR_HOME", home).env("RRADAR_DB", db);
    command
}

fn create_attached_transaction(home: &std::path::Path, db: &std::path::Path) -> (String, PathBuf) {
    std::fs::create_dir_all(home).unwrap();
    let fixture = fixtures();
    let output = isolated_bin(home, db)
        .args([
            "process",
            fixture.to_str().unwrap(),
            "--confirm",
            "--attach",
            "--quiet",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "process: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let listed = isolated_bin(home, db)
        .args(["list", "--json", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let rows: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let row = &rows[0];
    let id = row["id"].as_str().unwrap().to_owned();
    let stored = row["attachment_path"].as_str().unwrap();
    let file = home.join(stored);
    assert!(file.is_file(), "missing attachment: {}", file.display());
    (id, file)
}

#[test]
fn init_process_list_stats_export_edit_delete() {
    let fx = fixtures();
    assert!(fx.is_file(), "fixture missing: {}", fx.display());

    let out = bin().args(["init"]).output().unwrap();
    assert!(
        out.status.success(),
        "init: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = bin()
        .args(["process", fx.to_str().unwrap(), "--confirm", "--quiet"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "process: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("confirmed"), "{stdout}");

    let out = bin().args(["list", "--json"]).output().unwrap();
    assert!(out.status.success());
    let list = String::from_utf8_lossy(&out.stdout);
    assert!(list.contains("8900") || list.contains("全家"), "{list}");

    let out = bin()
        .args(["stats", "--year", "2024", "--month", "5"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("TWD"));

    let out = bin().args(["export", "csv"]).output().unwrap();
    assert!(out.status.success());
    // BOM + header
    let csv = out.stdout;
    assert!(csv.len() > 3);
    assert_eq!(&csv[0..3], &[0xEF, 0xBB, 0xBF]);

    // extract id from list json
    let v: serde_json::Value =
        serde_json::from_slice(&bin().args(["list", "--json"]).output().unwrap().stdout).unwrap();
    let id = v[0]["id"].as_str().unwrap().to_string();

    let out = bin()
        .args(["edit", &id, "--notes", "cli-test"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("cli-test"));

    let out = bin().args(["show", &id]).output().unwrap();
    assert!(out.status.success());

    let out = bin().args(["delete", &id, "--yes"]).output().unwrap();
    assert!(out.status.success());

    let out = bin().args(["list", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 0);

    let out = bin().args(["help", "process"]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("--confirm"));

    let out = bin()
        .args(["manual", "--merchant", "ManualShop", "--amount", "3"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("confirmed"));
}

#[test]
fn version_long_and_json() {
    let out = bin().args(["version"]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("receiptradar"));

    let out = bin().args(["version", "--long"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("ledger_schema"), "{s}");
    assert!(s.contains("local-first"), "{s}");

    let out = bin().args(["version", "--json"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("\"product_id\""), "{s}");
    assert!(s.contains("\"ledger_schema\""), "{s}");
}

#[test]
fn attach_backup_restore_reads_attachment_bytes() {
    let home = temp_case("attach-backup-restore");
    let db = home.join("ledger.db");
    let bak = home.join("pack.rradar");
    let restored_home = home.join("restored");
    let restored_db = restored_home.join("ledger.db");
    std::fs::create_dir_all(&restored_home).unwrap();

    let (id, attachment) = create_attached_transaction(&home, &db);
    let original = std::fs::read(&attachment).unwrap();

    let create = isolated_bin(&home, &db)
        .args([
            "backup",
            "create",
            "-p",
            "test-pass",
            "-o",
            bak.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        create.status.success(),
        "{}",
        String::from_utf8_lossy(&create.stderr)
    );

    let restore = isolated_bin(&home, &db)
        .args([
            "backup",
            "restore",
            "--in",
            bak.to_str().unwrap(),
            "-p",
            "test-pass",
            "--db",
            restored_db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        restore.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&restore.stdout),
        String::from_utf8_lossy(&restore.stderr)
    );

    let show = isolated_bin(&restored_home, &restored_db)
        .args(["show", &id, "--json", "--db", restored_db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        show.status.success(),
        "{}",
        String::from_utf8_lossy(&show.stderr)
    );
    let row: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    let stored = row["attachment_path"].as_str().expect("attachment_path");
    let restored_file = restored_home.join(stored);
    assert!(
        restored_file.is_file(),
        "missing restored attachment {}",
        restored_file.display()
    );
    assert_eq!(std::fs::read(&restored_file).unwrap(), original);
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn every_cli_purge_entry_point_cleans_attachments_and_emits_json() {
    for mode in ["purge", "delete-purge", "purge-all"] {
        let home = temp_case(mode);
        let db = home.join("ledger.db");
        let (id, attachment) = create_attached_transaction(&home, &db);

        if mode == "purge-all" {
            let trashed = isolated_bin(&home, &db)
                .args(["delete", &id, "--yes", "--db", db.to_str().unwrap()])
                .output()
                .unwrap();
            assert!(
                trashed.status.success(),
                "{}",
                String::from_utf8_lossy(&trashed.stderr)
            );
        }

        let output = match mode {
            "purge" => isolated_bin(&home, &db)
                .args([
                    "purge",
                    &id,
                    "--yes",
                    "--json",
                    "--db",
                    db.to_str().unwrap(),
                ])
                .output()
                .unwrap(),
            "delete-purge" => isolated_bin(&home, &db)
                .args([
                    "delete",
                    &id,
                    "--yes",
                    "--purge",
                    "--json",
                    "--db",
                    db.to_str().unwrap(),
                ])
                .output()
                .unwrap(),
            "purge-all" => isolated_bin(&home, &db)
                .args([
                    "purge",
                    "--all",
                    "--yes",
                    "--json",
                    "--db",
                    db.to_str().unwrap(),
                ])
                .output()
                .unwrap(),
            _ => unreachable!(),
        };
        assert!(
            output.status.success(),
            "{mode}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["purged_transactions"].as_u64(), Some(1), "{report}");
        assert_eq!(
            report["attachments"]["deleted"].as_array().map(Vec::len),
            Some(1),
            "{report}"
        );
        assert!(!attachment.exists(), "{mode} left attachment behind");
        assert!(home.join("attachments").is_dir());
        std::fs::remove_dir_all(home).unwrap();
    }
}

#[test]
fn cli_purge_refusal_and_post_commit_cleanup_failure_are_honest() {
    let home = temp_case("purge-failure");
    let db = home.join("ledger.db");
    let (id, attachment) = create_attached_transaction(&home, &db);

    let refused = isolated_bin(&home, &db)
        .args(["purge", &id, "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(attachment.is_file());

    std::fs::remove_file(&attachment).unwrap();
    std::fs::create_dir(&attachment).unwrap();
    let output = isolated_bin(&home, &db)
        .args([
            "purge",
            &id,
            "--yes",
            "--json",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["purged_transactions"].as_u64(), Some(1));
    assert_eq!(
        report["attachments"]["cleanup_errors"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let rows = isolated_bin(&home, &db)
        .args(["list", "--json", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    let rows: serde_json::Value = serde_json::from_slice(&rows.stdout).unwrap();
    assert_eq!(rows.as_array().map(Vec::len), Some(0));
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn cli_purge_of_sealed_ledger_persists_before_attachment_cleanup() {
    let home = temp_case("sealed-purge");
    let db = home.join("ledger.db");
    let sealed = home.join("ledger.rrsealed");
    let reopened = home.join("reopened.db");
    let (id, attachment) = create_attached_transaction(&home, &db);

    let seal = isolated_bin(&home, &db)
        .args([
            "seal",
            "--db",
            db.to_str().unwrap(),
            "--out",
            sealed.to_str().unwrap(),
            "--passphrase",
            "test-passphrase",
        ])
        .output()
        .unwrap();
    assert!(
        seal.status.success(),
        "{}",
        String::from_utf8_lossy(&seal.stderr)
    );

    let purge = isolated_bin(&home, &sealed)
        .args([
            "purge",
            &id,
            "--yes",
            "--json",
            "--db",
            sealed.to_str().unwrap(),
            "--passphrase",
            "test-passphrase",
        ])
        .output()
        .unwrap();
    assert!(
        purge.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&purge.stdout),
        String::from_utf8_lossy(&purge.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&purge.stdout).unwrap();
    assert_eq!(report["purged_transactions"].as_u64(), Some(1));
    assert_eq!(
        report["attachments"]["deleted"].as_array().map(Vec::len),
        Some(1)
    );
    assert!(!attachment.exists());

    let unseal = isolated_bin(&home, &sealed)
        .args([
            "unseal",
            "--in",
            sealed.to_str().unwrap(),
            "--out",
            reopened.to_str().unwrap(),
            "--passphrase",
            "test-passphrase",
        ])
        .output()
        .unwrap();
    assert!(
        unseal.status.success(),
        "{}",
        String::from_utf8_lossy(&unseal.stderr)
    );
    let rows = isolated_bin(&home, &reopened)
        .args(["list", "--json", "--db", reopened.to_str().unwrap()])
        .output()
        .unwrap();
    let rows: serde_json::Value = serde_json::from_slice(&rows.stdout).unwrap();
    assert_eq!(rows.as_array().map(Vec::len), Some(0));
    assert!(home.join("attachments").is_dir());
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn backup_info_verify_and_merge() {
    let fx = fixtures();
    let home = std::env::temp_dir().join(format!("rradar-bak-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    let db = home.join("ledger.db");
    let bak = home.join("t.rradar");

    assert!(bin()
        .args(["init"])
        .env("RRADAR_HOME", &home)
        .env("RRADAR_DB", &db)
        .output()
        .unwrap()
        .status
        .success());

    // Use isolated env for remaining commands via bin() which already sets home —
    // override DB path explicitly.
    let mut c = bin();
    c.env("RRADAR_HOME", &home);
    c.env("RRADAR_DB", &db);
    let out = c
        .args([
            "process",
            fx.to_str().unwrap(),
            "--confirm",
            "-q",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = bin()
        .env("RRADAR_HOME", &home)
        .env("RRADAR_DB", &db)
        .env("RRADAR_FAST_BACKUP", "1")
        .args([
            "backup",
            "create",
            "-p",
            "test-pass",
            "-o",
            bak.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(bak.is_file());

    let out = bin()
        .args([
            "backup",
            "info",
            "--in",
            bak.to_str().unwrap(),
            "-p",
            "test-pass",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("ledger_schema"), "{s}");
    assert!(s.contains("local-first"), "{s}");

    let out = bin()
        .args([
            "backup",
            "verify",
            "--in",
            bak.to_str().unwrap(),
            "-p",
            "test-pass",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("OK"));

    let out = bin()
        .args(["migrate", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("schema="));

    // Merge into empty second db
    let db2 = home.join("ledger2.db");
    let out = bin()
        .args([
            "backup",
            "restore",
            "--in",
            bak.to_str().unwrap(),
            "-p",
            "test-pass",
            "--db",
            db2.to_str().unwrap(),
            "--merge",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("inserted="));
}

#[test]
fn demo_closed_loop() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    assert!(fixtures.is_dir(), "fixtures missing");
    let home = std::env::temp_dir().join(format!("rradar-demo-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    let db = home.join("ledger.db");
    let out = bin()
        .args([
            "demo",
            "--fixtures",
            fixtures.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "demo: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("DEMO_OK"), "{stdout}");
    assert!(db.is_file(), "demo ledger not created");
}

#[test]
fn bench_mock_text_fixtures() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text");
    assert!(fixtures.is_dir(), "fixtures/text missing");
    let out = bin()
        .args([
            "bench",
            fixtures.to_str().unwrap(),
            "--engine",
            "mock",
            "--json",
            "--no-warmup",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "bench: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("bench json");
    assert!(v["success"].as_u64().unwrap_or(0) >= 1, "{v}");
    assert!(
        v["p50_ms"].as_u64().is_some() || v["success"].as_u64() == Some(0),
        "{v}"
    );
    assert_eq!(v["engine"].as_str(), Some("mock"));
}

#[test]
fn measure_daily_path_ok() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    assert!(fixtures.is_dir(), "fixtures missing");
    let out = bin()
        .args([
            "measure",
            "--fixtures",
            fixtures.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "measure: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("MEASURE_OK"), "{stdout}");
    assert!(stdout.contains("fail=0"), "{stdout}");
}

#[test]
fn month_close_glance() {
    let home = temp_case("month");
    let db = home.join("ledger.db");
    std::fs::create_dir_all(&home).unwrap();
    let fx = fixtures();
    let out = isolated_bin(&home, &db)
        .args([
            "add",
            fx.to_str().unwrap(),
            "--as-today",
            "--quiet",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success());

    let md = home.join("month.md");
    let csv = home.join("month.csv");
    let out = isolated_bin(&home, &db)
        .args([
            "month",
            "--db",
            db.to_str().unwrap(),
            "-o",
            md.to_str().unwrap(),
            "--csv",
            csv.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "month: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("MONTH_OK"), "{s}");
    assert!(s.contains("spend |"), "{s}");
    assert!(s.contains("categories |"), "{s}");
    assert!(s.contains("top |"), "{s}");
    assert!(s.contains("wrote | csv |"), "{s}");
    assert!(md.is_file(), "markdown not written");
    assert!(csv.is_file(), "csv not written");
    let body = std::fs::read_to_string(&md).unwrap();
    assert!(body.contains("ReceiptRadar report"), "{body}");
    let csv_bytes = std::fs::read(&csv).unwrap();
    assert_eq!(
        &csv_bytes[0..3],
        &[0xEF, 0xBB, 0xBF],
        "CSV should have UTF-8 BOM"
    );
    let csv_text = String::from_utf8_lossy(&csv_bytes);
    assert!(
        csv_text.contains("全家") || csv_text.contains("8900"),
        "{csv_text}"
    );

    let json = isolated_bin(&home, &db)
        .args([
            "close",
            "--json",
            "--db",
            db.to_str().unwrap(),
            "--csv",
            home.join("via-json.csv").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(json.status.success());
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert!(v["period"].as_str().unwrap().len() == 7, "{v}");
    assert!(!v["stats"].as_array().unwrap().is_empty(), "{v}");
    assert_eq!(v["csv_rows"].as_u64(), Some(1), "{v}");
    assert!(home.join("via-json.csv").is_file());
}

#[test]
fn scoop_inbox_as_today() {
    let home = temp_case("scoop");
    let db = home.join("ledger.db");
    let inbox = home.join("inbox");
    std::fs::create_dir_all(&inbox).unwrap();
    let fx = fixtures();
    std::fs::copy(&fx, inbox.join("familymart_89.txt")).unwrap();
    let tea = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/text/bubbletea_50lan_tw.txt");
    std::fs::copy(&tea, inbox.join("bubbletea_50lan_tw.txt")).unwrap();

    let mut cmd = isolated_bin(&home, &db);
    cmd.env("RRADAR_INBOX", &inbox);
    let out = cmd
        .args(["scoop", "--quiet", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "scoop: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("SCOOP_OK"), "{stdout}");
    assert!(stdout.contains("archived=2"), "{stdout}");

    // Top-level inbox should be empty; files under done/
    let top: Vec<_> = std::fs::read_dir(&inbox)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file())
        .collect();
    assert!(top.is_empty(), "expected archived out of inbox top-level");
    let done_day = inbox.join("done");
    assert!(done_day.is_dir(), "missing done/");

    let today = isolated_bin(&home, &db)
        .args(["today", "--json", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(today.status.success());
    let v: serde_json::Value = serde_json::from_slice(&today.stdout).unwrap();
    let recent = v["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 2, "{v}");
    let period = v["period"].as_str().unwrap();
    for row in recent {
        let date = row["transacted_at"].as_str().unwrap();
        assert!(date.starts_with(period), "date {date} vs period {period}");
    }

    // Second scoop should see empty inbox
    let mut cmd2 = isolated_bin(&home, &db);
    cmd2.env("RRADAR_INBOX", &inbox);
    let out2 = cmd2
        .args(["scoop", "--quiet", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out2.status.success());
    let s2 = String::from_utf8_lossy(&out2.stdout);
    assert!(s2.contains("SCOOP_OK n=0"), "{s2}");
}

#[test]
fn day_closed_loop() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    assert!(fixtures.is_dir(), "fixtures missing");
    let home = temp_case("day");
    let db = home.join("ledger.db");
    std::fs::create_dir_all(&home).unwrap();
    let out = isolated_bin(&home, &db)
        .args([
            "day",
            "--fixtures",
            fixtures.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "day: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("DAY_OK"), "{stdout}");
    assert!(db.is_file(), "day ledger not created");

    let today = isolated_bin(&home, &db)
        .args(["today", "--json", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(today.status.success());
    let v: serde_json::Value = serde_json::from_slice(&today.stdout).unwrap();
    let recent = v["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 5, "{v}");
}

#[test]
fn add_defaults_to_confirm_and_today_glance() {
    let home = temp_case("today");
    let db = home.join("ledger.db");
    std::fs::create_dir_all(&home).unwrap();
    let fx = fixtures();

    // `add` writes without --confirm
    let out = isolated_bin(&home, &db)
        .args([
            "add",
            fx.to_str().unwrap(),
            "--quiet",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "add: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("confirmed"), "{stdout}");

    // `--preview` must NOT write a second row
    let out = isolated_bin(&home, &db)
        .args([
            "add",
            fx.to_str().unwrap(),
            "--preview",
            "--quiet",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "preview: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let preview = String::from_utf8_lossy(&out.stdout);
    assert!(
        !preview.contains("confirmed"),
        "preview should not confirm: {preview}"
    );

    let count = isolated_bin(&home, &db)
        .args(["count", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(count.status.success());
    let count_s = String::from_utf8_lossy(&count.stdout);
    assert!(
        count_s.contains('1') || count_s.trim() == "1",
        "expected 1 tx after preview: {count_s}"
    );

    // Fixture is dated 2024-05 — today for that month should list it
    let out = isolated_bin(&home, &db)
        .args([
            "today",
            "--year",
            "2024",
            "--month",
            "5",
            "--json",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "today: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["period"], "2024-05");
    let recent = v["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 1, "{v}");
    assert!(
        recent[0]["merchant"]
            .as_str()
            .unwrap_or("")
            .contains("全家")
            || recent[0]["amount_minor"].as_i64() == Some(8900),
        "{v}"
    );

    // Human today + aliases
    for cmd in ["today", "home", "status"] {
        let out = isolated_bin(&home, &db)
            .args([
                cmd,
                "--year",
                "2024",
                "--month",
                "5",
                "--db",
                db.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{cmd}: {}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let s = String::from_utf8_lossy(&out.stdout);
        assert!(s.contains("today | 2024-05"), "{cmd}: {s}");
        assert!(s.contains("spend |"), "{cmd}: {s}");
        assert!(s.contains("recent |"), "{cmd}: {s}");
    }
}

#[test]
fn add_as_today_lands_in_current_month_glance() {
    let home = temp_case("as-today");
    let db = home.join("ledger.db");
    std::fs::create_dir_all(&home).unwrap();
    let fx = fixtures();

    let out = isolated_bin(&home, &db).args(["init"]).output().unwrap();
    assert!(
        out.status.success(),
        "init: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = isolated_bin(&home, &db)
        .args([
            "add",
            fx.to_str().unwrap(),
            "--as-today",
            "--quiet",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "add --as-today: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let out = isolated_bin(&home, &db)
        .args(["today", "--json", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "today: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let recent = v["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 1, "{v}");
    let period = v["period"].as_str().unwrap();
    let date = recent[0]["transacted_at"].as_str().unwrap();
    assert!(
        date.starts_with(period),
        "expected date {date} in period {period}"
    );
    // seed / alias short name path still keeps raw merchant in JSON
    assert!(
        recent[0]["merchant"]
            .as_str()
            .unwrap_or("")
            .contains("全家"),
        "{v}"
    );

    let human = isolated_bin(&home, &db)
        .args(["today", "--db", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(human.status.success());
    let s = String::from_utf8_lossy(&human.stdout);
    assert!(s.contains("spend | TWD"), "{s}");
    // Display should shorten branch name when aliases/seed apply
    assert!(
        s.contains("全家") && !s.contains("臨江店"),
        "expected short merchant display: {s}"
    );
}

#[test]
fn release_check_ok() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    assert!(fixtures.is_dir(), "fixtures missing");
    let out = bin()
        .args([
            "release-check",
            "--fixtures",
            fixtures.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "release-check: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("RELEASE_CHECK_OK"), "{stdout}");
}
