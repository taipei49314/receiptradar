//! Local-only HTTP API (127.0.0.1). No cloud, no auth beyond bind address.

use rradar_core::{
    monthly_markdown, open_ledger_auto, process_path, CategoryEngine, Iso4217, ProcessOptions,
    PRODUCT_ID, VERSION,
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

/// Run blocking server until process killed.
pub fn run(opts: ServeOpts) -> Result<(), String> {
    let listener = TcpListener::bind(&opts.bind).map_err(|e| e.to_string())?;
    println!("serve | http://{} (local-only; no cloud)", opts.bind);
    println!("serve | db={}", opts.db.display());
    println!("serve | GET /health /version /transactions /stats /report?y=&m=");
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

    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n{body}",
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
    let cats = CategoryEngine::with_seed();
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
