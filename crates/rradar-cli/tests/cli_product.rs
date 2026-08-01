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
