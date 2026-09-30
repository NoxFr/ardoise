use crate::ui::format;
use crate::ui::theme::{BUDGET, MUTED};
use eframe::egui::{vec2, Align, Layout, RichText, Sense, Ui};

pub fn budget(ui: &mut Ui, spent: f64, budget: f64) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let small = |t: String| RichText::new(t).size(11.0).color(*MUTED);
        let tooltip =
            "Dépense estimée des 30 derniers jours pour les fournisseurs facturés à l'appel, hors abonnements";
        ui.label(small("Budget 30 j".into())).on_hover_text(tooltip);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(small(format!("{} / {}", format::money(spent), format::money(budget)))).on_hover_text(tooltip);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
            super::gauge(ui, rect, (spent / budget) as f32, *BUDGET);
            resp.on_hover_text(tooltip);
        });
    });
}
