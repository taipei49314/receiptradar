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
