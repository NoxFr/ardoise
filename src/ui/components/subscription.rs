use crate::domain::subscription::{RateLimitWindow, Subscription};
use crate::ui::format;
use crate::ui::theme::{white, BUDGET, MUTED};
use chrono::{DateTime, Utc};
use eframe::egui::{vec2, Align, Layout, RichText, Sense, Ui};

fn window_gauge(ui: &mut Ui, w: &RateLimitWindow, now: DateTime<Utc>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let small = |t: String| RichText::new(t).size(11.0).color(*MUTED);
        let resp = ui.label(small(format::window_label(w.window_minutes)));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(small(format!(
                "{} · reset dans {}",
                format::percent(w.used_percent),
                format::resets_in(w.resets_at, now)
            )));
            let (rect, bar_resp) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, white(0.08));
            let mut fill = rect;
            fill.set_width(rect.width() * (w.used_percent / 100.0).clamp(0.0, 1.0) as f32);
            ui.painter().rect_filled(fill, 2.0, *BUDGET);
            let tooltip = format!(
                "Quota {} : {} utilisés, réinitialisation le {}",
                format::window_label(w.window_minutes),
                format::percent(w.used_percent),
                w.resets_at.with_timezone(&chrono::Local).format("%d/%m à %H:%M")
            );
            resp.on_hover_text(tooltip.clone());
            bar_resp.on_hover_text(tooltip);
        });
    });
}

/// Abonnement forfaitaire du fournisseur, sans coût : jauges de quota si le fournisseur en expose
/// (Codex), sinon juste le plan du compte connecté (Claude Code).
pub fn subscription(ui: &mut Ui, sub: &Subscription) {
    ui.add_space(8.0);
    let label = if sub.plan.is_empty() { "Abonnement".to_string() } else { format!("Abonnement · {}", sub.plan) };
    ui.label(RichText::new(label).size(11.0).color(*MUTED))
        .on_hover_text("Forfait : l'usage de ce fournisseur est déjà couvert par l'abonnement, pas de coût par appel");
    let now = Utc::now();
    for w in [&sub.primary, &sub.secondary].into_iter().flatten() {
        ui.add_space(6.0);
        window_gauge(ui, w, now);
    }
}
