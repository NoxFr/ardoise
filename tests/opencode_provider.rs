use ardoise::providers::opencode::OpenCode;
use ardoise::providers::Provider;
use chrono::{Duration, Utc};
use rusqlite::Connection;
use tempfile::TempDir;

fn db_with(messages: &[(i64, serde_json::Value)]) -> (TempDir, OpenCode) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("opencode.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, time_created INTEGER NOT NULL,
         time_updated INTEGER NOT NULL, data TEXT NOT NULL);",
    )
    .unwrap();
    for (i, (t, data)) in messages.iter().enumerate() {
        conn.execute("INSERT INTO message VALUES (?1, 's', ?2, ?2, ?3)", (format!("m{i}"), t, data.to_string()))
            .unwrap();
    }
    (dir, OpenCode::with_db(path))
}

fn assistant(model: &str, provider: &str, cost: f64) -> serde_json::Value {
    serde_json::json!({
        "role": "assistant", "modelID": model, "providerID": provider, "cost": cost,
        "tokens": { "input": 10, "output": 5, "reasoning": 1, "cache": { "read": 100, "write": 0 } }
    })
}

#[test]
fn reads_assistant_messages_since_date() {
    let now = Utc::now();
    let ms = |d: Duration| (now - d).timestamp_millis();
    let (_dir, provider) = db_with(&[
        (ms(Duration::hours(1)), assistant("qwen3.6-35b-a3b", "scaleway", 0.25)),
        (ms(Duration::hours(2)), serde_json::json!({ "role": "user", "agent": "build" })),
        (ms(Duration::days(40)), assistant("qwen3.6-35b-a3b", "scaleway", 9.0)),
    ]);

    let entries = provider.load(now - Duration::days(30));

    assert_eq!(entries.len(), 1);
    let e = &entries[0];
    assert_eq!((e.model.as_str(), e.cost), ("qwen3.6-35b-a3b", 0.25));
    assert_eq!((e.input, e.output), (110, 6));
}

#[test]
fn missing_database_yields_nothing() {
    let dir = TempDir::new().unwrap();
    assert!(OpenCode::with_db(dir.path().join("absent.db")).load(Utc::now()).is_empty());
}

#[test]
fn does_not_modify_the_database() {
    let (dir, provider) = db_with(&[(Utc::now().timestamp_millis(), assistant("m", "p", 1.0))]);
    let path = dir.path().join("opencode.db");
    let before = std::fs::read(&path).unwrap();

    provider.load(Utc::now() - Duration::days(1));

    assert_eq!(std::fs::read(&path).unwrap(), before);
}
