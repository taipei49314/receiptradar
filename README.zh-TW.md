# ReceiptRadar（發票雷達）

**拍下發票，帳本留下——不上雲。**

> **CLI 產品已完成**：可日常本機記帳（mock OCR + 文字/QR）。真 ONNX 與手機相機為後續層。

完整說明見 [README.md](./README.md) 與 [docs/cli.md](./docs/cli.md)。

```bash
cargo install --path crates/rradar-cli
rradar init
rradar process fixtures/text/familymart_89.txt --confirm --explain
rradar list
rradar stats
```

預設帳本：`%APPDATA%\receiptradar\ledger.db`
