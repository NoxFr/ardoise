use eframe::egui::{Align, Label, Layout, Rect, Response, RichText, Ui, UiBuilder};

/// Place un libellé (tronqué si besoin) dans `rect`, aligné à gauche ou à droite.
pub fn cell(ui: &mut Ui, rect: Rect, text: RichText, align: Align) -> Response {
    let layout =
        if align == Align::Max { Layout::right_to_left(Align::Center) } else { Layout::left_to_right(Align::Center) };
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(layout), |ui| ui.add(Label::new(text).truncate())).inner
}
