#![allow(dead_code)]

use ardoise::domain::subscription::Subscription;
use ardoise::domain::usage::Entry;
use ardoise::providers::Provider;
use chrono::{DateTime, Duration, Utc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};

pub fn entry(model: &str, ago: Duration, cost: f64) -> Entry {
    Entry { time: Utc::now() - ago, model: model.into(), input: 1_000, output: 100, cost }
}

/// Fournisseur factice : renvoie des entrées fixes et compte les chargements.
/// Avec `gated`, tout chargement après le premier attend un `release.send(())`.
pub struct Fake {
    pub id: &'static str,
    pub name: &'static str,
    pub entries: Vec<Entry>,
    pub calls: Arc<AtomicUsize>,
    pub gate: Option<Mutex<mpsc::Receiver<()>>>,
    pub available: bool,
    pub subscription: Option<Subscription>,
    pub plan: Option<String>,
}

impl Fake {
    pub fn new(id: &'static str, name: &'static str, entries: Vec<Entry>) -> Self {
        Fake { id, name, entries, calls: Arc::default(), gate: None, available: true, subscription: None, plan: None }
    }

    pub fn with_plan(mut self, plan: &str) -> Self {
        self.plan = Some(plan.into());
        self
    }

    pub fn subscribed(mut self, s: Subscription) -> Self {
        self.subscription = Some(s);
        self
    }

    pub fn unavailable(mut self) -> Self {
        self.available = false;
        self
    }

    pub fn gated(mut self) -> (Self, mpsc::Sender<()>) {
        let (tx, rx) = mpsc::channel();
        self.gate = Some(Mutex::new(rx));
        (self, tx)
    }
}

impl Provider for Fake {
    fn id(&self) -> &'static str {
        self.id
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn available(&self) -> bool {
        self.available
    }

    fn load(&self, _: DateTime<Utc>) -> Vec<Entry> {
        if self.calls.fetch_add(1, Ordering::SeqCst) > 0 {
            if let Some(gate) = &self.gate {
                gate.lock().unwrap().recv().unwrap();
            }
        }
        self.entries.clone()
    }

    fn subscription(&self) -> Option<Subscription> {
        self.subscription.clone()
    }

    fn plan(&self) -> Option<String> {
        self.plan.clone()
    }
}

/// Ligne JSONL au format des transcripts Claude Code.
pub fn log_line(id: &str, model: &str, time: DateTime<Utc>, input: u64, output: u64) -> String {
    serde_json::json!({
        "timestamp": time.to_rfc3339(),
        "requestId": format!("req_{id}"),
        "message": {
            "id": format!("msg_{id}"),
            "model": model,
            "usage": { "input_tokens": input, "output_tokens": output }
        }
    })
    .to_string()
}

pub fn arc<P: Provider + 'static>(p: P) -> Arc<dyn Provider> {
    Arc::new(p)
}
