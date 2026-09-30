use super::pricing::cost;
use crate::domain::subscription::{RateLimitWindow, Subscription};
use crate::domain::usage::Entry;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(default)]
struct Usage {
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
}

impl Usage {
    fn minus(self, o: Usage) -> Usage {
        Usage {
            input_tokens: self.input_tokens.saturating_sub(o.input_tokens),
            cached_input_tokens: self.cached_input_tokens.saturating_sub(o.cached_input_tokens),
            output_tokens: self.output_tokens.saturating_sub(o.output_tokens),
        }
    }
}

/// Un rollout, ligne à ligne. Les compteurs `total_token_usage` sont cumulés et un rollout repris
/// hérite du total de son parent : on compte les différences, et seulement `last_token_usage` pour
/// le premier événement du fichier.
fn parse(lines: impl Iterator<Item = String>, since: DateTime<Utc>) -> Vec<Entry> {
    let mut model = String::from("gpt-5");
    let mut previous: Option<Usage> = None;
    let mut out = Vec::new();
    for line in lines {
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let payload = &v["payload"];
        match (v["type"].as_str(), payload["type"].as_str()) {
            (Some("turn_context"), _) => {
                if let Some(m) = payload["model"].as_str() {
                    model = m.to_string();
                }
            }
            (Some("event_msg"), Some("token_count")) => {
                let info = &payload["info"];
                let Ok(total) = Usage::deserialize(&info["total_token_usage"]) else { continue };
                let delta = match previous {
                    Some(p) if p == total => continue,
                    Some(p) => total.minus(p),
                    None => Usage::deserialize(&info["last_token_usage"]).unwrap_or(total),
                };
                previous = Some(total);
                let Some(time) = v["timestamp"].as_str().and_then(|t| t.parse::<DateTime<Utc>>().ok()) else {
                    continue;
                };
                if time < since {
                    continue;
                }
                out.push(Entry {
                    time,
                    model: model.clone(),
                    input: delta.input_tokens,
                    output: delta.output_tokens,
                    cost: cost(&model, delta.input_tokens, delta.cached_input_tokens, delta.output_tokens),
                });
            }
            _ => {}
        }
    }
    out
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RateLimitWindowRaw {
    used_percent: f64,
    window_minutes: i64,
    resets_at: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RateLimitsRaw {
    plan_type: Option<String>,
    primary: Option<RateLimitWindowRaw>,
    secondary: Option<RateLimitWindowRaw>,
}

fn window(w: Option<RateLimitWindowRaw>) -> Option<RateLimitWindow> {
    let w = w?;
    let resets_at = DateTime::from_timestamp(w.resets_at?, 0)?;
    Some(RateLimitWindow { used_percent: w.used_percent, window_minutes: w.window_minutes, resets_at })
}

/// Dernier instantané `rate_limits` rencontré dans le rollout (les événements `token_count` le
/// répètent, seule la dernière valeur compte).
fn parse_subscription(lines: impl Iterator<Item = String>) -> Option<Subscription> {
    let mut last = None;
    for line in lines {
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let payload = &v["payload"];
        if payload["type"].as_str() != Some("token_count") || payload["rate_limits"].is_null() {
            continue;
        }
        let Ok(raw) = RateLimitsRaw::deserialize(&payload["rate_limits"]) else { continue };
        last = Some(Subscription {
            plan: raw.plan_type.unwrap_or_default(),
            primary: window(raw.primary),
            secondary: window(raw.secondary),
        });
    }
    last
}

fn collect(dir: &Path, since: SystemTime, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let path = e.path();
        let Ok(meta) = e.metadata() else { continue };
        if meta.is_dir() {
            collect(&path, since, out);
        } else if path.extension().is_some_and(|x| x == "jsonl") && meta.modified().map_or(true, |m| m >= since) {
            out.push(path);
        }
    }
}

pub fn load(roots: &[PathBuf], since: DateTime<Utc>) -> Vec<Entry> {
    let mut files = Vec::new();
    for root in roots {
        collect(root, since.into(), &mut files);
    }
    files
        .iter()
        .filter_map(|f| File::open(f).ok())
        .flat_map(|f| parse(BufReader::new(f).lines().map_while(Result::ok), since))
        .collect()
}

/// Cherche l'instantané `rate_limits` le plus récent, en partant des rollouts modifiés le plus
/// récemment (les événements les plus anciens n'ont plus d'intérêt pour une jauge de quota).
pub fn subscription(roots: &[PathBuf]) -> Option<Subscription> {
    let mut files = Vec::new();
    for root in roots {
        collect(root, SystemTime::UNIX_EPOCH, &mut files);
    }
    files.sort_by_key(|f| std::cmp::Reverse(fs::metadata(f).and_then(|m| m.modified()).ok()));
    files.iter().find_map(|f| {
        let file = File::open(f).ok()?;
        parse_subscription(BufReader::new(file).lines().map_while(Result::ok))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn token_count(t: &str, total: [u64; 3], last: [u64; 3]) -> String {
        let u = |[i, c, o]: [u64; 3]| json!({ "input_tokens": i, "cached_input_tokens": c, "output_tokens": o });
        json!({ "timestamp": t, "type": "event_msg",
                "payload": { "type": "token_count", "info": { "total_token_usage": u(total), "last_token_usage": u(last) } } })
        .to_string()
    }

    fn turn(model: &str) -> String {
        json!({ "timestamp": "2026-09-30T08:00:00Z", "type": "turn_context", "payload": { "model": model } })
            .to_string()
    }

    fn since() -> DateTime<Utc> {
        "2026-09-01T00:00:00Z".parse().unwrap()
    }

    #[test]
    fn counts_deltas_of_cumulative_totals() {
        let lines = vec![
            turn("gpt-5-codex"),
            token_count("2026-09-30T08:00:01Z", [100, 0, 10], [100, 0, 10]),
            token_count("2026-09-30T08:00:02Z", [100, 0, 10], [100, 0, 10]),
            token_count("2026-09-30T08:01:00Z", [300, 50, 40], [200, 50, 30]),
        ];
        let e = parse(lines.into_iter(), since());
        assert_eq!(e.len(), 2, "le doublon est ignoré");
        assert_eq!((e[0].input, e[0].output), (100, 10));
        assert_eq!((e[1].input, e[1].output), (200, 30));
        assert_eq!(e[1].model, "gpt-5-codex");
    }

    #[test]
    fn resumed_rollout_does_not_recount_inherited_total() {
        let lines = vec![token_count("2026-09-30T09:00:00Z", [1_000_000, 0, 500_000], [1_000, 0, 50])];
        let e = parse(lines.into_iter(), since());
        assert_eq!((e[0].input, e[0].output), (1_000, 50));
    }

    #[test]
    fn follows_model_changes_and_skips_old_events() {
        let lines = vec![
            turn("gpt-5-mini"),
            token_count("2026-08-01T00:00:00Z", [10, 0, 1], [10, 0, 1]),
            token_count("2026-09-30T08:00:00Z", [1_000_010, 0, 1], [1_000_000, 0, 0]),
            turn("gpt-5-codex"),
            token_count("2026-09-30T08:05:00Z", [2_000_010, 0, 1], [1_000_000, 0, 0]),
        ];
        let e = parse(lines.into_iter(), since());
        assert_eq!(e.len(), 2);
        assert!((e[0].cost - 0.25).abs() < 1e-9, "gpt-5-mini");
        assert!((e[1].cost - 1.25).abs() < 1e-9, "gpt-5-codex");
    }

    fn rate_limits(used: f64, plan: &str) -> String {
        json!({ "timestamp": "2026-09-30T08:00:00Z", "type": "event_msg",
                "payload": { "type": "token_count", "info": null, "rate_limits": {
                    "plan_type": plan,
                    "primary": { "used_percent": used, "window_minutes": 300, "resets_at": 1_790_000_000 },
                    "secondary": null } } })
        .to_string()
    }

    #[test]
    fn keeps_the_last_rate_limits_snapshot() {
        let lines = vec![rate_limits(10.0, "plus"), "pas du json".into(), rate_limits(42.5, "pro")];
        let s = parse_subscription(lines.into_iter()).unwrap();
        assert_eq!(s.plan, "pro");
        let p = s.primary.unwrap();
        assert_eq!((p.used_percent, p.window_minutes), (42.5, 300));
        assert_eq!(p.resets_at.timestamp(), 1_790_000_000);
        assert!(s.secondary.is_none());
    }

    #[test]
    fn no_rate_limits_means_no_subscription() {
        let lines = vec![token_count("2026-09-30T08:00:01Z", [100, 0, 10], [100, 0, 10])];
        assert!(parse_subscription(lines.into_iter()).is_none());
    }

    #[test]
    fn ignores_garbage_and_other_events() {
        let lines = vec!["pas du json".into(), json!({ "type": "response_item", "payload": {} }).to_string()];
        assert!(parse(lines.into_iter(), since()).is_empty());
    }
}
