//! Lance Ardoise avec des données factices, pour produire la capture d'écran du README.
//! `cargo run --release --example screenshot`

use ardoise::app::{App, Config, WIDTH};
use ardoise::domain::period::Period;
use ardoise::domain::subscription::Subscription;
use ardoise::domain::usage::Entry;
use ardoise::providers::Provider;
use ardoise::settings::Settings;
use chrono::{DateTime, Duration, Utc};
use eframe::egui::ViewportBuilder;
use std::sync::Arc;

struct Demo {
    id: &'static str,
    name: &'static str,
    entries: Vec<Entry>,
    plan: Option<&'static str>,
}

impl Provider for Demo {
    fn id(&self) -> &'static str {
        self.id
    }
    fn name(&self) -> &'static str {
        self.name
    }
    fn available(&self) -> bool {
        true
    }
    fn load(&self, _since: DateTime<Utc>) -> Vec<Entry> {
        self.entries.clone()
    }
    fn subscription(&self) -> Option<Subscription> {
        None
    }
    fn plan(&self) -> Option<String> {
        self.plan.map(str::to_string)
    }
}

fn entry(model: &str, ago: Duration, cost: f64, input: u64, output: u64) -> Entry {
    Entry { time: Utc::now() - ago, model: model.into(), input, output, cost }
}

fn claude_sample() -> Vec<Entry> {
    vec![
        entry("Opus 5.5", Duration::hours(2), 8.40, 42_000, 6_100),
        entry("Sonnet 5", Duration::days(1), 3.20, 61_000, 4_300),
        entry("Opus 5.5", Duration::days(2), 12.75, 58_000, 9_800),
        entry("Sonnet 5", Duration::days(5), 4.10, 33_000, 2_900),
        entry("Opus 5", Duration::days(9), 21.60, 120_000, 15_400),
        entry("Sonnet 5", Duration::days(14), 6.90, 47_000, 5_100),
        entry("Opus 5", Duration::days(21), 18.30, 98_000, 11_200),
        entry("Sonnet 5", Duration::days(27), 5.50, 39_000, 3_600),
    ]
}

fn codex_sample() -> Vec<Entry> {
    vec![
        entry("gpt-5.1-codex", Duration::hours(6), 4.80, 28_000, 5_200),
        entry("gpt-5.1-codex", Duration::days(3), 7.35, 44_000, 8_100),
        entry("gpt-5-mini", Duration::days(8), 1.20, 19_000, 2_400),
        entry("gpt-5.1-codex", Duration::days(18), 9.10, 52_000, 9_900),
    ]
}

fn opencode_sample() -> Vec<Entry> {
    vec![
        entry("qwen3.6-35b-a3b", Duration::hours(4), 0.64, 9_000, 1_100),
        entry("qwen3.6-35b-a3b", Duration::days(6), 0.38, 5_400, 700),
        entry("glm-4.7", Duration::days(12), 0.92, 14_000, 1_900),
    ]
}

fn main() -> eframe::Result {
    let settings_path = std::env::temp_dir().join("ardoise-screenshot-settings.json");
    let settings = Settings { period: Period::Month, details: true, budget: Some(400.0), ..Settings::default() };
    settings.save(&settings_path).expect("écriture des réglages de démonstration");

    let config = Config {
        providers: vec![
            Arc::new(Demo { id: "claude", name: "Claude Code", entries: claude_sample(), plan: Some("max_20x") }),
            Arc::new(Demo { id: "codex", name: "Codex", entries: codex_sample(), plan: None }),
            Arc::new(Demo { id: "opencode", name: "OpenCode", entries: opencode_sample(), plan: None }),
        ],
        settings_path: Some(settings_path),
    };
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Ardoise")
            .with_app_id("ardoise")
            .with_inner_size([WIDTH, 700.0])
            .with_resizable(true)
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native("Ardoise", options, Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx, config)))))
}
