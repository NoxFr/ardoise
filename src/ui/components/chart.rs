use super::cell::cell;
use crate::domain::period::Granularity;
use crate::domain::usage::{Bucket, ModelStat};
use crate::ui::format;
use crate::ui::theme::{white, MUTED};
use chrono::Local;
use eframe::egui::{pos2, vec2, Align, Color32, CornerRadius, Rect, RichText, Sense, Shape, Stroke, Ui};

const HEIGHT: f32 = 52.0;

/// Histogramme empilé par modèle ; `colors[i]` correspond à `models[i]`. Sans `show_cost`
/// (abonnement), les tranches contiennent des tokens et non des $.
pub fn chart(
    ui: &mut Ui,
    buckets: &[Bucket],
    models: &[ModelStat],
    colors: &[Color32],
    granularity: Granularity,
    show_cost: bool,
) {
    let label = |b: &Bucket| format::bucket_label(b.start.with_timezone(&Local), granularity);
    let value = |v: f64| if show_cost { format::money(v) } else { format::tokens(v as u64) };
    let title = match (show_cost, granularity) {
        (true, Granularity::Hour) => "Dépenses par heure",
        (true, Granularity::Day) => "Dépenses par jour",
        (true, Granularity::Month) => "Dépenses par mois",
        (false, Granularity::Hour) => "Tokens par heure",
        (false, Granularity::Day) => "Tokens par jour",
        (false, Granularity::Month) => "Tokens par mois",
    };
    let small = |t: String| RichText::new(t).size(10.0).color(*MUTED);
    let max = buckets.iter().map(Bucket::total).fold(0.0, f64::max);

    ui.add_space(8.0);
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 13.0), Sense::hover());
    cell(ui, row, small(title.into()), Align::Min);
    cell(ui, row, small(format!("max {}", value(max))), Align::Max);
    ui.add_space(8.0);

    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
    let p = ui.painter();
    p.hline(rect.x_range(), rect.top(), Stroke::new(1.0_f32, white(0.1)));
    p.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0_f32, white(0.1)));
    let dashes = Shape::dashed_line(
        &[pos2(rect.left(), rect.center().y), pos2(rect.right(), rect.center().y)],
        Stroke::new(1.0_f32, white(0.07)),
        3.0,
        3.0,
    );
    p.extend(dashes);

    let n = buckets.len().max(1);
    let gap = match n {
        n if n > 24 => 1.0,
        n if n > 7 => 2.0,
        _ => 4.0,
    };
    let w = (rect.width() - gap * (n - 1) as f32) / n as f32;
    let inner = rect.shrink2(vec2(0.0, 1.0));
    for (i, b) in buckets.iter().enumerate() {
        let x = rect.left() + i as f32 * (w + gap);
        let total = b.total();
        let column = Rect::from_min_max(pos2(x, rect.top()), pos2(x + w, rect.bottom()));
        ui.interact(column, ui.id().with(("bar", i)), Sense::hover()).on_hover_ui(|ui| {
            ui.label(format!("{} · {}", label(b), value(total)));
        });
        if total <= 0.0 || max <= 0.0 {
            continue;
        }
        let h = ((total / max) as f32).max(0.03) * inner.height();
        let segs: Vec<_> = models
            .iter()
            .zip(colors)
            .map(|(m, c)| (b.by_model.get(&m.name).copied().unwrap_or(0.0), *c))
            .filter(|(v, _)| *v > 0.0)
            .collect();
        let mut y = inner.bottom();
        for (k, (v, color)) in segs.iter().enumerate() {
            let sh = (*v / total) as f32 * h;
            let bottom = if k == 0 { 2 } else { 0 };
            let top = if k == segs.len() - 1 { 2 } else { 0 };
            let corners = CornerRadius { nw: top, ne: top, sw: bottom, se: bottom };
            ui.painter().rect_filled(Rect::from_min_max(pos2(x, y - sh), pos2(x + w, y)), corners, *color);
            y -= sh;
        }
    }

    ui.add_space(6.0);
    let (axis, _) = ui.allocate_exact_size(vec2(ui.available_width(), 13.0), Sense::hover());
    if let (Some(first), Some(last)) = (buckets.first(), buckets.last()) {
        cell(ui, axis, small(label(first)), Align::Min);
        cell(ui, axis, small(label(last)), Align::Max);
    }
}
