use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::path::{Component, Path};

/// Read per-response model identities without depending on the current CLI settings.
pub fn response_models(root: &Path, session_id: &str) -> rusqlite::Result<HashMap<u32, String>> {
    let mut components = Path::new(session_id).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Ok(HashMap::new());
    }
    let path = root.join("conversations").join(format!("{session_id}.db"));
    let conn = match Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn,
        Err(_) if matches!(path.try_exists(), Ok(false)) => return Ok(HashMap::new()),
        Err(error) => return Err(error),
    };
    read_response_models(&conn)
}

fn read_response_models(conn: &Connection) -> rusqlite::Result<HashMap<u32, String>> {
    // Older/newer schemas may not expose attribution. Actual read errors (including
    // busy or corrupt databases) must propagate so sync does not cache a downgrade.
    for (table, required) in [
        ("gen_metadata", &["data"][..]),
        ("steps", &["idx", "metadata"][..]),
    ] {
        let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if required
            .iter()
            .any(|name| !columns.iter().any(|column| column == name))
        {
            return Ok(HashMap::new());
        }
    }
    // Observed in Antigravity CLI's SQLite data and embedded protobuf descriptors:
    // gen_metadata.data: field 4 = generation UUID, field 1.19 = model UID.
    // steps.metadata: field 12 = generation UUID. The steps.idx is the transcript
    // step_index; gen_metadata.idx is a different index and must not be joined to it.
    let mut generations: HashMap<Vec<u8>, Option<String>> = HashMap::new();
    let mut statement = conn.prepare("SELECT data FROM gen_metadata")?;
    let rows = statement.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
    for row in rows {
        let blob = row?;
        let Some(fields) = protobuf_fields(&blob) else {
            continue;
        };
        let Some(id) = bytes_field(&fields, 4).filter(|id| !id.is_empty()) else {
            continue;
        };
        let Some(inner) = bytes_field(&fields, 1).and_then(protobuf_fields) else {
            continue;
        };
        let Some(uid) = bytes_field(&inner, 19).and_then(|v| std::str::from_utf8(v).ok()) else {
            continue;
        };
        // Preserve unknown identifiers, but never expose arbitrary binary payloads.
        if uid.is_empty()
            || uid.len() > 200
            || !uid
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
        {
            continue;
        }
        let model = model_label(uid).to_owned();
        generations
            .entry(id.to_vec())
            .and_modify(|previous| {
                if previous.as_ref() != Some(&model) {
                    *previous = None;
                }
            })
            .or_insert(Some(model));
    }
    let mut result = HashMap::new();
    let mut statement =
        conn.prepare("SELECT idx, metadata FROM steps WHERE metadata IS NOT NULL")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    for row in rows {
        let (index, blob) = row?;
        let Ok(index) = u32::try_from(index) else {
            continue;
        };
        let Some(fields) = protobuf_fields(&blob) else {
            continue;
        };
        if let Some(Some(model)) = bytes_field(&fields, 12).and_then(|id| generations.get(id)) {
            result.insert(index, model.clone());
        }
    }
    Ok(result)
}

fn model_label(uid: &str) -> &str {
    // Verified with `agy models`; keep aliases explicit so unknown future models
    // cannot silently inherit an unrelated model's price.
    match uid {
        "agy-demo-medium" | "gemini-4-argon-medium" => "Gemini 4 Argon (Medium)",
        "agy-demo-high" | "gemini-4-argon-high" => "Gemini 4 Argon (High)",
        "agy-demo-low" | "gemini-4-argon-low" => "Gemini 4 Argon (Low)",
        "gemini-3.8-flash-high" => "Gemini 3.8 Flash (High)",
        "gemini-3.8-flash-medium" => "Gemini 3.8 Flash (Medium)",
        "gemini-3.8-flash-low" => "Gemini 3.8 Flash (Low)",
        "gemini-flash-demo" => "Gemini 3.7 Flash",
        "gemini-3.8-flash-demo" => "Gemini 3.8 Flash",
        _ => uid,
    }
}

fn bytes_field<'a>(fields: &[(u64, &'a [u8])], number: u64) -> Option<&'a [u8]> {
    let mut matching = fields.iter().filter(|(field, _)| *field == number);
    let value = matching.next()?.1;
    // An ambiguous identity is safer to leave unresolved.
    if matching.next().is_some() {
        None
    } else {
        Some(value)
    }
}

