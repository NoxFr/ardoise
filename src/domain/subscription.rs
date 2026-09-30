use chrono::{DateTime, Utc};

/// Position dans une fenêtre glissante de quota (ex. 5 h, 7 j) : pas de coût, un taux d'usage.
#[derive(Clone)]
pub struct RateLimitWindow {
    pub used_percent: f64,
    pub window_minutes: i64,
    pub resets_at: DateTime<Utc>,
}

/// Abonnement forfaitaire du fournisseur : la dépense n'est pas facturée à l'appel, seules des
/// fenêtres de quota s'appliquent.
#[derive(Clone)]
pub struct Subscription {
    pub plan: String,
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
}
