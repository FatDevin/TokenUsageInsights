use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

// `db` and `paths` are the same source files main.rs uses; only a handful of
// their exports are needed here, so silence dead-code warnings for the rest
// instead of forking a trimmed-down copy of shared logic.
#[path = "../db.rs"]
#[allow(dead_code)]
mod db;
#[path = "../grok.rs"]
#[allow(dead_code)]
mod grok;
#[path = "../paths.rs"]
#[allow(dead_code)]
mod paths;
#[path = "../vscode.rs"]
#[allow(dead_code)]
mod vscode;

const EXPORT_VERSION: u8 = 1;
const HELP_TEXT: &str = r#"Token 使用量 CLI 匯入 / 匯出工具

用途:
  export  匯出指定日、月或年的資料為 JSON（可重複匯入且支援重複資料去重）
  import  匯入 JSON 檔內的所有資料（每筆資料依 timestamp 決定日期）

共用參數:
  --agent <name>      助理名稱: antigravity / copilot / codex / claude / cursor / grok
                     亦可使用 claude-code / claude_code / claudecode（會正規化為 claude）

匯出:
  token-usage-insights-cli export --agent <name> --date YYYY[-MM[-DD]] --out <path>
  例如:
  token-usage-insights-cli export --agent codex --date 2026-07-09 --out daily.json

匯入:
  token-usage-insights-cli import --agent <name> --file <path>
  例如:
  token-usage-insights-cli import --agent codex --file daily.json

注意:
  - 若未指定 export 的 --out，會直接輸出到 stdout
  - import 檔案若含 assistant，必須與 --agent 正規化後一致，否則阻止匯入
  - import 會以 `assistant_type + import_source_id` 做資料去重，重複匯入只會插入一次
  - 每次 import 都會建立可追蹤、可由看板撤銷的匯入批次
"#;

#[derive(Serialize)]
struct UsageDayExportPayload {
    version: u8,
    assistant: String,
    date: String,
    exported_at: String,
    records: Vec<db::UsageDayExportRecord>,
}

#[derive(Deserialize)]
struct UsageDayImportPayload {
    // Kept for schema parity with the exported JSON; not read during import
    // (import always re-derives these from the current run, not the file).
    #[allow(dead_code)]
    #[serde(default)]
    version: Option<u8>,
    #[serde(default)]
    assistant: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    exported_at: Option<String>,
    #[serde(default)]
    records: Vec<db::UsageDayExportRecord>,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return 1;
    }

    match args[1].as_str() {
        "export" => run_export(&args[2..]),
        "import" => run_import(&args[2..]),
        "-h" | "--help" | "help" => {
            print_help();
            0
        }
        _ => {
            eprintln!("未知指令：{}", args[1]);
            print_help();
            2
        }
    }
}

fn run_export(args: &[String]) -> i32 {
    if has_help(args) {
        print_export_help();
        return 0;
    }

    let mut assistant = None::<String>;
    let mut date = None::<String>;
    let mut out_path = None::<String>;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--agent" => {
                assistant = Some(next_flag_value(args, &mut i, "agent"));
            }
            "--date" => {
                date = Some(next_flag_value(args, &mut i, "date"));
            }
            "--out" => {
                out_path = Some(next_flag_value(args, &mut i, "out"));
            }
            arg => {
                eprintln!("未知參數: {arg}");
                return 2;
            }
        }
        i += 1;
    }

    let assistant = match assistant {
        Some(v) => normalize_assistant_name(&v),
        None => {
            eprintln!("缺少 --agent");
            return 2;
        }
    };

    let date = match date {
        Some(v) => v,
        None => {
            eprintln!("缺少 --date");
            return 2;
        }
    };

    if !is_supported_assistant(&assistant) {
        eprintln!("不支援的助理類型: {assistant}");
        return 2;
    }

    if !is_valid_period(&date) {
        eprintln!("資料範圍格式不正確，請使用 YYYY、YYYY-MM 或 YYYY-MM-DD");
        return 2;
    }

    let conn = match db::get_db_conn() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("開啟資料庫失敗: {err}");
            return 1;
        }
    };

    if let Err(err) = db::init_db(&conn) {
        eprintln!("初始化資料庫失敗: {err}");
        return 1;
    }

    let records = match db::export_usage_period_entries(&conn, &assistant, &date) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("匯出資料失敗: {err}");
            return 1;
        }
    };

    if records.is_empty() {
        eprintln!("指定日期沒有可匯出的資料");
        return 1;
    }

    let payload = UsageDayExportPayload {
        version: EXPORT_VERSION,
        assistant: assistant.clone(),
        date: date.clone(),
        exported_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        records,
    };

    let json = match serde_json::to_string_pretty(&payload) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("產生匯出 JSON 失敗: {err}");
            return 1;
        }
    };

    match out_path {
        Some(out) => {
            if let Err(err) = fs::write(PathBuf::from(&out), json) {
                eprintln!("寫入檔案失敗 {out}: {err}");
                return 1;
            }
            println!("已匯出 {} 筆到 {out}", payload.records.len());
        }
        None => {
            println!("{json}");
        }
    }

    0
}

