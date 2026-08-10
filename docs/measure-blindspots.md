# Daily-path measure & blind spots

**Policy:** do not trust a behavior until `rradar measure` marks it `PASS`.  
`BLIND` = explicitly unmeasured — unknown, not “fine”.

```bash
cargo run -p rradar-cli -- measure --fixtures fixtures
cargo run -p rradar-cli -- measure --quiet
# JSON report also written under the printed sandbox path
```

Exit: `MEASURE_OK pass=N fail=0 blind=M` or `MEASURE_FAIL …`.

## What is measured (trust only if PASS)

| Probe | Behavior |
|-------|----------|
| `add_default_confirm` | `add` writes without `-c` |
| `add_preview_no_write` | `add --preview` does not insert |
| `as_today_current_month` | `--as-today` stamps UTC today month |
| `merchant_display_short` | branch names shorten (aliases/seed) |
| `today_month_stats` | month stats non-empty after as-today |
| `extract_prefer_price_tax_total` | 價稅合計 beats 應稅 |
| `category_ibon_not_seven` | ibon → shopping, not 7-ELEVEN grocery |
| `scoop_archive_clears_inbox` | scoop archives to `done/` |
| `scoop_second_is_noop` | second scoop does not double-count |
| `month_csv_bom` | `month -o/--csv` writes md + UTF-8 BOM csv |
| `day_closed_loop` | `day --quiet` → DAY_OK |
| `fixture_cht_bill` | 中華電信 價稅合計 699 / utilities |
| `fixture_ibon_shopping` | ibon 35 / shopping |
| `watch_once_as_today` | `watch --once --as-today --db` writes current month |
| `scoop_attach` | `scoop --attach` stores attachment_path |
| `budget_over_flag` | spend over monthly limit → OVER |
| `qr_as_today` | TW e-invoice QR + `--as-today` → current month |
| `inbox_done_collision` | same inbox filename twice → `name` + `name-2` in done/ |
| `undo_after_confirm` | `undo --yes` soft-removes last confirm |
| `multi_currency_month_csv` | TWD + USD rows in `month --csv` |
| `scoop_attach_sealed` | `scoop --attach` against `.rrsealed` + passphrase |
| `watch_restart_picks_new` | second `watch --once` after new drop → count=2 |
| `concurrent_scoop` | two OS scoop processes → count=2, inbox empty |
| `onnx_real_photo` | *(only when `--features onnx` + models ready)* |

## Blind spots (do not trust yet)

| ID | Why |
|----|-----|
| `onnx_real_photo` | Needs `--features onnx` + fetched models (conditional probe when ready) |
| `watch_daemon_crash` | In-loop crash mid-process (restart-between-runs is measured) |
| `mobile_frb` | Flutter/FRB daily path out of CLI measure scope |

Update this table when probes are added or blinds are closed.
