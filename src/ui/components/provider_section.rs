use super::cell::cell;
use super::{chart, model_row};
use crate::domain::period::Granularity;
use crate::domain::subscription::Subscription;
use crate::domain::usage::{Bucket, ModelStat, Summary};
use crate::ui::format;
use crate::ui::theme::{semibold, shade, white, MUTED, TEXT};
use eframe::egui::{pos2, vec2, Align, Pos2, Rect, Response, RichText, Sense, Stroke, Ui};

pub struct Section<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub summary: &'a Summary,
    /// Part du fournisseur dans le total, entre 0 et 1.
    pub share: f64,
    pub buckets: &'a [Bucket],
    pub granularity: Granularity,
    pub details: bool,
    pub subscription: Option<&'a Subscription>,
    pub plan: Option<&'a str>,
}

/// Pastille du plan, à la suite du nom, sans dépasser `max_right`.
fn plan_badge(ui: &mut Ui, left_center: Pos2, max_right: f32, plan: &str, provider: &str) -> Response {
    let color = shade(provider, 0);
    let font = semibold(10.0);
    let width = ui.painter().layout_no_wrap(plan.into(), font.clone(), color).size().x + 12.0;
    let right = (left_center.x + width).min(max_right);
    let rect = Rect::from_min_max(pos2(left_center.x, left_center.y - 8.0), pos2(right, left_center.y + 8.0));
    ui.painter().rect_filled(rect, 8.0, color.gamma_multiply(0.18));
    cell(ui, rect.shrink2(vec2(6.0, 0.0)), RichText::new(plan).font(font).color(color), Align::Min)
}

pub fn provider_section(ui: &mut Ui, s: Section) {
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let logo = Rect::from_center_size(pos2(row.left() + 6.0, row.center().y), vec2(12.0, 12.0));
    super::provider_logo(ui.painter(), logo, s.id);
    let show_cost = s.subscription.is_none();
    let share = Rect::from_min_max(pos2(row.right() - 28.0, row.top()), row.max);
    let share_cell = cell(ui, share, RichText::new(format::share(s.share)).size(10.0).color(*MUTED), Align::Max);
    if show_cost {
        share_cell.on_hover_text(format!("Part de {} dans la dépense totale de la période", s.name));
    } else {
        share_cell.on_hover_text("Non applicable : couvert par l'abonnement, pas de dépense à l'appel");
    }
    let value = Rect::from_min_max(pos2(row.left(), row.top()), pos2(share.left() - 6.0, row.bottom()));
    let value_text = if show_cost { format::money(s.summary.cost) } else { format::tokens(s.summary.tokens) };
    let value_cell = cell(ui, value, RichText::new(value_text).font(semibold(12.0)), Align::Max);
    if !show_cost {
        value_cell.on_hover_text("Tokens consommés, déjà couverts par l'abonnement (pas de coût à l'appel)");
    }
    let name_width = ui.painter().layout_no_wrap(s.name.into(), semibold(14.0), *TEXT).size().x;
    let name_right = (row.left() + 19.0 + name_width).min(row.center().x);
    let name = Rect::from_min_max(pos2(row.left() + 18.0, row.top()), pos2(name_right, row.bottom()));
    cell(ui, name, RichText::new(s.name).font(semibold(14.0)), Align::Min);
    if let Some(plan) = s.plan.map(format::plan_name).filter(|p| !p.is_empty()) {
        let tooltip = if show_cost {
            "Facturé à l'usage : coût estimé au tarif API public"
        } else {
            "Forfait : l'usage est déjà couvert par l'abonnement, pas de coût à l'appel"
        };
        plan_badge(ui, pos2(name.right() + 8.0, row.center().y), value.right() - 60.0, &plan, s.id)
            .on_hover_text(tooltip);
    }
    ui.add_space(6.0);
    let (rule, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(rule.x_range(), rule.center().y, Stroke::new(1.0_f32, white(0.07)));

    if s.summary.models.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("Aucune dépense sur la période").size(11.0).color(*MUTED));
    }
    let colors: Vec<_> = (0..s.summary.models.len()).map(|k| shade(s.id, k)).collect();
    let weight = |m: &ModelStat| if show_cost { m.cost } else { (m.input + m.output) as f64 };
    let biggest = s.summary.models.iter().map(weight).fold(0.0, f64::max);
    let sum: f64 = s.summary.models.iter().map(weight).sum();
    for (m, color) in s.summary.models.iter().zip(&colors) {
        ui.add_space(8.0);
        let bar = if biggest > 0.0 { (weight(m) / biggest) as f32 } else { 0.0 };
        let share = if sum > 0.0 { weight(m) / sum } else { 0.0 };
        model_row(ui, m, *color, bar, share, s.details, show_cost);
    }
    ui.add_space(8.0);
    chart(ui, s.buckets, &s.summary.models, &colors, s.granularity, show_cost);
    if let Some(sub) = s.subscription {
        super::subscription(ui, sub);
    }
}
