//! Local-only HTTP API (loopback bind only). No cloud relay, no remote listen.
//!
//! Product surface for desktop automation, demo recording, and optional
//! machine-local integrations. Multi-device still = encrypted backup/handoff files.

use rradar_core::{
    attachments_root_for_db, category_engine_with_packs, inbox_dir, monthly_markdown,
    normalize_tags, open_ledger_auto, process_path, store_attachment, Iso4217, ProcessOptions,
    TxUpdate, LEDGER_SCHEMA_VERSION, PRODUCT_ID, VERSION,
};
use rradar_ocr::engine_by_name;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct ServeOpts {
    pub bind: String,
    pub db: std::path::PathBuf,
    pub passphrase: Option<String>,
}

struct State {
    db: std::path::PathBuf,
    passphrase: Option<String>,
}

/// True if bind address is loopback (IPv4 127.*, localhost, or IPv6 ::1).
pub fn is_loopback_bind(bind: &str) -> bool {
    let host = bind.rsplit_once(':').map(|(h, _)| h).unwrap_or(bind);
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    host == "127.0.0.1" || host == "localhost" || host == "::1" || host.starts_with("127.")
}

/// Run blocking server until process killed.
pub fn run(opts: ServeOpts) -> Result<(), String> {
    if !is_loopback_bind(&opts.bind) {
        return Err(format!(
            "refuse non-loopback bind `{}` (local-only API; use 127.0.0.1:PORT)",
            opts.bind
        ));
    }
    let listener = TcpListener::bind(&opts.bind).map_err(|e| e.to_string())?;
    let actual = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| opts.bind.clone());
    print_banner(&actual, &opts.db);
    let state = Arc::new(State {
        db: opts.db,
        passphrase: opts.passphrase,
    });
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let st = Arc::clone(&state);
                let _ = std::thread::spawn(move || {
                    if let Err(e) = handle(s, &st) {
                        eprintln!("serve | err: {e}");
                    }
                });
            }
            Err(e) => eprintln!("serve | accept: {e}"),
        }
    }
    Ok(())
}

fn print_banner(bind: &str, db: &Path) {
    // Avoid `http://` string for offline network-audit; still clear for operators.
    println!("serve | listening on {bind} (loopback only; no cloud)");
    println!("serve | db={}", db.display());
    println!(
        "serve | GET /health /version /capabilities /paths /transactions /transaction?id= /stats /report?y=&m= /models"
    );
    println!(
        "serve | POST /process  JSON {{\"path\":\"...\",\"confirm\":true,\"attach\":true,\"tags\":\"demo\"}}"
    );
    println!("serve | POST /attach   JSON {{\"id\":\"...\",\"path\":\"receipt.png\"}}");
}

/// Bind an ephemeral loopback port, run the server until `stop`, return the bound addr.
pub fn spawn_loopback(opts: ServeOpts, stop: Arc<AtomicBool>) -> Result<String, String> {
    if !is_loopback_bind(&opts.bind) {
        return Err(format!(
            "refuse non-loopback bind `{}` (local-only API)",
            opts.bind
        ));
    }
    let listener = TcpListener::bind(&opts.bind).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let addr = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .to_string();
    let state = Arc::new(State {
        db: opts.db,
        passphrase: opts.passphrase,
    });
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((s, _)) => {
                    let st = Arc::clone(&state);
                    let _ = std::thread::spawn(move || {
                        let _ = handle(s, &st);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    });
    Ok(addr)
}

/// One-shot product smoke against a live loopback server (for demo / CI).
///
/// Hits health → capabilities → process(confirm+attach) → transactions → stats.
pub fn smoke_local_api(
    db: &Path,
    fixture_path: &Path,
    passphrase: Option<&str>,
) -> Result<SmokeReport, String> {
    if !fixture_path.is_file() {
        return Err(format!("fixture missing: {}", fixture_path.display()));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let addr = spawn_loopback(
        ServeOpts {
            bind: "127.0.0.1:0".into(),
            db: db.to_path_buf(),
            passphrase: passphrase.map(|s| s.to_string()),
        },
        Arc::clone(&stop),
    )?;
    // Brief settle for accept loop
    std::thread::sleep(Duration::from_millis(40));

    let mut report = SmokeReport {
        bind: addr.clone(),
        health_ok: false,
        capabilities_ok: false,
        process_confirmed: false,
        attachment_set: false,
        transactions_n: 0,
        stats_ok: false,
    };

    let health = http_get_retry(&addr, "/health", 20)?;
    report.health_ok = status_ok(&health.status) && health.body.contains("ok");

    let caps = http_get(&addr, "/capabilities")?;
    report.capabilities_ok = status_ok(&caps.status)
        && caps.body.contains("\"cloud_sync\":false")
        && caps.body.contains("attachment_store");

    let path_json =
        serde_json::to_string(&fixture_path.display().to_string()).map_err(|e| e.to_string())?;
    let body = format!(
        r#"{{"path":{path_json},"confirm":true,"attach":true,"tags":"demo,api-smoke","engine":"mock","currency":"TWD"}}"#
    );
    let proc = http_post(&addr, "/process", &body)?;
    report.process_confirmed = status_ok(&proc.status)
        && (proc.body.contains("\"inserted\":true") || proc.body.contains("inserted"));
    report.attachment_set = proc.body.contains("attachments/") || proc.body.contains("attachment");

    let txs = http_get(&addr, "/transactions")?;
    if status_ok(&txs.status) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txs.body) {
            report.transactions_n = v.as_array().map(|a| a.len()).unwrap_or(0);
        }
    }

    let stats = http_get(&addr, "/stats")?;
    report.stats_ok = status_ok(&stats.status) && !stats.body.is_empty();

    stop.store(true, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(40));

    if !report.health_ok {
        return Err("api-smoke: /health failed".into());
    }
    if !report.capabilities_ok {
        return Err(format!(
            "api-smoke: /capabilities unexpected: {}",
            caps.body.chars().take(200).collect::<String>()
        ));
    }
    if !report.process_confirmed {
        return Err(format!(
            "api-smoke: /process confirm failed: {}",
            proc.body.chars().take(300).collect::<String>()
        ));
    }
    if report.transactions_n == 0 {
        return Err("api-smoke: /transactions empty after confirm".into());
    }
    if !report.stats_ok {
        return Err("api-smoke: /stats failed".into());
    }
    Ok(report)
}

