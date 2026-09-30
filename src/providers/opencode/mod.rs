//! OpenCode : table `message` de `~/.local/share/opencode/opencode.db`, coût calculé par OpenCode.

mod db;

use super::Provider;
use crate::domain::usage::Entry;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

pub struct OpenCode {
    db: PathBuf,
}

impl OpenCode {
    pub fn with_db(db: PathBuf) -> Self {
        OpenCode { db }
    }
}

impl Default for OpenCode {
    fn default() -> Self {
        let data = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share"));
        OpenCode { db: data.join("opencode/opencode.db") }
    }
}

impl Provider for OpenCode {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn name(&self) -> &'static str {
        "OpenCode"
    }

    fn available(&self) -> bool {
        self.db.exists()
    }

    fn load(&self, since: DateTime<Utc>) -> Vec<Entry> {
        db::load(&self.db, since).unwrap_or_default()
    }
}
