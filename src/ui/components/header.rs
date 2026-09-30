use super::{segmented, segmented_width};
use crate::domain::period::Period;
use crate::ui::theme::{regular, semibold, white, ICON, TAB_OFF, TAB_ON};
use eframe::egui::{
    vec2, Align, Button, Color32, CursorIcon, Id, Layout, Painter, Pos2, Response, RichText, Sense, Shape, Stroke, Ui,
    Vec2, WidgetInfo, WidgetType,
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

const SPACING: f32 = 6.0;
const ICON_SIZE: f32 = 24.0;
const DETAILS_TEXT: f32 = 11.0;

/// Sur une ligne si tout tient, sinon titre et icônes en haut, réglages d'affichage en dessous.
/// Le choix se fait sur les largeurs mesurées, pas sur le frame précédent : pas de saut.
pub fn header(ui: &mut Ui, st: HeaderState) -> Option<HeaderAction> {
    let mut action = None;
    let one_line = title_width(ui) + SPACING + icons_width() + SPACING + controls_width(ui) <= ui.available_width();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING;
        let (logo, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        super::app_logo(ui.painter(), logo);
        ui.label(RichText::new("Ardoise").font(semibold(13.0)));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = SPACING;
            icons(ui, &st, &mut action);
            if one_line {
                controls(ui, &st, &mut action);
            }
        });
    });
    if !one_line {
        ui.add_space(8.0);
        // Dans une ligne : un layout de droite à gauche posé seul prendrait toute la hauteur.
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = SPACING;
                controls(ui, &st, &mut action);
            });
        });
    }
    action
}

fn title_width(ui: &Ui) -> f32 {
    let title = ui.painter().layout_no_wrap("Ardoise".into(), semibold(13.0), Color32::WHITE);
    16.0 + SPACING + title.size().x
}

fn icons_width() -> f32 {
    3.0 * ICON_SIZE + 2.0 * SPACING
}

fn controls_width(ui: &Ui) -> f32 {
    let details = ui.painter().layout_no_wrap("Détails".into(), regular(DETAILS_TEXT), Color32::WHITE);
    let details = details.size().x + 2.0 * ui.spacing().button_padding.x;
    details + SPACING + segmented_width(ui, &Period::ALL.map(Period::label))
}

fn icons(ui: &mut Ui, st: &HeaderState, action: &mut Option<HeaderAction>) {
    if icon_button(ui, "Fermer", close_icon).on_hover_text("Fermer").clicked() {
        *action = Some(HeaderAction::Close);
    }
    let refresh = ui.add_enabled_ui(!st.loading, |ui| icon_button(ui, "Rafraîchir", refresh_icon)).inner;
    if refresh.on_hover_text(st.status).clicked() {
        *action = Some(HeaderAction::Refresh);
    }
    let gear = icon_button(ui, "Paramètres", gear_icon).on_hover_text("Paramètres");
    if st.settings {
        ui.painter().rect_filled(gear.rect, 6.0, white(0.16));
        gear_icon(ui.painter(), gear.rect.center(), Stroke::new(1.6_f32, *ICON));
    }
    if gear.clicked() {
        *action = Some(HeaderAction::ToggleSettings);
    }
}

/// Période et bouton « Détails », de droite à gauche.
fn controls(ui: &mut Ui, st: &HeaderState, action: &mut Option<HeaderAction>) {
    let mut i = Period::ALL.iter().position(|p| *p == st.period).unwrap_or(1);
    if segmented(ui, Id::new("period"), &Period::ALL.map(Period::label), &mut i) {
        *action = Some(HeaderAction::Period(Period::ALL[i]));
    }

    let details = st.details;
    let text = RichText::new("Détails").size(DETAILS_TEXT).color(if details { *TAB_ON } else { *TAB_OFF });
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
        *action = Some(HeaderAction::ToggleDetails);
    }
}

/// Bouton-icône 24×24 dessiné au pinceau (pas de glyphe emoji), étiqueté pour l'accessibilité.
fn icon_button(ui: &mut Ui, label: &str, draw: fn(&Painter, Pos2, Stroke)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(ICON_SIZE), Sense::click());
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
