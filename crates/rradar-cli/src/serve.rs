//! Local-only HTTP API (loopback bind only). No cloud relay, no remote listen.

use rradar_core::{
    category_engine_with_packs, monthly_markdown, open_ledger_auto, process_path, Iso4217,
    ProcessOptions, PRODUCT_ID, VERSION,
};
use rradar_ocr::engine_by_name;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;

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
    // Avoid `http://` string for offline network-audit; still clear for operators.
    println!(
        "serve | listening on {} (loopback only; no cloud)",
        opts.bind
    );
    println!("serve | db={}", opts.db.display());
    println!("serve | GET /health /version /transactions /stats /report?y=&m= /models");
    println!("serve | POST /process  JSON {{\"path\":\"...\",\"confirm\":true}}");
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

fn handle(mut stream: TcpStream, st: &State) -> Result<(), String> {
    let mut buf = [0u8; 8192];
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

    let (status, ctype, body) = match (method, path) {
        ("GET", "/health") | ("GET", "/") => {
            ("200 OK", "text/plain; charset=utf-8", "ok".to_string())
        }
        ("GET", "/version") => (
            "200 OK",
            "application/json",
            format!(
                "{{\"product_id\":\"{PRODUCT_ID}\",\"version\":\"{VERSION}\",\"local_only\":true}}"
            ),
        ),
        ("GET", "/transactions") => json_result(|| {
            let (ledger, tmp) =
                open_ledger_auto(&st.db, st.passphrase.as_deref()).map_err(|e| e.to_string())?;
            let rows = ledger
                .list_transactions(200, 0)
                .map_err(|e| e.to_string())?;
            if let Some(t) = tmp {
                let _ = std::fs::remove_file(t);
            }
            serde_json::to_string(&rows).map_err(|e| e.to_string())
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
    let draft = process_path(
        Path::new(&req.path),
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
    let hash = rradar_core::preprocess::content_hash(&std::fs::read(&req.path).unwrap_or_default());
    let result = ledger
        .confirm_draft(&draft, Some(&hash), None, false)
        .map_err(|e| e.to_string())?;
    if let Some(t) = tmp {
        let _ = std::fs::remove_file(t);
    }
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

/// Default bind used when CLI omits --bind.
pub fn default_bind() -> String {
    "127.0.0.1:7432".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_accepts_local_only() {
        assert!(is_loopback_bind("127.0.0.1:7432"));
        assert!(is_loopback_bind("localhost:9"));
        assert!(is_loopback_bind("[::1]:7432") || is_loopback_bind("::1:7432"));
        assert!(!is_loopback_bind("0.0.0.0:7432"));
        assert!(!is_loopback_bind("192.168.1.1:80"));
    }
}
