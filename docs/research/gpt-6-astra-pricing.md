# GPT-6 Astra API 定價研究

研究日期：2026-09-06
資料範圍：OpenAI 官方發布與定價文件
觸發原因：Codex 使用量記錄出現 `gpt-6-astra` 模型名稱（session_id=01a074ab-f530-7cf1-b65f-d2e52a330258 turn_no=518），`pricing.csv` 缺少對應條目，導致「找不到可用的模型價格規則」錯誤。

---

## 結論

**GPT-6 Astra 於 2026-09-03 發布，為 OpenAI 針對 Agentic 與電腦操作場景設計的新一代 Frontier 模型。其官方 API 定價依 Prompt 上下文長度（以 272,000 Token 為門檻）分為兩個層級：**

1. **短上下文（Prompt ≤ 272K Tokens）：**
   - 輸入：每 100 萬 Token 10.00 美元
   - 快取內容輸入（Cached Input）：每 100 萬 Token 1.00 美元
   - 輸出：每 100 萬 Token 50.00 美元
   - 批次 API（Batch / Flex，50% 牌價）：5.00 / 0.50 / 25.00 美元
2. **長上下文（Prompt > 272K Tokens）：**
   - 輸入：每 100 萬 Token 20.00 美元
   - 快取內容輸入（Cached Input）：每 100 萬 Token 2.00 美元
   - 輸出：每 100 萬 Token 75.00 美元
   - 批次 API（Batch / Flex，50% 牌價）：10.00 / 1.00 / 37.50 美元

---

## 官方價格欄位

單位為美元／100 萬 Token（Global 區域）：

| 服務層級 | 適用條件 | 輸入 | 快取內容輸入 | 輸出 | 批次 API |
|---|---|---:|---:|---:|---|
| Standard（≤ 272K） | Prompt ≤ 272,000 Tokens | 10.00 | 1.00 | 50.00 | 5.00/0.50/25.00 |
| Long Context（> 272K） | Prompt > 272,000 Tokens | 20.00 | 2.00 | 75.00 | 10.00/1.00/37.50 |

---

## `pricing.csv` 採用的費率與理由

依循本儲存庫既有門檻模型結構（如 `GPT-5.5 (<272k)` / `GPT-5.5 (>272k)` 與 `GPT-5.4`）：

| 模型名稱 | 部署類型 | 單位 | 輸入 | 快取輸入 | 輸出 | 批次 API |
|---|---|---|---:|---:|---:|---|
| GPT-6 Astra (<272k) | Global | 1M Tokens | 10.00 | 1.00 | 50.00 | 5.00/0.50/25.00 |
| GPT-6 Astra (>272k) | Global | 1M Tokens | 20.00 | 2.00 | 75.00 | 10.00/1.00/37.50 |
| GPT-6 Astra | Global | 1M Tokens | 10.00 | 1.00 | 50.00 | 5.00/0.50/25.00 |
| gpt-6-astra | Cursor | 1M Tokens | 10.00 | 1.00 | 50.00 | N/A |

理由：

1. OpenAI 對長上下文採非邊際階梯定價（一旦 Prompt 超過 272,000 Tokens，整筆請求依高階梯費率計價）。`src/pricing.rs` 的 `parse_threshold_rule` 完整支援 `(<272k)` 與 `(>272k)` 標籤解析，可在不同長度下自動套用正確階梯。
2. 同時保留 `GPT-6 Astra` 預設規則（標準費率），確保無門檻標籤的查詢或回退皆能有對應牌價。
3. Cursor 清單加入 `gpt-6-astra`，方便前端模型費率表搜尋及 Cursor Agent 的潛在調用。

---

## 資料適用界線

- 上下文窗口為 1,050,000 Token；輸出上限為 128,000 Token。
- 快取寫入費用為 12.50 美元（≤ 272K）／25.00 美元（> 272K），目前未納入非 Claude 模型的單價計算。
- 若使用 Regional 端點可能有約 10% 加成；Fast Mode 費率為 Standard 的 2 倍，目前不納入標準牌價。
