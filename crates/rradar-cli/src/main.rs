//! `rradar` CLI — scaffold entrypoint for Track A.
//!
//! Full `process` / `--explain` commands land in PR-A12.

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("version") | Some("--version") | Some("-V") => {
            println!("{}", rradar_core::identify());
            ExitCode::SUCCESS
        }
        Some("process") => {
            eprintln!(
                "error: `rradar process` is not implemented yet (Track A PR-A12).\n\
                 Offline OCR + extract pipeline lands after the ONNX spike (PR-A04/A05)."
            );
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("error: unknown command `{other}`\n");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!(
        "\
rradar — ReceiptRadar CLI
{ident}

USAGE:
    rradar <COMMAND>

COMMANDS:
    version     Print core version
    process     Process a receipt image (coming in PR-A12)
    help        Show this help

Local-first. No cloud. No account.
See README.md and docs/ for the thin-slice v0.1 plan.
",
        ident = rradar_core::identify()
    );
}
