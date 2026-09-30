use crate::domain::usage::Entry;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Message {
    role: String,
    #[serde(rename = "modelID")]
    model_id: Option<String>,
    #[serde(default)]
    cost: f64,
    #[serde(default)]
    tokens: Tokens,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Tokens {
    input: u64,
    output: u64,
    reasoning: u64,
    cache: Cache,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Cache {
    read: u64,
    write: u64,
}

fn parse(created_ms: i64, data: &str) -> Option<Entry> {
    let m: Message = serde_json::from_str(data).ok()?;
    if m.role != "assistant" {
        return None;
    }
    let t = m.tokens;
    Some(Entry {
        time: DateTime::from_timestamp_millis(created_ms)?,
        model: m.model_id?,
        input: t.input + t.cache.read + t.cache.write,
        output: t.output + t.reasoning,
        cost: m.cost,
    })
}

/// Base absente ou illisible : aucune consommation, pas d'erreur.
pub fn load(db: &Path, since: DateTime<Utc>) -> rusqlite::Result<Vec<Entry>> {
    if !db.exists() {
        return Ok(Vec::new());
    }
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    let mut stmt = conn.prepare("SELECT time_created, data FROM message WHERE time_created >= ?1")?;
    let rows = stmt.query_map([since.timestamp_millis()], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows.filter_map(Result::ok).filter_map(|(t, d)| parse(t, &d)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASSISTANT: &str = r#"{"role":"assistant","modelID":"qwen3.6-35b-a3b","providerID":"scaleway","cost":0.12,
        "tokens":{"input":100,"output":20,"reasoning":5,"cache":{"read":1000,"write":10}}}"#;

    #[test]
    fn parses_assistant_message() {
        let e = parse(1_782_206_853_850, ASSISTANT).unwrap();
        assert_eq!(e.model, "qwen3.6-35b-a3b");
        assert_eq!((e.input, e.output), (1110, 25));
        assert_eq!(e.cost, 0.12);
        assert_eq!(e.time.timestamp_millis(), 1_782_206_853_850);
    }

    #[test]
    fn ignores_user_messages_and_garbage() {
        assert!(parse(0, r#"{"role":"user","agent":"build"}"#).is_none());
        assert!(parse(0, "not json").is_none());
    }

    #[test]
    fn missing_db_is_empty() {
        assert!(load(Path::new("/nonexistent/opencode.db"), Utc::now()).unwrap().is_empty());
    }
}