fn run_import(args: &[String]) -> i32 {
    if has_help(args) {
        print_import_help();
        return 0;
    }

    let mut assistant = None::<String>;
    let mut date = None::<String>;
    let mut file_path = None::<String>;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--agent" => {
                assistant = Some(next_flag_value(args, &mut i, "agent"));
            }
            "--date" => {
                date = Some(next_flag_value(args, &mut i, "date"));
            }
            "--file" => {
                file_path = Some(next_flag_value(args, &mut i, "file"));
            }
            arg => {
                eprintln!("未知參數: {arg}");
                return 2;
            }
        }
        i += 1;
    }

    let assistant = match assistant {
        Some(v) => normalize_assistant_name(&v),
        None => {
            eprintln!("缺少 --agent");
            return 2;
        }
    };

    if !is_supported_assistant(&assistant) {
        eprintln!("不支援的助理類型: {assistant}");
        return 2;
    }

    let file_path = match file_path {
        Some(v) => PathBuf::from(v),
        None => {
            eprintln!("缺少 --file");
            return 2;
        }
    };

    if !file_path.exists() {
        eprintln!("找不到檔案: {:?}", file_path);
        return 1;
    }

    let input = match fs::read_to_string(&file_path) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("讀取匯入檔案失敗: {err}");
            return 1;
        }
    };

    let payload: UsageDayImportPayload = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("解析 JSON 失敗: {err}");
            return 1;
        }
    };

    let imported_from = date
        .or(payload.date)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "all".to_string());

    let source_assistant =
        match validate_import_source_assistant(&assistant, payload.assistant.as_deref()) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("{error}");
                return 2;
            }
        };

    if payload.records.is_empty() {
        eprintln!("匯入檔案沒有 records");
        return 2;
    }

    let mut conn = match db::get_db_conn() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("開啟資料庫失敗: {err}");
            return 1;
        }
    };

    if let Err(err) = db::init_db(&conn) {
        eprintln!("初始化資料庫失敗: {err}");
        return 1;
    }

    let summary = match db::import_usage_day_entries(
        &mut conn,
        &assistant,
        &imported_from,
        payload.records,
        db::UsageImportMetadata {
            source_assistant,
            source_file_name: file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string),
        },
    ) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("匯入失敗: {err}");
            return 1;
        }
    };

    match serde_json::to_string_pretty(&summary) {
        Ok(out) => println!("{out}"),
        Err(err) => {
            eprintln!("輸出匯入結果失敗: {err}");
            return 1;
        }
    }

    0
}

fn next_flag_value(args: &[String], i: &mut usize, flag: &str) -> String {
    match args.get(*i + 1) {
        Some(value) => {
            if value.starts_with("--") {
                eprintln!("缺少 --{flag} 的值");
                std::process::exit(2);
            }
            *i += 1;
            value.clone()
        }
        None => {
            eprintln!("缺少 --{flag} 的值");
            std::process::exit(2);
        }
    }
}

fn normalize_assistant_name(assistant: &str) -> String {
    let normalized = assistant.trim().to_lowercase();
    match normalized.as_str() {
        "claude-code" | "claude_code" | "claudecode" => "claude".to_string(),
        "cursor" => "cursor".to_string(),
        "grok-build" | "grok_build" | "grokbuild" => "grok".to_string(),
        _ => normalized,
    }
}

fn validate_import_source_assistant(
    target_assistant: &str,
    payload_assistant: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(payload_assistant) = payload_assistant else {
        return Ok(None);
    };
    let payload_assistant = normalize_assistant_name(payload_assistant);
    if payload_assistant != target_assistant {
        return Err(format!(
            "匯入已取消：檔案內 assistant={payload_assistant}，但 --agent 指定為 {target_assistant}。"
        ));
    }
    Ok(Some(payload_assistant))
}

fn is_supported_assistant(assistant: &str) -> bool {
    matches!(
        normalize_assistant_name(assistant).as_str(),
        "antigravity" | "copilot" | "codex" | "claude" | "cursor" | "grok"
    )
}

fn is_valid_date(date: &str) -> bool {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let year = match parts[0].parse::<i32>() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let month = match parts[1].parse::<i32>() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let day = match parts[2].parse::<i32>() {
        Ok(v) => v,
        Err(_) => return false,
    };
    if year <= 0 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }
    true
}

fn is_valid_period(period: &str) -> bool {
    match period.len() {
        4 => period.parse::<i32>().is_ok_and(|year| year > 0),
        7 => {
            let Some((year, month)) = period.split_once('-') else {
                return false;
            };
            year.parse::<i32>().is_ok_and(|year| year > 0)
                && month
                    .parse::<i32>()
                    .is_ok_and(|month| (1..=12).contains(&month))
        }
        10 => is_valid_date(period),
        _ => false,
    }
}

fn print_help() {
    println!("{HELP_TEXT}");
}

fn print_export_help() {
    println!(
        r#"export usage:
  token-usage-insights-cli export --agent <name> --date YYYY[-MM[-DD]] --out <path>

參數:
  --agent <name>    助理名稱（antigravity/copilot/codex/claude/cursor/grok）
  --date <period>     匯出年份、月份或日期
  --out <path>      輸出檔案路徑，不指定則輸出到 stdout
  --help, -h        顯示此說明
"#
    );
}

fn print_import_help() {
    println!(
        r#"import usage:
  token-usage-insights-cli import --agent <name> --file <path>

參數:
  --agent <name>      助理名稱（antigravity/copilot/codex/claude/cursor/grok）
  --file <path>       匯入檔案
  --date <label>       相容舊版，僅作為匯入紀錄標籤，不影響資料日期
  --help, -h          顯示此說明
"#
    );
}

fn has_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::validate_import_source_assistant;

    #[test]
    fn import_source_assistant_must_match_cli_target() {
        let error = validate_import_source_assistant("antigravity", Some("codex")).unwrap_err();
        assert!(error.contains("匯入已取消"));
        assert!(error.contains("assistant=codex"));
        assert!(error.contains("--agent 指定為 antigravity"));
    }

    #[test]
    fn import_source_assistant_accepts_alias_and_legacy_file() {
        assert_eq!(
            validate_import_source_assistant("claude", Some("claude-code")).unwrap(),
            Some("claude".to_string())
        );
        assert_eq!(
            validate_import_source_assistant("codex", None).unwrap(),
            None
        );
    }
}
