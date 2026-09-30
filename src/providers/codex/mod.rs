//! Codex CLI : rollouts JSONL de `~/.codex/sessions`, coût estimé au tarif API public OpenAI.

mod pricing;
mod rollout;

use super::Provider;
use crate::domain::subscription::Subscription;
use crate::domain::usage::Entry;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

pub struct Codex {
    roots: Vec<PathBuf>,
}

impl Codex {
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Codex { roots }
    }
}

impl Default for Codex {
    fn default() -> Self {
        let home = std::env::var("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".codex"));
        Codex { roots: vec![home.join("sessions"), home.join("archived_sessions")] }
    }
}

impl Provider for Codex {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn name(&self) -> &'static str {
        "Codex"
    }

    fn available(&self) -> bool {
        self.roots.iter().any(|r| r.exists())
    }

    fn load(&self, since: DateTime<Utc>) -> Vec<Entry> {
        rollout::load(&self.roots, since)
    }

    fn subscription(&self) -> Option<Subscription> {
        rollout::subscription(&self.roots)
    }
}
