use crate::ui::format;
use crate::ui::theme::{semibold, FAINT, TEXT_SUB};
use eframe::egui::{vec2, Align, Layout, RichText, Ui};

const HEIGHT: f32 = 30.0;

pub fn total(ui: &mut Ui, cost: f64, tokens: u64, updated: &str) {
    // Hauteur fixe : un layout aligné en bas prendrait sinon toute la hauteur disponible.
    let size = vec2(ui.available_width(), HEIGHT);
    ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Max), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(RichText::new(format::money(cost)).font(semibold(26.0)));
        ui.label(RichText::new(format!("{} tokens", format::tokens(tokens))).size(12.0).color(*TEXT_SUB));
        ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
            ui.label(RichText::new(updated).size(10.0).color(*FAINT));
        });
    });
}
