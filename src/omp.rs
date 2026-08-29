//! OMP (<https://omp.sh/>) is an open-source fork of the Pi Coding Agent
//! (<https://pi.dev/>, source: <https://github.com/can1357/oh-my-pi>) and
//! persists sessions using the exact same tree-structured JSONL format under
//! `<dir>/agent/sessions/`. Parsing logic is fully shared with `crate::pi`;
//! this module only carries OMP-specific identifiers.
use crate::db::UsageEntry;
use std::path::{Path, PathBuf};

pub(crate) const SOURCE_KIND: &str = "omp-session";

pub(crate) fn find_session_files(dir: &Path) -> Vec<PathBuf> {
    crate::pi::find_session_files(dir)
}

pub(crate) fn parse_session_usage_file(path: &Path) -> Result<Vec<UsageEntry>, String> {
    crate::pi::parse_session_usage_file(path, SOURCE_KIND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn parse_session_usage_file_uses_omp_source_kind() {
        let root = std::env::temp_dir().join(format!(
            "token-usage-insights-omp-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"session","version":3,"id":"omp-sess-1","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/project"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"m2","parentId":null,"timestamp":"2024-12-03T14:00:02.000Z","message":{{"role":"assistant","content":[{{"type":"text","text":"Hi!"}}],"provider":"anthropic","model":"claude-sonnet-4-5","usage":{{"input":10,"output":5,"totalTokens":15,"cost":{{"total":0.0005}}}},"stopReason":"stop"}}}}"#
        )
        .unwrap();

        let entries = parse_session_usage_file(&path).unwrap();

        fs::remove_dir_all(&root).ok();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "omp-sess-1");
        assert_eq!(entries[0].source_kind.as_deref(), Some(SOURCE_KIND));
    }
}
