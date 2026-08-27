# Gemini 3.7 Flash API 定價研究

研究日期：2026-08-24
資料範圍：Google 官方定價文件（Gemini Developer API / Agent Platform）
觸發原因：Antigravity 使用量記錄出現 `Gemini 3.7 Flash (High)` 模型名稱，
`pricing.csv` 缺少對應條目，導致「找不到可用的模型價格規則」錯誤。

* * *

## 結論

**Gemini 3.7 Flash 的付費標準 API 單價為：輸入每 100 萬 Token 1.50 美元、
快取內容輸入每 100 萬 Token 0.15 美元、輸出每 100 萬 Token 7.50 美元。
輸出單價包含思考 Token。**

**Gemini 3.7 Flash 的 High、Medium 與 Low 不是 Google 公開的不同模型 ID，
而是同一個 `gemini-3.7-flash` 模型採用不同 `thinking_level`
（官方支援 low／medium／high）。三者應共用相同單位費率。**

High 與 Low 的實際總費用仍可能不同，因為思考層級會影響產生的思考
Token 數量；差異來自計費 Token 數量，不是單位費率。

官方來源：

- [Agent Platform Pricing | Google Cloud](https://cloud.google.com/gemini-enterprise-agent-platform/generative-ai/pricing)
- [Gemini 3.7 Flash 模型卡 | Google DeepMind](https://deepmind.google/models/model-cards/gemini-3-7-flash/)

* * *

## 官方價格欄位

Gemini 3.7 Flash 於 2026-08-13 發布，採用限時優惠價，並已公告調漲時程。
單位為美元／100 萬 Token（Global 區域）：

| 服務層級 | 適用期間 | 輸入 | 快取內容輸入 | 輸出，含思考 Token |
|---|---|---:|---:|---:|
| Standard（優惠價） | 至 2026-12-31 | 0.75 | 0.075 | 3.75 |
| Standard（標準價） | 2027-01-01 起 | 1.50 | 0.15 | 7.50 |
| Batch／Flex（優惠價） | 至 2026-12-31 | 0.375 | 0.0375 | 1.875 |
| Batch／Flex（標準價） | 2027-01-01 起 | 0.75 | 0.075 | 3.75 |
| Priority（標準價） | 2027-01-01 起 | 2.70 | 0.27 | 13.50 |

* * *

## `pricing.csv` 採用的費率與理由

`pricing.csv` 依循本儲存庫既有慣例（見 Gemini 3.6 Flash 的定價研究與
commit 91c280a），記錄**官方付費標準（Standard）牌價**，而非限時優惠價：

| 模型名稱 | 部署類型 | 單位 | 輸入 | 快取輸入 | 輸出 | 批次 API |
|---|---|---|---:|---:|---:|---|
| Gemini 3.7 Flash（含 Medium／High／Low 變體） | Google AI | 1M Tokens | 1.50 | 0.15 | 7.50 | 0.75/0.075/3.75 |

理由：

1. 優惠價為限時促銷（至 2026-12-31），2027-01-01 起調整為標準價。
   使用標準價可避免未來需再次更新資料，且成本估算偏保守。
2. 與 Gemini 3.6 Flash 條目的費率結構完全一致，維持表格一致性。
3. 不同思考層級（Medium／High／Low）對應同一個模型與相同單位費率，
   總費用差異來自實際產生的思考 Token 數量。

* * *

## 資料適用界線

- 價格為 Global 區域付費方案牌價；Non-global 區域自 2026-07-01 起另有
  約 10% 加成（例如 Standard 輸入 1.65 美元）。
- 若查詢輸入內容超過 200K Token，所有（輸入與輸出）Token 皆以長上下文
  費率計價。Gemini 3.7 Flash 的長上下文費率與短上下文相同（無加成）。
- 快取儲存費用為 1.00 美元／100 萬 Token／小時，未計入 `pricing.csv`。
- 模型知識截止日期：2026-03；上下文窗口 1,048,576 Token；輸出上限 65,536 Token。