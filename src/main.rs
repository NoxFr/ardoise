use ardoise::app::{App, Config, WIDTH};
use ardoise::providers;
use ardoise::settings::Settings;
use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result {
    let config = Config {
        providers: providers::all(),
        // Budget sur 30 jours, tous fournisseurs : ARDOISE_BUDGET=1200
        budget: std::env::var("ARDOISE_BUDGET").ok().and_then(|v| v.replace(',', ".").parse().ok()),
        settings_path: Settings::default_path(),
    };
    let size = config.settings_path.as_deref().map(Settings::load).and_then(|s| s.size).unwrap_or([WIDTH, 600.0]);
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Ardoise")
            .with_app_id("ardoise")
            .with_inner_size(size)
            .with_min_inner_size([320.0, 120.0])
            .with_resizable(true)
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native("Ardoise", options, Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx, config)))))
}
