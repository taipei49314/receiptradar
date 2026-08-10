# ReceiptRadar（發票雷達）

**拍下發票，帳本留下——不上雲。**

> **已完成範圍（CLI release candidate）：** 本機記帳 closed loop（process → ledger → trash/restore/purge含附件清理 → export/backup → 本機 HTTP API）。預設 mock OCR + 文字/QR；真 ONNX 為可選。
>
> **不在本次完成範圍：** `apps/mobile` Flutter shell 為 **實驗/mock** UI（`MockRradarApi`），不屬於 CLI 產品候選。

完整說明見 [README.md](./README.md) 與 [docs/cli.md](./docs/cli.md)。

```bash
cargo install --path crates/rradar-cli
rradar day                 # 30 秒台灣日常閉環（建議錄影）
rradar init
rradar add fixtures/text/familymart_89.txt --as-today --explain
rradar today
rradar list
rradar stats
```

預設帳本：`%APPDATA%\receiptradar\ledger.db`
