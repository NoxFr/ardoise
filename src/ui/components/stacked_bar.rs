use crate::ui::theme::white;
use eframe::egui::{Color32, CornerRadius, Rect, Sense, Ui, pos2, vec2};

const HEIGHT: f32 = 6.0;
const GAP: f32 = 2.0;

/// Barre de répartition : segments proportionnels, séparés de 2 px, arrondis aux extrémités seulement.
pub fn stacked_bar(ui: &mut Ui, parts: &[(f32, Color32)]) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
    let parts: Vec<_> = parts.iter().filter(|p| p.0 > 0.0).collect();
    let total: f32 = parts.iter().map(|p| p.0).sum();
    if total <= 0.0 {
        ui.painter().rect_filled(rect, 3.0, white(0.08));
        return;
    }
    let usable = rect.width() - GAP * parts.len().saturating_sub(1) as f32;
    let mut x = rect.left();
    for (i, (v, color)) in parts.iter().enumerate() {
        let w = usable * v / total;
        let r = if i == 0 { 3 } else { 0 };
        let l = if i == parts.len() - 1 { 3 } else { 0 };
        let corners = CornerRadius { nw: r, sw: r, ne: l, se: l };
        ui.painter().rect_filled(Rect::from_min_size(pos2(x, rect.top()), vec2(w, HEIGHT)), corners, *color);
        x += w + GAP;
    }
}
