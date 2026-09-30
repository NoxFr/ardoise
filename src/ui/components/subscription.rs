use crate::domain::subscription::{RateLimitWindow, Subscription};
use crate::ui::format;
use crate::ui::theme::{BUDGET, MUTED, TEXT_SOFT};
use chrono::{DateTime, Utc};
use eframe::egui::{vec2, Align, Layout, RichText, Sense, Ui};

fn window_gauge(ui: &mut Ui, w: &RateLimitWindow, now: DateTime<Utc>) {
    let tooltip = format!(
        "Quota sur {} glissants : {} utilisés, réinitialisation le {}",
        format::window_label(w.window_minutes),
        format::percent(w.used_percent),
        w.resets_at.with_timezone(&chrono::Local).format("%d/%m à %H:%M")
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new(format::window_name(w.window_minutes)).size(11.0).color(*TEXT_SOFT))
            .on_hover_text(&tooltip);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let value = format!("{} · {}", format::percent(w.used_percent), format::reset(w.resets_at, now));
            ui.label(RichText::new(value).size(11.0).color(*MUTED)).on_hover_text(&tooltip);
        });
    });
    ui.add_space(2.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
    super::gauge(ui, rect, (w.used_percent / 100.0) as f32, *BUDGET);
    resp.on_hover_text(tooltip);
}

/// Jauges de quota de l'abonnement, si le fournisseur en expose (Codex) ; le plan est dans l'en-tête.
pub fn subscription(ui: &mut Ui, sub: &Subscription) {
    let now = Utc::now();
    for w in [&sub.primary, &sub.secondary].into_iter().flatten() {
        ui.add_space(8.0);
        window_gauge(ui, w, now);
    }
}
