use crate::ui::theme::{MUTED, TEXT, TEXT_SOFT, regular, shade, white};
use eframe::egui::{
    Align, Align2, Button, CursorIcon, Frame, Id, Layout, Margin, Rect, RichText, Sense, Stroke, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};

pub struct Agent<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub shown: bool,
    pub available: bool,
}

pub enum SettingsAction {
    Toggle(usize),
    Zoom(i32),
}

pub fn settings_panel(
    ui: &mut Ui,
    agents: &[Agent],
    zoom: f32,
    can_shrink: bool,
    can_grow: bool,
) -> Option<SettingsAction> {
    let mut action = None;
    Frame::NONE.fill(white(0.04)).corner_radius(10.0).inner_margin(Margin::same(12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Agents affichés").size(10.0).color(*MUTED));
        ui.add_space(8.0);
        for (i, a) in agents.iter().enumerate() {
            if checkbox_row(ui, Id::new(("agent", a.id)), a) {
                action = Some(SettingsAction::Toggle(i));
            }
            ui.add_space(6.0);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Taille").size(10.0).color(*MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let btn =
                    |t: &str| Button::new(RichText::new(t).size(12.0)).fill(white(0.06)).min_size(vec2(22.0, 20.0));
                if ui.add_enabled(can_grow, btn("+")).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    action = Some(SettingsAction::Zoom(1));
                }
                ui.add_sized(
                    [40.0, 20.0],
                    eframe::egui::Label::new(RichText::new(format!("{:.0} %", zoom * 100.0)).size(11.0)),
                );
                if ui.add_enabled(can_shrink, btn("−")).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    action = Some(SettingsAction::Zoom(-1));
                }
            });
        });
    });
    action
}

/// Case à cocher à la couleur de l'agent ; toute la ligne est cliquable.
fn checkbox_row(ui: &mut Ui, id: Id, a: &Agent) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, a.shown, a.name));
    if resp.hovered() {
        ui.painter().rect_filled(rect.expand2(vec2(4.0, 1.0)), 5.0, white(0.05));
    }
    let color = shade(a.id, 0);
    let bx = Rect::from_center_size(pos2(rect.left() + 6.0, rect.center().y), vec2(12.0, 12.0));
    if a.shown {
        ui.painter().rect_filled(bx, 3.0, color);
        let s = Stroke::new(1.6_f32, eframe::egui::Color32::from_gray(20));
        let c = bx.center();
        ui.painter().line_segment([c + vec2(-3.0, 0.0), c + vec2(-1.0, 2.5)], s);
        ui.painter().line_segment([c + vec2(-1.0, 2.5), c + vec2(3.5, -2.5)], s);
    } else {
        ui.painter().rect_stroke(bx, 3.0, Stroke::new(1.0_f32, white(0.3)), eframe::egui::StrokeKind::Inside);
    }
    let logo = Rect::from_center_size(pos2(bx.right() + 14.0, rect.center().y), vec2(12.0, 12.0));
    super::provider_logo(ui.painter(), logo, a.id);
    let text_color = if a.shown { *TEXT } else { *TEXT_SOFT };
    let text_pos = pos2(logo.right() + 6.0, rect.center().y);
    ui.painter().text(text_pos, Align2::LEFT_CENTER, a.name, regular(12.0), text_color);
    if !a.available {
        ui.painter().text(rect.right_center(), Align2::RIGHT_CENTER, "aucune donnée locale", regular(10.0), *MUTED);
    }
    resp.clicked()
}
