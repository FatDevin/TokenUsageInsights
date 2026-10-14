# Lessons

## 2026-07-10 Rust exact test filter executed zero tests

- Classification: missing verification.
- Failure mode: `cargo test <short-name> -- --exact` matched no tests because Rust's exact name includes the module path.
- Detection signal: Cargo reported `running 0 tests` with the target test filtered out.
- Prevention rule: use the fully qualified name for `--exact`, or omit `--exact`; never accept a targeted test run unless at least one test executed.
- Tripwire: check the test summary for `1 passed` or an intentional failure before treating the command as evidence.

## 2026-07-10 Cross-realm JavaScript assertion

- Classification: incorrect assumption about test behavior.
- Failure mode: `assert.deepStrictEqual` rejected structurally identical arrays created inside `vm` because their prototypes came from a different JavaScript realm.
- Detection signal: the assertion reported identical displayed structures but non-reference-equivalent realm prototypes.
- Prevention rule: normalize `vm` results into host values or compare serialized primitive projections such as session ID arrays.
- Tripwire: when a `vm` assertion displays identical actual/expected values, check realm/prototype identity before treating it as a product failure.

## 2026-07-10 Session identifiers are untrusted input

- Classification: security/privacy oversight.
- Failure mode: the first cycle-safety patch used `Map` in the tree builder but left a plain-object lookup and raw session-ID interpolation in the table renderer.
- Detection signal: independent frontend review found prototype-key collisions and unescaped values inside `innerHTML` templates.
- Prevention rule: treat every field parsed from assistant logs as untrusted; use `Map` or null-prototype dictionaries for identifier keys and escape all HTML interpolation.
- Tripwire: search changed render paths for plain `{}` identifier maps and raw `${...id}` interpolation before UI verification.

## 2026-07-10 Migration semantics require a new marker

- Classification: unsafe change scope.
- Failure mode: migration behavior changed during review while retaining the same `v3` marker, which could let an environment that ran the earlier implementation skip the corrected logic.
- Detection signal: independent data review compared the marker value with the revised migration transaction.
- Prevention rule: whenever deployed or potentially executed migration semantics change, allocate a new monotonic marker even within the same development task unless non-execution is proven.
- Tripwire: before rollout, search the database and code for the proposed marker and confirm it uniquely identifies the final transaction semantics.

## 2026-07-10 Deployment must tolerate an already-stopped service

- Classification: environment-dependent assumption.
- Failure mode: the release switch assumed port 3003 would still have the previously observed listener and aborted when the service exited before deployment began.
- Detection signal: the switch stopped at its initial listener check before making any state change.
- Prevention rule: deployment scripts must branch safely for both running and already-stopped services while preserving the same backup and rollback guarantees.
- Tripwire: re-read listener/process state at action time and treat absence as a valid start path, not an exceptional failure.

## 2026-07-10 Readiness deadlines must include pre-bind migrations

- Classification: incorrect environment/runtime assumption.
- Failure mode: the deployment wrapper treated absence of a listener after 10 seconds as startup failure even though the process could still be performing a first-run reparse before binding.
- Detection signal: release build succeeded, the new process did not report an immediate exit, rollback succeeded, and the service architecture performs sync work before opening port 3003.
- Prevention rule: capture startup logs, distinguish process exit from slow readiness, and size readiness deadlines for migration work before terminating a new binary.
- Tripwire: after any interrupted migration attempt, allocate a fresh marker before retrying so partially written sync state cannot suppress the corrected rebuild.

## 2026-07-10 Session identity depends on metadata sequence

- Classification: incorrect assumption about repository data behavior.
- Failure mode: preferring `payload.id` over `payload.session_id` was insufficient because subagent rollouts contain a later embedded parent `session_meta` that overwrote the correct child identity.
- Detection signal: post-migration transcript/session count stayed at 9 despite 45 rollout files; metadata audit showed first-event IDs matched file UUIDs while later events did not.
- Prevention rule: model the complete event sequence in parser fixtures and lock immutable file/session identity at the first valid canonical metadata event.
- Tripwire: acceptance checks must compare retained transcript count with unique session identity count after a real reparse, not only check that self-parent count is zero.

## 2026-07-10 Multi-file patches require explicit file boundaries

- Classification: tooling error.
- Failure mode: a lesson-file hunk was placed before its `Update File` header, so `apply_patch` searched for lesson text inside `src/db.rs` and rejected the entire patch.
- Detection signal: patch verification named the wrong target file for the missing lesson heading.
- Prevention rule: every multi-file patch must introduce the next `Update File` header before any hunk context from that file.
- Tripwire: if a patch reports context from the wrong file, confirm atomic rejection and correct file boundaries before retrying.

