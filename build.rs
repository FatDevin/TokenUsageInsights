//! Embeds the date of the build's source commit so the dashboard footer never
//! shows a hand-maintained (and therefore stale) release date.

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=APP_BUILD_DATE");
    // HEAD and its reflog change on every commit/checkout, so the date refreshes
    // without requiring a clean build. Missing paths (e.g. git worktrees) are skipped.
    for path in [".git/HEAD", ".git/logs/HEAD"] {
        if Path::new(path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }

    let date = std::env::var("APP_BUILD_DATE")
        .ok()
        .filter(|value| is_iso_date(value))
        .or_else(commit_date)
        .unwrap_or_default();
    println!("cargo:rustc-env=APP_BUILD_DATE={date}");
}

fn commit_date() -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cs"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let date = String::from_utf8(output.stdout).ok()?.trim().to_string();
    is_iso_date(&date).then_some(date)
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}
