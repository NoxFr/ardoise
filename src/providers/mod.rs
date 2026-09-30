pub mod claude;
pub mod codex;
mod files;
pub mod opencode;

use crate::domain::subscription::Subscription;
use crate::domain::usage::Entry;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Source de consommation (Claude Code, Codex, OpenCode…).
pub trait Provider: Send + Sync {
    /// Identifiant stable, utilisé pour les réglages et les couleurs.
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// Des données locales existent (sinon le réglage l'indique).
    fn available(&self) -> bool;
    /// Appels facturés depuis `since`.
    fn load(&self, since: DateTime<Utc>) -> Vec<Entry>;
    /// État courant des quotas d'abonnement, si le fournisseur en expose localement.
    fn subscription(&self) -> Option<Subscription> {
        None
    }
}

pub fn all() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(claude::Claude::default()),
        Arc::new(codex::Codex::default()),
        Arc::new(opencode::OpenCode::default()),
    ]
}
