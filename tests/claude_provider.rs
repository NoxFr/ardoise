mod common;

use ardoise::providers::claude::Claude;
use ardoise::providers::Provider;
use chrono::{Duration, Utc};
use common::log_line;
use std::fs;
use tempfile::TempDir;

fn write(dir: &TempDir, rel: &str, lines: &[String]) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n")).unwrap();
}

#[test]
fn loads_sessions_and_subagents_recursively() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    write(&dir, "proj-a/s1.jsonl", &[log_line("1", "claude-opus-5-5", now, 1_000_000, 0)]);
    write(&dir, "proj-a/s1/subagents/agent-1.jsonl", &[log_line("2", "claude-haiku-4-5", now, 1_000_000, 0)]);

    let mut entries = Claude::with_roots(vec![dir.path().to_path_buf()]).load(now - Duration::days(1));
    entries.sort_by(|a, b| a.cost.total_cmp(&b.cost));

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].model, "Haiku 4.5");
    assert_eq!(entries[1].model, "Opus 5.5");
    assert!((entries[1].cost - 4.0).abs() < 1e-9);
}

#[test]
fn deduplicates_streamed_lines_keeping_the_last() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    write(
        &dir,
        "p/s.jsonl",
        &[log_line("1", "claude-sonnet-5", now, 10, 1), log_line("1", "claude-sonnet-5", now, 10, 500)],
    );
    write(&dir, "p/resumed.jsonl", &[log_line("1", "claude-sonnet-5", now, 10, 500)]);

    let entries = Claude::with_roots(vec![dir.path().to_path_buf()]).load(now - Duration::days(1));

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].output, 500);
}

#[test]
fn skips_old_invalid_and_unknown_entries() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    write(
        &dir,
        "p/s.jsonl",
        &[
            log_line("old", "claude-sonnet-5", now - Duration::days(40), 10, 1),
            log_line("synthetic", "<synthetic>", now, 10, 1),
            r#"{"type":"user","message":{"role":"user","content":"hello \"usage\""}}"#.into(),
            "not json but has \"usage\"".into(),
            log_line("ok", "claude-sonnet-5", now, 10, 1),
        ],
    );
    write(&dir, "p/notes.txt", &[log_line("txt", "claude-sonnet-5", now, 10, 1)]);

    let entries = Claude::with_roots(vec![dir.path().to_path_buf()]).load(now - Duration::days(30));

    assert_eq!(entries.len(), 1);
}

#[test]
fn missing_root_yields_nothing() {
    let dir = TempDir::new().unwrap();
    assert!(Claude::with_roots(vec![dir.path().join("absent")]).load(Utc::now()).is_empty());
}

#[test]
fn reload_picks_up_appended_lines_and_removed_files() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    let claude = Claude::with_roots(vec![dir.path().to_path_buf()]);
    write(&dir, "p/s.jsonl", &[log_line("1", "claude-sonnet-5", now, 10, 1)]);
    write(&dir, "p/other.jsonl", &[log_line("2", "claude-sonnet-5", now, 10, 1)]);
    assert_eq!(claude.load(now - Duration::days(1)).len(), 2);

    write(
        &dir,
        "p/s.jsonl",
        &[log_line("1", "claude-sonnet-5", now, 10, 1), log_line("3", "claude-sonnet-5", now, 10, 1)],
    );
    fs::remove_file(dir.path().join("p/other.jsonl")).unwrap();

    assert_eq!(claude.load(now - Duration::days(1)).len(), 2, "une ligne ajoutée, un fichier supprimé");
}
