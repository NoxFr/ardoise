//! Claude Code : transcripts JSONL de `~/.claude/projects`, coût estimé au tarif API public.

mod logs;
mod models;
mod pricing;

use super::Provider;
use super::files::FileCache;
use crate::domain::subscription::Subscription;
use crate::domain::usage::Entry;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

pub struct Claude {
    roots: Vec<PathBuf>,
    cache: FileCache<logs::Parsed>,
    /// `~/.claude.json` : ne donne que le compte *actuellement* connecté, pas d'historique.
    account_path: Option<PathBuf>,
}

impl Claude {
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Claude { roots, cache: FileCache::default(), account_path: None }
    }
}

impl Default for Claude {
    fn default() -> Self {
        let roots = match std::env::var("CLAUDE_CONFIG_DIR") {
            Ok(dirs) => dirs.split(',').map(|d| Path::new(d.trim()).join("projects")).collect(),
            Err(_) => {
                let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
                vec![home.join(".claude/projects"), home.join(".config/claude/projects")]
            }
        };
        let account_path = std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".claude.json"));
        Claude { roots, cache: FileCache::default(), account_path }
    }
}

/// Type de facturation du compte actuellement connecté, si c'est un abonnement forfaitaire.
/// `~/.claude.json` ne garde que le compte courant : rien à en tirer sur les sessions passées.
fn active_subscription(path: &Path) -> Option<Subscription> {
    let raw = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let account = &v["oauthAccount"];
    let billing = account["billingType"].as_str()?;
    if !billing.contains("subscription") {
        return None;
    }
    let plan = account["seatTier"].as_str().unwrap_or(billing);
    Some(Subscription { plan: plan.to_string(), primary: None, secondary: None })
}

impl Provider for Claude {
    fn id(&self) -> &'static str {
        "claude"
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }

    fn available(&self) -> bool {
        self.roots.iter().any(|r| r.exists())
    }

    fn load(&self, since: DateTime<Utc>) -> Vec<Entry> {
        logs::load(&self.cache, &self.roots, since)
    }

    fn subscription(&self) -> Option<Subscription> {
        active_subscription(self.account_path.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_account(dir: &TempDir, json: &str) -> PathBuf {
        let path = dir.path().join(".claude.json");
        std::fs::write(&path, json).unwrap();
        path
    }

    #[test]
    fn reads_subscription_billing_as_a_plan() {
        let dir = TempDir::new().unwrap();
        let path =
            write_account(&dir, r#"{"oauthAccount":{"billingType":"stripe_subscription","seatTier":"team_standard"}}"#);
        let s = active_subscription(&path).unwrap();
        assert_eq!(s.plan, "team_standard");
        assert!(s.primary.is_none() && s.secondary.is_none());
    }

    #[test]
    fn api_billing_has_no_subscription() {
        let dir = TempDir::new().unwrap();
        let path = write_account(&dir, r#"{"oauthAccount":{"billingType":"api_key"}}"#);
        assert!(active_subscription(&path).is_none());
    }

    #[test]
    fn missing_or_invalid_file_has_no_subscription() {
        let dir = TempDir::new().unwrap();
        assert!(active_subscription(&dir.path().join("absent.json")).is_none());
        let path = write_account(&dir, "{oops");
        assert!(active_subscription(&path).is_none());
    }
}
