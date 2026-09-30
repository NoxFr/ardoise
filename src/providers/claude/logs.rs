use super::models::display_name;
use super::pricing::{price, TokenUsage};
use crate::domain::usage::Entry;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Deserialize)]
struct Line {
    timestamp: Option<DateTime<Utc>>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
    cache_creation: Option<CacheCreation>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CacheCreation {
    ephemeral_5m_input_tokens: u64,
    ephemeral_1h_input_tokens: u64,
}

impl From<&Usage> for TokenUsage {
    fn from(u: &Usage) -> Self {
        let (cache_write_5m, cache_write_1h) = match &u.cache_creation {
            Some(c) if c.ephemeral_5m_input_tokens + c.ephemeral_1h_input_tokens > 0 => {
                (c.ephemeral_5m_input_tokens, c.ephemeral_1h_input_tokens)
            }
            _ => (u.cache_creation_input_tokens, 0),
        };
        TokenUsage {
            input: u.input_tokens,
            output: u.output_tokens,
            cache_write_5m,
            cache_write_1h,
            cache_read: u.cache_read_input_tokens,
        }
    }
}

fn parse_line(line: &str) -> Option<(Option<String>, Entry)> {
    let l: Line = serde_json::from_str(line).ok()?;
    let msg = l.message?;
    let usage = msg.usage?;
    let model = msg.model?;
    let pr = price(&model)?;
    let key = msg.id.map(|id| format!("{id}:{}", l.request_id.unwrap_or_default()));
    Some((
        key,
        Entry {
            time: l.timestamp?,
            model: display_name(&model),
            input: usage.input_tokens + usage.cache_creation_input_tokens + usage.cache_read_input_tokens,
            output: usage.output_tokens,
            cost: pr.cost(&(&usage).into()),
        },
    ))
}

fn collect_files(dir: &Path, since: SystemTime, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let path = e.path();
        let Ok(meta) = e.metadata() else { continue };
        if meta.is_dir() {
            collect_files(&path, since, out);
        } else if path.extension().is_some_and(|x| x == "jsonl") && meta.modified().map_or(true, |m| m >= since) {
            out.push(path);
        }
    }
}

/// Lit les transcripts sous `roots` et renvoie les messages facturés depuis `since`, dédoublonnés.
pub fn load(roots: &[PathBuf], since: DateTime<Utc>) -> Vec<Entry> {
    let mut files = Vec::new();
    for root in roots {
        collect_files(root, since.into(), &mut files);
    }
    let mut keyed: HashMap<String, Entry> = HashMap::new();
    let mut unkeyed = Vec::new();
    for f in files {
        let Ok(file) = File::open(&f) else { continue };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if !line.contains("\"usage\"") {
                continue;
            }
            match parse_line(&line) {
                Some((_, e)) if e.time < since => {}
                Some((Some(k), e)) => {
                    keyed.insert(k, e);
                }
                Some((None, e)) => unkeyed.push(e),
                None => {}
            }
        }
    }
    unkeyed.extend(keyed.into_values());
    unkeyed
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = r#"{"timestamp":"2026-09-30T06:44:52.130Z","requestId":"req_1","message":{"id":"msg_1","model":"claude-opus-5-5","usage":{"input_tokens":1000000,"output_tokens":1000000,"cache_creation_input_tokens":1000000,"cache_read_input_tokens":1000000,"cache_creation":{"ephemeral_1h_input_tokens":1000000,"ephemeral_5m_input_tokens":0}}}}"#;

    #[test]
    fn parses_and_prices_line() {
        let (key, e) = parse_line(LINE).unwrap();
        assert_eq!(key.as_deref(), Some("msg_1:req_1"));
        assert_eq!(e.model, "Opus 5.5");
        assert_eq!(e.input, 3_000_000);
        assert_eq!(e.output, 1_000_000);
        // 4 input + 20 output + 8 write 1h + 0.2 read
        assert!((e.cost - 32.2).abs() < 1e-9);
    }

    #[test]
    fn cache_write_defaults_to_5m() {
        let u = Usage { cache_creation_input_tokens: 42, ..Default::default() };
        let t = TokenUsage::from(&u);
        assert_eq!((t.cache_write_5m, t.cache_write_1h), (42, 0));
    }

    #[test]
    fn ignores_unknown_model() {
        assert!(parse_line(&LINE.replace("claude-opus-5-5", "<synthetic>")).is_none());
    }
}
