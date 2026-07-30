# ReceiptRadar（發票雷達）

**拍下發票，帳本留下——不上雲。**  
*Offline receipt → ledger in seconds. Local-first. No account.*

完整產品敘事與英文 README 為主：[README.md](./README.md)。

## 一句話

手機相機拍發票／收據 → **裝置內** OCR 或電子發票 **QR 優先** 解碼 → 本機帳本。  
**無帳號、核心路徑不上雲、不上傳影像。**

## 現況

倉庫為 **Track A scaffold**（`v0.1.0-alpha`）。真實 ONNX OCR、Flutter 相機閉環、加密備份依設計文件 PR 計畫推進中。

```bash
cargo test --workspace
cargo run -p rradar-cli -- version
```

## 隱私

- 離線 flavor：可不帶 `INTERNET` 權限  
- 模型：release asset + **hash**，禁止静默下載  
- 官方 **永不** 提供 sync relay；多裝置靠加密備份／匯出  

詳見 [docs/privacy.md](./docs/privacy.md)。

## 授權

原始碼 [Apache-2.0](./LICENSE)。模型權重與第三方見後續 `THIRD_PARTY_NOTICES`。