fn varint(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let byte = *bytes.get(*offset)?;
        *offset += 1;
        if shift == 63 && byte > 1 {
            return None;
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

/// Extract length-delimited fields, validating and skipping other wire types.
/// There is no recursive decoding of untrusted data or allocation from lengths.
fn protobuf_fields(bytes: &[u8]) -> Option<Vec<(u64, &[u8])>> {
    let mut offset = 0;
    let mut result = Vec::new();
    while offset < bytes.len() {
        let tag = varint(bytes, &mut offset)?;
        let field = tag >> 3;
        if field == 0 || field > 0x1fff_ffff {
            return None;
        }
        let length = match tag & 7 {
            0 => {
                varint(bytes, &mut offset)?;
                continue;
            }
            1 => 8,
            2 => usize::try_from(varint(bytes, &mut offset)?).ok()?,
            5 => 4,
            _ => return None,
        };
        let end = offset.checked_add(length)?;
        let value = bytes.get(offset..end)?;
        if tag & 7 == 2 {
            result.push((field, value));
        }
        offset = end;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_varint(mut value: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        while value >= 128 {
            bytes.push((value as u8 & 127) | 128);
            value >>= 7;
        }
        bytes.push(value as u8);
        bytes
    }

    fn field(number: u64, value: &[u8]) -> Vec<u8> {
        let mut bytes = encode_varint((number << 3) | 2);
        bytes.extend(encode_varint(value.len() as u64));
        bytes.extend(value);
        bytes
    }

    #[test]
    fn joins_generation_uuid_instead_of_unrelated_row_indices() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE gen_metadata(idx INTEGER, data BLOB); CREATE TABLE steps(idx INTEGER, metadata BLOB);").unwrap();
        for (index, uid) in [(1, "agy-demo-medium"), (2, "new-unknown-model")] {
            let id = format!("generation-{index}");
            let mut blob = field(4, id.as_bytes());
            blob.extend(field(1, &field(19, uid.as_bytes())));
            conn.execute(
                "INSERT INTO gen_metadata VALUES (?1, ?2)",
                rusqlite::params![index, blob],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO steps VALUES (?1, ?2)",
                rusqlite::params![index * 10, field(12, id.as_bytes())],
            )
            .unwrap();
        }
        let models = read_response_models(&conn).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[&10], "Gemini 4 Argon (Medium)");
        assert_eq!(models[&20], "new-unknown-model");
        assert!(!models.contains_key(&1));
    }

    #[test]
    fn malformed_protobuf_is_rejected_without_panicking() {
        for bytes in [
            vec![0],
            vec![0x0a, 10, 1],
            vec![0x08, 0x80],
            vec![0xff; 20],
            vec![0x0f],
            vec![0x09, 0],
        ] {
            assert!(protobuf_fields(&bytes).is_none());
        }
        let mut duplicate = field(4, b"one");
        duplicate.extend(field(4, b"two"));
        assert!(bytes_field(&protobuf_fields(&duplicate).unwrap(), 4).is_none());
    }

    #[test]
    fn rejects_session_path_traversal() {
        assert!(response_models(Path::new("."), "../invalid-session")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn missing_database_does_not_get_created() {
        let root = std::env::temp_dir().join(format!(
            "antigravity-metadata-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(response_models(&root, "missing-session")
            .unwrap()
            .is_empty());
        assert!(!root.exists());
    }

    #[test]
    fn unsupported_schema_has_no_model_attribution() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE steps(idx INTEGER);")
            .unwrap();
        assert!(read_response_models(&conn).unwrap().is_empty());
    }

    #[test]
    fn unreadable_database_is_an_error_instead_of_unknown_models() {
        let root = std::env::temp_dir().join(format!(
            "antigravity-metadata-invalid-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let directory = root.join("conversations");
        std::fs::create_dir_all(&directory).unwrap();
        let database = directory.join("invalid.db");
        std::fs::write(&database, b"This is deliberately not a SQLite database.").unwrap();
        let result = response_models(&root, "invalid");
        std::fs::remove_file(database).unwrap();
        std::fs::remove_dir(directory).unwrap();
        std::fs::remove_dir(root).unwrap();
        assert!(result.is_err());
    }
}
