use crate::ui::theme::{regular, white, TAB_OFF, TAB_ON};
use eframe::egui::{pos2, vec2, Align2, Color32, CursorIcon, Id, Rect, Sense, Ui, WidgetInfo, WidgetType};

const PAD: f32 = 2.0;
const GAP: f32 = 2.0;
const BTN_PAD: [f32; 2] = [9.0, 3.0];
const TEXT: f32 = 11.0;

/// Sélecteur compact de la maquette : pastille translucide, option active surélevée.
/// La taille ne dépend que des libellés, jamais du survol. Renvoie `true` si la sélection change.
pub fn segmented(ui: &mut Ui, id: Id, labels: &[&str], selected: &mut usize) -> bool {
    let galleys: Vec<_> =
        labels.iter().map(|l| ui.painter().layout_no_wrap(l.to_string(), regular(TEXT), Color32::WHITE)).collect();
    let height = galleys.iter().map(|g| g.size().y).fold(0.0, f32::max) + 2.0 * BTN_PAD[1];
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 2.0 * BTN_PAD[0]).collect();
    let total = widths.iter().sum::<f32>() + GAP * (labels.len() as f32 - 1.0) + 2.0 * PAD;
    let (rect, _) = ui.allocate_exact_size(vec2(total, height + 2.0 * PAD), Sense::hover());
    ui.painter().rect_filled(rect, 8.0, white(0.06));

    let mut changed = false;
    let mut x = rect.left() + PAD;
    for (i, (label, w)) in labels.iter().zip(&widths).enumerate() {
        let r = Rect::from_min_size(pos2(x, rect.top() + PAD), vec2(*w, height));
        x += w + GAP;
        let resp = ui.interact(r, id.with(i), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let sel = *selected == i;
        resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, sel, *label));
        if resp.clicked() && !sel {
            *selected = i;
            changed = true;
        }
        let fill = match (sel, resp.hovered()) {
            (true, _) => white(0.16),
            (false, true) => white(0.08),
            _ => Color32::TRANSPARENT,
        };
        ui.painter().rect_filled(r, 6.0, fill);
        let color = if sel { *TAB_ON } else { *TAB_OFF };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, *label, regular(TEXT), color);
    }
    changed
}