#[derive(Debug, Clone)]
pub struct SmokeReport {
    pub bind: String,
    pub health_ok: bool,
    pub capabilities_ok: bool,
    pub process_confirmed: bool,
    pub attachment_set: bool,
    pub transactions_n: usize,
    pub stats_ok: bool,
}

struct HttpResponse {
    status: String,
    body: String,
}

fn status_ok(status_line: &str) -> bool {
    // "HTTP/1.1 200 OK" (not starts_with("200"))
    status_line.split_whitespace().nth(1) == Some("200")
}

fn http_get(addr: &str, path: &str) -> Result<HttpResponse, String> {
    raw_http(addr, "GET", path, None)
}

fn http_get_retry(addr: &str, path: &str, attempts: u32) -> Result<HttpResponse, String> {
    let mut last = "connect failed".to_string();
    for _ in 0..attempts {
        match http_get(addr, path) {
            Ok(r) => return Ok(r),
            Err(e) => {
                last = e;
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
    Err(last)
}

fn http_post(addr: &str, path: &str, body: &str) -> Result<HttpResponse, String> {
    raw_http(addr, "POST", path, Some(body))
}

fn raw_http(
    addr: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<HttpResponse, String> {
    let mut stream = TcpStream::connect(addr).map_err(|e| format!("connect {addr}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
    let body = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&buf);
    let (head, body_part) = text
        .split_once("\r\n\r\n")
        .or_else(|| text.split_once("\n\n"))
        .unwrap_or((text.as_ref(), ""));
    let status = head.lines().next().unwrap_or("HTTP/1.1 000").to_string();
    Ok(HttpResponse {
        status,
        body: body_part.to_string(),
    })
}

fn handle(mut stream: TcpStream, st: &State) -> Result<(), String> {
    let mut buf = [0u8; 65536];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut lines = req.lines();
    let first = lines.next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let target = parts.next().unwrap_or("/");
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };

    // Preflight for local HTML demos (null origin).
    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: null\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
        stream
            .write_all(resp.as_bytes())
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let (status, ctype, body) = match (method, path) {
        ("GET", "/health") | ("GET", "/") => {
            ("200 OK", "text/plain; charset=utf-8", "ok".to_string())
        }
        ("GET", "/version") => (
            "200 OK",
            "application/json",
            format!(
                "{{\"product_id\":\"{PRODUCT_ID}\",\"version\":\"{VERSION}\",\"ledger_schema\":{LEDGER_SCHEMA_VERSION},\"local_only\":true}}"
            ),
        ),
        ("GET", "/capabilities") => (
            "200 OK",
            "application/json",
            serde_json::json!({
                "product_id": PRODUCT_ID,
                "version": VERSION,
                "ledger_schema": LEDGER_SCHEMA_VERSION,
                "cloud_sync": false,
                "official_relay": false,
                "multi_device_handoff": true,
                "rule_packs": true,
                "local_http_serve": true,
                "tags_attachments": true,
                "attachment_store": true,
                "backup_includes_attachments": true,
                "capture_oneshot": true,
                "engines": ["mock", "onnx"],
                "notes": "local-first; multi-device via backup/handoff file only",
            })
            .to_string(),
        ),
        ("GET", "/paths") => (
            "200 OK",
            "application/json",
            serde_json::json!({
                "db": st.db.display().to_string(),
                "attachments": attachments_root_for_db(&st.db).display().to_string(),
                "inbox": inbox_dir().display().to_string(),
                "local_only": true,
            })
            .to_string(),
        ),
        ("GET", "/transactions") => json_result(|| {
            let (ledger, tmp) =
                open_ledger_auto(&st.db, st.passphrase.as_deref()).map_err(|e| e.to_string())?;
            let limit = query_param(query, "limit")
                .and_then(|s| s.parse().ok())
                .unwrap_or(200usize);
            let currency = query_param(query, "currency");
            let q = query_param(query, "q").or_else(|| query_param(query, "query"));
            let rows = ledger
                .list_filtered(limit, 0, currency, q)
                .map_err(|e| e.to_string())?;
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            serde_json::to_string(&rows).map_err(|e| e.to_string())
        }),
        ("GET", "/transaction") => json_result(|| {
            let id = query_param(query, "id").ok_or("missing id query param")?;
            let (ledger, tmp) =
                open_ledger_auto(&st.db, st.passphrase.as_deref()).map_err(|e| e.to_string())?;
            let tx = ledger.get_transaction(id).map_err(|e| e.to_string())?;
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            serde_json::to_string(&tx).map_err(|e| e.to_string())
        }),
        ("GET", "/stats") => json_result(|| {
            let (ledger, tmp) =
                open_ledger_auto(&st.db, st.passphrase.as_deref()).map_err(|e| e.to_string())?;
            let rows = ledger.stats_by_currency_all().map_err(|e| e.to_string())?;
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            serde_json::to_string(&rows).map_err(|e| e.to_string())
        }),
        ("GET", "/models") => json_result(|| {
            let dir = rradar_ocr::default_models_dir();
            let checks = rradar_ocr::verify_models_dir(&dir, false).unwrap_or_default();
            let pins: Vec<serde_json::Value> = checks
                .iter()
                .map(|c| match c {
                    rradar_ocr::PinCheck::Ok { pin, bytes } => serde_json::json!({
                        "file": pin.filename,
                        "status": "ok",
                        "bytes": bytes,
                        "sha256": pin.sha256_hex,
                    }),
                    rradar_ocr::PinCheck::Missing { pin } => serde_json::json!({
                        "file": pin.filename,
                        "status": "missing",
                        "sha256": pin.sha256_hex,
                    }),
                    rradar_ocr::PinCheck::Mismatch {
                        pin,
                        actual_hex,
                        bytes,
                    } => serde_json::json!({
                        "file": pin.filename,
                        "status": "mismatch",
                        "bytes": bytes,
                        "sha256_expected": pin.sha256_hex,
                        "sha256_actual": actual_hex,
                    }),
                })
                .collect();
            Ok(serde_json::json!({
                "dir": dir.display().to_string(),
                "onnx_feature": rradar_ocr::onnx_feature_enabled(),
                "pins_ok": rradar_ocr::all_pins_ok(&checks),
                "pins": pins,
                "local_only": true,
            })
            .to_string())
        }),
        ("GET", "/report") => {
            let y = query_param(query, "y")
                .and_then(|s| s.parse().ok())
                .unwrap_or(2024);
            let m = query_param(query, "m")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            match (|| {
                let (ledger, tmp) = open_ledger_auto(&st.db, st.passphrase.as_deref())
                    .map_err(|e| e.to_string())?;
                let md = monthly_markdown(&ledger, y, m).map_err(|e| e.to_string())?;
                if let Some(t) = tmp {
                    let _ = std::fs::remove_file(t);
                }
                Ok::<_, String>(md)
            })() {
                Ok(md) => ("200 OK", "text/markdown; charset=utf-8", md),
                Err(e) => ("500 Internal Server Error", "text/plain; charset=utf-8", e),
            }
        }
        ("POST", "/process") => {
            let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(req.len());
            let body = &req[body_start..];
            match process_post(body, st) {
                Ok(j) => ("200 OK", "application/json", j),
                Err(e) => (
                    "400 Bad Request",
                    "application/json",
                    format!(
                        "{{\"error\":{}}}",
                        serde_json::to_string(&e).unwrap_or_default()
                    ),
                ),
            }
        }
        ("POST", "/attach") => {
            let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(req.len());
            let body = &req[body_start..];
            match attach_post(body, st) {
                Ok(j) => ("200 OK", "application/json", j),
                Err(e) => (
                    "400 Bad Request",
                    "application/json",
                    format!(
                        "{{\"error\":{}}}",
                        serde_json::to_string(&e).unwrap_or_default()
                    ),
                ),
            }
        }
        _ => (
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found".into(),
        ),
    };

    // CORS: null origin only — local file:// demos; not a public API.
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: null\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(resp.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn json_result(f: impl FnOnce() -> Result<String, String>) -> (&'static str, &'static str, String) {
    match f() {
        Ok(j) => ("200 OK", "application/json", j),
        Err(e) => (
            "500 Internal Server Error",
            "application/json",
            format!(
                "{{\"error\":{}}}",
                serde_json::to_string(&e).unwrap_or_default()
            ),
        ),
    }
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(v);
            }
        }
    }
    None
}

fn process_post(body: &str, st: &State) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Req {
        path: String,
        #[serde(default)]
        confirm: bool,
        #[serde(default)]
        attach: bool,
        #[serde(default)]
        tags: Option<String>,
        #[serde(default)]
        force: bool,
        #[serde(default)]
        engine: Option<String>,
        #[serde(default)]
        currency: Option<String>,
    }
    let req: Req = serde_json::from_str(body.trim()).map_err(|e| e.to_string())?;
    let eng_name = req.engine.as_deref().unwrap_or("mock");
    let eng = engine_by_name(eng_name).map_err(|e| e.to_string())?;
    let cats = category_engine_with_packs();
    let currency = req
        .currency
        .as_deref()
        .and_then(Iso4217::parse)
        .unwrap_or(Iso4217::TWD);
    let path = Path::new(&req.path);
    let draft = process_path(
        path,
        eng.as_ref(),
        &cats,
        ProcessOptions {
            default_currency: currency,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    if !req.confirm {
        return serde_json::to_string(&draft).map_err(|e| e.to_string());
    }
    let (ledger, tmp) =
        open_ledger_auto(&st.db, st.passphrase.as_deref()).map_err(|e| e.to_string())?;
    let hash = rradar_core::preprocess::content_hash(&std::fs::read(path).unwrap_or_default());
    let result = ledger
        .confirm_draft(&draft, Some(&hash), None, req.force)
        .map_err(|e| e.to_string())?;

    let mut out_tx = result.transaction.clone();
    if result.inserted {
        let mut patch = TxUpdate::default();
        if req.attach && path.is_file() {
            if let Ok(rel) = store_attachment(ledger.path(), &result.transaction.id, path) {
                patch.attachment_path = Some(rel);
            }
        }
        if let Some(ref t) = req.tags {
            patch.tags = Some(normalize_tags(t).unwrap_or_default());
        }
        if patch.attachment_path.is_some() || patch.tags.is_some() {
            if let Ok(tx) = ledger.update_transaction(&result.transaction.id, &patch) {
                out_tx = tx;
            }
        }
    }

    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    Ok(serde_json::json!({
        "transaction": out_tx,
        "dedupe": result.dedupe,
        "inserted": result.inserted,
        "local_only": true,
    })
    .to_string())
}

fn attach_post(body: &str, st: &State) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Req {
        id: String,
        path: String,
    }
    let req: Req = serde_json::from_str(body.trim()).map_err(|e| e.to_string())?;
    let (ledger, tmp) =
        open_ledger_auto(&st.db, st.passphrase.as_deref()).map_err(|e| e.to_string())?;
    let _ = ledger.get_transaction(&req.id).map_err(|e| e.to_string())?;
    let rel = store_attachment(ledger.path(), &req.id, Path::new(&req.path))
        .map_err(|e| e.to_string())?;
    let tx = ledger
        .update_transaction(
            &req.id,
            &TxUpdate {
                attachment_path: Some(rel),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    serde_json::to_string(&tx).map_err(|e| e.to_string())
}

/// Default bind used when CLI omits --bind.
pub fn default_bind() -> String {
    "127.0.0.1:7432".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rradar_core::Ledger;
    use std::path::PathBuf;

    #[test]
    fn loopback_accepts_local_only() {
        assert!(is_loopback_bind("127.0.0.1:7432"));
        assert!(is_loopback_bind("localhost:9"));
        assert!(is_loopback_bind("[::1]:7432") || is_loopback_bind("::1:7432"));
        assert!(!is_loopback_bind("0.0.0.0:7432"));
        assert!(!is_loopback_bind("192.168.1.1:80"));
    }

    #[test]
    fn api_smoke_process_attach_and_list() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture = root.join("fixtures/text/familymart_89.txt");
        assert!(fixture.is_file(), "need fixture at {}", fixture.display());
        let dir = std::env::temp_dir().join(format!("rradar-api-smoke-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("ledger.db");
        Ledger::open(&db).unwrap();

        let report = smoke_local_api(&db, &fixture, None).expect("smoke");
        assert!(report.health_ok);
        assert!(report.capabilities_ok);
        assert!(report.process_confirmed);
        assert!(report.transactions_n >= 1);
        assert!(report.stats_ok);
        // attach of a .txt is fine for local store
        assert!(
            report.attachment_set,
            "expected attachment path in process response"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
