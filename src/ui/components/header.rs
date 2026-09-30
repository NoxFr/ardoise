use super::segmented;
use crate::domain::period::Period;
use crate::ui::theme::{semibold, white, ICON, TAB_OFF, TAB_ON};
use eframe::egui::{
    vec2, Align, Button, CursorIcon, Id, Layout, Painter, Pos2, Response, RichText, Sense, Shape, Stroke, Ui, Vec2,
    WidgetInfo, WidgetType,
};
use std::f32::consts::PI;

pub enum HeaderAction {
    Refresh,
    Close,
    ToggleDetails,
    ToggleSettings,
    Period(Period),
}

pub struct HeaderState<'a> {
    pub period: Period,
    pub details: bool,
    pub settings: bool,
    pub loading: bool,
    pub status: &'a str,
}

pub fn header(ui: &mut Ui, st: HeaderState) -> Option<HeaderAction> {
    let HeaderState { period, details, settings, loading, status } = st;
    let mut action = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (logo, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        super::app_logo(ui.painter(), logo);
        ui.label(RichText::new("Ardoise").font(semibold(13.0)));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if icon_button(ui, "Fermer", close_icon).on_hover_text("Fermer").clicked() {
                action = Some(HeaderAction::Close);
            }
            let refresh = ui.add_enabled_ui(!loading, |ui| icon_button(ui, "Rafraîchir", refresh_icon)).inner;
            if refresh.on_hover_text(status).clicked() {
                action = Some(HeaderAction::Refresh);
            }
            let gear = icon_button(ui, "Paramètres", gear_icon).on_hover_text("Paramètres");
            if settings {
                ui.painter().rect_filled(gear.rect, 6.0, white(0.16));
                gear_icon(ui.painter(), gear.rect.center(), Stroke::new(1.6_f32, *ICON));
            }
            if gear.clicked() {
                action = Some(HeaderAction::ToggleSettings);
            }

            let mut i = Period::ALL.iter().position(|p| *p == period).unwrap_or(1);
            if segmented(ui, Id::new("period"), &Period::ALL.map(Period::label), &mut i) {
                action = Some(HeaderAction::Period(Period::ALL[i]));
            }

            let text = RichText::new("Détails").size(11.0).color(if details { *TAB_ON } else { *TAB_OFF });
            let btn = Button::new(text)
                .fill(if details { white(0.16) } else { white(0.0) })
                .stroke(Stroke::new(1.0_f32, white(0.14)))
                .corner_radius(8)
                .min_size(vec2(0.0, 22.0));
            let details_resp = ui
                .add(btn.selected(details))
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Afficher les tokens par modèle");
            if details_resp.clicked() {
                action = Some(HeaderAction::ToggleDetails);
            }
        });
    });
    action
}

/// Bouton-icône 24×24 dessiné au pinceau (pas de glyphe emoji), étiqueté pour l'accessibilité.
fn icon_button(ui: &mut Ui, label: &str, draw: fn(&Painter, Pos2, Stroke)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    if resp.hovered() && ui.is_enabled() {
        ui.painter().rect_filled(rect, 6.0, white(0.1));
    }
    let color = if ui.is_enabled() { *ICON } else { ICON.gamma_multiply(0.45) };
    draw(ui.painter(), rect.center(), Stroke::new(1.6_f32, color));
    resp
}

/// Flèche circulaire (Lucide « rotate-cw »), 13 px.
fn refresh_icon(p: &Painter, c: Pos2, stroke: Stroke) {
    let r = 5.2;
    let arc: Vec<Pos2> =
        (0..=24).map(|i| -PI * 0.3 + i as f32 / 24.0 * PI * 1.7).map(|a| c + r * Vec2::angled(a)).collect();
    p.add(Shape::line(arc, stroke));
    let tip = c + r * Vec2::angled(-PI * 0.3);
    p.line_segment([tip, tip + vec2(0.0, -3.2)], stroke);
    p.line_segment([tip, tip + vec2(-3.2, 0.2)], stroke);
}

fn close_icon(p: &Painter, c: Pos2, stroke: Stroke) {
    let d = 3.6;
    p.line_segment([c + vec2(-d, -d), c + vec2(d, d)], stroke);
    p.line_segment([c + vec2(-d, d), c + vec2(d, -d)], stroke);
}

/// Roue crantée : anneau et huit dents.
fn gear_icon(p: &Painter, c: Pos2, stroke: Stroke) {
    p.circle_stroke(c, 2.6, stroke);
    for k in 0..8 {
        let d = Vec2::angled(k as f32 * PI / 4.0);
        p.line_segment([c + d * 4.4, c + d * 6.2], stroke);
    }
    p.circle_stroke(c, 4.6, Stroke::new(stroke.width * 0.8, stroke.color));
}
