use crate::ui::theme::white;
use eframe::egui::{Color32, Rect, Ui};

/// Piste translucide remplie à `ratio` (borné entre 0 et 1) : budget, quotas, lignes de modèle.
pub fn gauge(ui: &Ui, rect: Rect, ratio: f32, color: Color32) {
    ui.painter().rect_filled(rect, 2.0, white(0.08));
    let mut fill = rect;
    fill.set_width(rect.width() * ratio.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, 2.0, color);
}
