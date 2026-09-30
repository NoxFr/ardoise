use super::cell::cell;
use crate::domain::usage::ModelStat;
use crate::ui::format;
use crate::ui::theme::{white, MUTED, TEXT, TEXT_SOFT};
use eframe::egui::{pos2, vec2, Align, Color32, Rect, RichText, Sense, Ui};

const BAR_W: f32 = 70.0;
const VALUE_W: f32 = 54.0;
const SHARE_W: f32 = 28.0;
const GAP: f32 = 8.0;

/// `bar` : part du modèle rapportée au plus gros modèle du fournisseur ; `share` : part du
/// fournisseur ; `show_cost` : faux pour un fournisseur en abonnement (tokens à la place du $).
pub fn model_row(ui: &mut Ui, m: &ModelStat, color: Color32, bar: f32, share: f64, details: bool, show_cost: bool) {
    if details {
        detailed(ui, m, color, bar, share, show_cost);
    } else {
        compact(ui, m, color, bar, show_cost);
    }
}

fn dot(ui: &Ui, left: f32, cy: f32, color: Color32) {
    ui.painter().rect_filled(Rect::from_center_size(pos2(left + 3.0, cy), vec2(6.0, 6.0)), 1.0, color);
}

fn track(ui: &Ui, rect: Rect, ratio: f32, color: Color32) {
    ui.painter().rect_filled(rect, 2.0, white(0.08));
    let mut fill = rect;
    fill.set_width(rect.width() * ratio.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, 2.0, color);
}

fn compact(ui: &mut Ui, m: &ModelStat, color: Color32, bar: f32, show_cost: bool) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let value_x = rect.right() - VALUE_W;
    let bar_x = value_x - GAP - BAR_W;
    dot(ui, rect.left(), rect.center().y, color);
    let name = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top()), pos2(bar_x - GAP, rect.bottom()));
    cell(ui, name, RichText::new(&m.name).size(12.0).color(*TEXT_SOFT), Align::Min);
    track(ui, Rect::from_center_size(pos2(bar_x + BAR_W / 2.0, rect.center().y), vec2(BAR_W, 3.0)), bar, color);
    let value = Rect::from_min_max(pos2(value_x, rect.top()), rect.max);
    let text = if show_cost { format::money(m.cost) } else { format::tokens(m.input + m.output) };
    cell(ui, value, RichText::new(text).size(12.0).color(*TEXT_SOFT), Align::Max);
    resp.on_hover_text(format!("↓ {} entrée · ↑ {} sortie", format::tokens(m.input), format::tokens(m.output)));
}

fn detailed(ui: &mut Ui, m: &ModelStat, color: Color32, bar: f32, share: f64, show_cost: bool) {
    ui.add_space(2.0);
    let (top, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    dot(ui, top.left(), top.center().y, color);
    let share_rect = Rect::from_min_max(pos2(top.right() - SHARE_W, top.top()), top.max);
    cell(ui, share_rect, RichText::new(format::share(share)).size(10.0).color(*MUTED), Align::Max);
    let value = Rect::from_min_max(
        pos2(top.right() - SHARE_W - 6.0 - 90.0, top.top()),
        pos2(top.right() - SHARE_W - 6.0, top.bottom()),
    );
    let text = if show_cost { format::money(m.cost) } else { format::tokens(m.input + m.output) };
    cell(ui, value, RichText::new(text).size(12.0).color(*TEXT), Align::Max);
    let name = Rect::from_min_max(pos2(top.left() + 12.0, top.top()), pos2(value.left() - 6.0, top.bottom()));
    cell(ui, name, RichText::new(&m.name).size(12.0).color(*TEXT), Align::Min);

    ui.add_space(5.0);
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 3.0), Sense::hover());
    track(ui, line, bar, color);

    ui.add_space(5.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let small = |t: String| RichText::new(t).size(10.0).color(*MUTED);
        ui.label(small(format!("↓ {}", format::tokens(m.input))));
        ui.label(small(format!("↑ {}", format::tokens(m.output))));
        ui.with_layout(eframe::egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(small(format!("{} tok", format::tokens(m.input + m.output))));
        });
    });
    ui.add_space(2.0);
}