## 2026-07-10 PowerShell variables are case-insensitive

- Classification: environment-dependent tooling error.
- Failure mode: `$home` was treated as the read-only built-in `$HOME`, so the HTTP smoke command failed before making its request.
- Detection signal: PowerShell reported `Cannot overwrite variable HOME because it is read-only or constant`.
- Prevention rule: use descriptive response names such as `$homeResponse` instead of identifiers that differ from built-ins only by case.
- Tripwire: treat PowerShell variable names as case-insensitive when reviewing inline verification scripts.

## 2026-07-10 Optional remote files must not fail release discovery

- Classification: environment-dependent tooling error.
- Failure mode: a diagnostic batch treated an optional remote `tasks/todo.md` lookup as mandatory, causing the combined command to return failure despite successfully reading the release tag and version.
- Detection signal: useful v0.1.1 output was present, but the batch exited non-zero only because the remote audit file did not exist.
- Prevention rule: probe optional paths separately or explicitly normalize their absence to success; only required release artifacts may gate discovery.
- Tripwire: label every remote read as required or optional before composing parallel release checks.

## 2026-10-15 區分匯入器限制與來源資料能力

- 分類：對儲存庫行為的假設不完整。
- 問題：只檢查既有 statusline 匯入器，未先驗證原生 transcript 是否已包含 Token 欄位。
- 偵測訊號：使用者提供的 session 有逐筆 input_tokens、cache_read_tokens、output_tokens。
- 預防規則：回答是否需要額外收集設定時，分別驗證來源格式與現有匯入流程；不要將缺少匯入支援等同來源沒有資料。
- 檢查點：以不輸出對話內容的欄位統計檢查原生記錄，並確認模型、快取與費用語意。

## 2026-10-15 Windows 建置空間與修改範圍

- 分類：環境假設、工具定位與驗證不足。
- 偵測：G: 空間耗盡造成 rustc os error 112；跨磁碟移動失敗產物時有並行編譯造成檔案消失；初次搜尋 README* 未涵蓋翻譯版本。
- 預防：先檢查磁碟空間；統一 CARGO_TARGET_DIR；移動建置產物前停止所有使用同一 target 的工作；用 `rg --files -g 'README*'` 找出全部語系。
- 檢查點：所有代理共用同一建置路徑與排程，完整文件清單與空間檢查必須在最後驗證前完成。
- SQL 修改需限定所屬函式：相似 INSERT 片段不可跨函式取代；逐一核對欄位、placeholder 與 params 數量。

## 2026-10-15 原生與舊資料並存的同步邊界

- 分類：缺少錯誤與並行寫入情境驗證。
- 偵測：審查發現損壞 session 阻斷後續匯入；原生 transcript 未寫完時，新增 hook 造成重複計算；metadata 暫時讀取失敗可能覆蓋已知模型。
- 預防：以 session 隔離解析錯誤，已知原生 session 優先權不可依賴本次解析成功，暫時性 metadata 錯誤不得寫成功同步狀態。
- 檢查點：加入損壞後仍有健康 session、hook 與部分原生寫入並存、metadata 讀取失敗的回歸測試。


## 2026-10-15 多來源整合測試必須使用完整身份

- 分類：測試對現有 API 契約的假設錯誤。
- 偵測：原生來源含 source_dir_key，但測試未傳此參數，依既有 NULL 篩選契約回傳 Session not found。
- 預防：從匯出資料取得 source_kind 與 source_dir_key 一起查詢；不可為測試便宜而放寬來源隔離。
- 檢查點：設定 API 可保留舊版選用腳本命令；驗證「免設定」應檢查原生資料能匯入與 native data_path，不應假設相容欄位必須清空。

## 2026-10-15 沿用實際存在的元件樣式

- 分類：UI 驗證不足。
- 偵測：390px 瀏覽器截圖顯示原生指南長路徑使 modal 橫向溢出，新增的 code-block class 在現有 CSS 中不存在。
- 預防：沿用已定義的 code-block-container，讓長程式碼區塊自行捲動，不撐開 modal。
- 檢查點：瀏覽器驗證容器 scrollWidth 等於 clientWidth；修正後均為 361px。

## 2026-10-15 新資料來源不得漏掉工作目錄

- 分類：驗收範圍不完整。
- 偵測：原生 Antigravity Token 與模型皆正確，但使用者發現 CWD 缺失；匯入器固定寫入 None。
- 預防：新來源的驗收包含會話名稱、模型、CWD、Token、費用與時間軸，不只核對用量總數。
- 檢查點：對會話 metadata 的工作區欄位建立獨立 fixture，並確認同步快取升版後會補齊歷史資料。
