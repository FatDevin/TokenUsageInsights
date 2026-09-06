# GPT-Daybreak-Blue API 定價研究與別名對照

研究日期：2026-09-06
資料範圍：OpenAI 模型命名、官方牌價對照及使用者使用量記錄
觸發原因：Codex 使用量記錄出現 `gpt-daybreak-blue-latest` 模型名稱（session_id=01a06d15-a222-7bc2-9c0b-19ec24de2c80 turn_no=6），`pricing.csv` 缺少對應條目，導致「找不到可用的模型價格規則」錯誤。

---

## 結論

**`gpt-daybreak-blue-latest` 與 `gpt-daybreak-blue` 費率完全等同於 `gpt-5.6-sol`。**

單位為美元／100 萬 Token（Global 區域）：
- **輸入（Input）：** 每 100 萬 Token 5.00 美元
- **快取內容輸入（Cached Input）：** 每 100 萬 Token 0.50 美元
- **輸出（Output）：** 每 100 萬 Token 30.00 美元
- **批次 API（Batch API）：** N/A

---

## `pricing.csv` 採用的費率

| 模型名稱 | 部署類型 | 單位 | 輸入 | 快取輸入 | 輸出 | 批次 API |
|---|---|---|---:|---:|---:|---|
| gpt-daybreak-blue-latest | Global | 1M Tokens | 5.00 | 0.50 | 30.00 | N/A |
| gpt-daybreak-blue | Global | 1M Tokens | 5.00 | 0.50 | 30.00 | N/A |
| gpt-daybreak-blue-latest | Cursor | 1M Tokens | 5.00 | 0.50 | 30.00 | N/A |

---

## 說明

1. `gpt-daybreak-blue` 為對應 `gpt-5.6-sol` 之專案／預覽別名代號，各項 Token 費率結構與 `gpt-5.6-sol` 官方標準牌價完全一致。
2. 同時收錄 `gpt-daybreak-blue-latest` 與基礎前綴 `gpt-daybreak-blue`，使大小寫與包含後綴（如 `:cloud`、`-latest`）之呼叫皆可正確計算成本。
