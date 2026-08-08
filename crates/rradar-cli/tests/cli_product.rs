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
