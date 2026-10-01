#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use ardoise::app::{App, Config, WIDTH};
use ardoise::providers;
use ardoise::settings::Settings;
use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result {
    // RUST_LOG=ardoise=debug pour le détail des chargements.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("ardoise=info")).init();
    let config = Config { providers: providers::all(), settings_path: Settings::default_path() };
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
        // Avec la vsync, sous X11 le gestionnaire de fenêtres étire l'image précédente pendant un
        // redimensionnement, le temps que la nouvelle arrive. egui ne redessine qu'à la demande.
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration { wgpu_setup: low_power_gpu(), ..Default::default() }
            .with_surface_config(eframe::egui_wgpu::SurfaceConfig {
                present_mode: eframe::wgpu::PresentMode::AutoNoVsync,
                ..eframe::egui_wgpu::SurfaceConfig::LOW_LATENCY
            }),
        ..Default::default()
    };
    eframe::run_native("Ardoise", options, Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx, config)))))
}

/// GPU intégré plutôt que dédié (egui choisit « haute performance » par défaut) : sur un portable
/// NVIDIA sous X11, la carte dédiée étire l'ancienne image pendant tout le redimensionnement et
/// ignore la transparence. `WGPU_POWER_PREF=high` pour revenir au GPU dédié.
fn low_power_gpu() -> eframe::egui_wgpu::WgpuSetup {
    let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.power_preference =
        eframe::wgpu::PowerPreference::from_env().unwrap_or(eframe::wgpu::PowerPreference::LowPower);
    eframe::egui_wgpu::WgpuSetup::CreateNew(setup)
}
