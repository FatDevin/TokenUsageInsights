# Gemini 3.6 Flash API 定價研究

研究日期：2026-07-28
資料範圍：Google Gemini Developer API 官方文件
官方定價頁更新日期：2026-07-21 UTC

* * *

## 結論

**Gemini 3.6 Flash 的付費標準 API 單價為：輸入每 100 萬 Token 1.50 美元、快取內容輸入每 100 萬 Token 0.15 美元、輸出每 100 萬 Token 7.50 美元。輸出單價包含思考 Token。**

**Gemini 3.6 Flash 的 High 與 Low 不是 Google 公開的不同模型 ID，而是同一個 `gemini-3.6-flash` 模型採用不同 `thinking_level`。兩者應共用相同單位費率。**

High 與 Low 的實際總費用仍可能不同，因為思考層級會影響產生的思考 Token 數量；差異來自計費 Token 數量，不是單位費率。

官方來源：

- [Gemini Developer API pricing](https://ai.google.dev/gemini-api/docs/pricing)
- [Gemini thinking](https://ai.google.dev/gemini-api/docs/thinking)
- [Gemini 3.6 Flash 模型頁](https://ai.google.dev/gemini-api/docs/models/gemini-3.6-flash)
- [Using the latest Gemini models](https://ai.google.dev/gemini-api/docs/latest-model)

* * *

## 官方價格欄位

下表均為 Google 官方公布的付費方案牌價，單位為美元／100 萬 Token：

| 服務層級 | 輸入 | 快取內容輸入 | 輸出，含思考 Token | 快取儲存 |
|---|---:|---:|---:|---:|
| Standard | 1.50 | 0.15 | 7.50 | 1.00／100 萬 Token／小時 |
| Batch | 0.75 | 0.075 | 3.75 | 1.00／100 萬 Token／小時 |
| Flex | 0.75 | 0.075 | 3.75 | 1.00／100 萬 Token／小時 |
| Priority | 2.70 | 0.27 | 13.50 | 1.00／100 萬 Token／小時 |

官方定價頁將 Standard 的輸出欄明確標為包含思考 Token；Batch、Flex 與 Priority 亦採相同標示。官方思考功能文件另說明，啟用思考時，回應費用按輸出 Token 與思考 Token 的總和計算。

`pricing.csv` 現有欄位可對應如下：

| `pricing.csv` 欄位 | 官方欄位 | Gemini 3.6 Flash 建議值 |
|---|---|---:|
| 輸入價格 | Standard input price | 1.50 |
| 快取輸入價格 | Standard context caching price | 0.15 |
| 輸出價格 | Standard output price including thinking tokens | 7.50 |
| 批次 API 價格 | Batch input／context caching／output | `0.75/0.075/3.75` |

依目前檔案格式，研究結果已加入下列資料列：

```csv
Gemini 3.6 Flash,Google AI,1M Tokens,1.50,0.15,7.50,0.75/0.075/3.75
Gemini 3.6 Flash (Medium),Google AI,1M Tokens,1.50,0.15,7.50,0.75/0.075/3.75
Gemini 3.6 Flash (High),Google AI,1M Tokens,1.50,0.15,7.50,0.75/0.075/3.75
Gemini 3.6 Flash (Low),Google AI,1M Tokens,1.50,0.15,7.50,0.75/0.075/3.75
```

* * *

## High、Low 與基礎模型的關係

Google 官方模型頁只列出一個穩定模型 ID：

```text
gemini-3.6-flash
```

官方思考功能文件則把思考強度定義為請求參數 `thinking_level`。Gemini 3.6 Flash 支援：

```text
minimal
low
medium
high
```

預設值是 `medium`。官方 API 範例使用相同的 `gemini-3.6-flash` 模型 ID，並在 `generation_config` 中設定 `thinking_level: "low"`，沒有切換成另一個模型 ID。

因此可驗證地判定：

- `Gemini 3.6 Flash (Low)` 是 `gemini-3.6-flash` 搭配 Low 思考層級的顯示名稱。
- `Gemini 3.6 Flash (High)` 是 `gemini-3.6-flash` 搭配 High 思考層級的顯示名稱。
- Google 定價頁只對 `gemini-3.6-flash` 公布一組費率，沒有依思考層級另列單價。
- 思考 Token 按輸出費率計費，所以 High 可能因產生較多思考 Token 而有較高總費用。

**在 `pricing.csv` 以完整顯示名稱建立多筆規則是模型名稱比對需求，不代表 Google 提供多個不同費率的模型。**

* * *

## 適用門檻與限制

### Token 數量門檻

官方 Gemini Developer API 定價頁沒有為 Gemini 3.6 Flash 設定依提示長度或情境長度分級的價格門檻。與部分模型的長情境分段計價不同，Gemini 3.6 Flash 只有每個服務層級各一組 Token 單價。

官方模型頁列出的技術上限為：

- 輸入上限：1,048,576 Token
- 輸出上限：65,536 Token

這些是模型容量上限，不是價格分級門檻，因此不需要建立 `<272k` 或 `>272k` 規則。

### 免費方案與付費估價

官方定價頁顯示 Standard 免費方案的輸入、輸出與快取內容輸入可免費使用，但有模型與速率限制。`pricing.csv` 用途是估算 API 成本，因此應採付費 Standard 牌價，不應把模型價格設為零。

Google 另註明 Google AI Studio 在支援地區可免費使用。這不等於所有透過 Gemini Developer API 或代理程式產生的 Token 都是零成本。

### 目前 CSV 無法完整表達的費用

現有 `pricing.csv` Token 單價欄位無法表達下列額外費用：

- 快取儲存費：每 100 萬 Token 每小時 1.00 美元。
- Google Search grounding：Gemini 3 系列每月共用 5,000 次免費額度後，每 1,000 個搜尋查詢 14 美元。
- Google Maps grounding：每月共用 5,000 次免費額度後，每 1,000 個搜尋查詢 14 美元。
- Priority 服務層級相對 Standard 的加價。

因此加入 Token 單價規則後，能解決一般 Standard Token 成本估算，但不代表涵蓋所有工具、快取儲存與服務層級費用。

* * *

## 不確定性與資料界線

- 本研究只採用 Google Gemini Developer API 官方牌價，不把 Vertex AI、Gemini Enterprise Agent Platform、第三方轉售服務或訂閱方案費率混入。
- Google 官方文件沒有把 `Gemini 3.6 Flash (High)` 或 `Gemini 3.6 Flash (Low)` 當成獨立模型名稱；這兩個字串是上層產品的顯示名稱。共用費率的判定是由官方單一模型 ID、`thinking_level` 請求參數及單一模型定價表共同推導。
- 若上層資料來源的 `output` 欄位沒有包含思考 Token，僅加入價格規則仍會低估費用。Google 官方 API 會另外提供思考 Token 用量欄位，成本計算必須確保將其納入按輸出單價計價的 Token 數量。
- Google 定價可能變動；以上結論是 2026-07-28 查核結果，官方定價頁顯示最後更新日期為 2026-07-21 UTC。
