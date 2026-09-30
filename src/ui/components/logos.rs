use crate::ui::theme::shade;
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Stroke, Vec2};
use std::f32::consts::PI;

const SLATE: Color32 = Color32::from_rgb(47, 52, 58);
const FRAME: Color32 = Color32::from_rgb(196, 160, 110);

/// Logo Ardoise : ardoise encadrée, trois barres de craie aux couleurs des agents.
pub fn app_logo(p: &Painter, rect: Rect) {
    let s = rect.width();
    p.rect_filled(rect, s * 0.22, FRAME);
    let board = rect.shrink(s * 0.1);
    p.rect_filled(board, s * 0.14, SLATE);
    let bars = [("claude", 0.45), ("codex", 0.7), ("opencode", 0.95)];
    let w = board.width() * 0.18;
    let gap = (board.width() * 0.76 - 3.0 * w) / 2.0;
    let base = board.bottom() - board.height() * 0.16;
    let max_h = board.height() * 0.62;
    for (i, (id, h)) in bars.iter().enumerate() {
        let x = board.left() + board.width() * 0.12 + i as f32 * (w + gap);
        p.rect_filled(Rect::from_min_max(pos2(x, base - max_h * h), pos2(x + w, base)), w * 0.3, shade(id, 0));
    }
}

/// Pictogramme de l'agent, tracé dans `rect` à sa couleur.
pub fn provider_logo(p: &Painter, rect: Rect, id: &str) {
    let color = shade(id, 0);
    let c = rect.center();
    let s = rect.width();
    match id {
        "claude" => spark(p, c, s * 0.5, color),
        "codex" => prompt(p, rect, color),
        "opencode" => braces(p, c, s, color),
        _ => {
            p.circle_filled(c, s * 0.35, color);
        }
    }
}

/// Étincelle à dix rayons, façon Claude.
fn spark(p: &Painter, c: Pos2, r: f32, color: Color32) {
    let stroke = Stroke::new(r * 0.28, color);
    for k in 0..10 {
        let d = Vec2::angled(k as f32 * PI / 5.0 - PI / 2.0);
        let len = if k % 2 == 0 { 1.0 } else { 0.78 };
        p.line_segment([c + d * r * 0.18, c + d * r * len], stroke);
    }
}

/// Invite de terminal « >_ » dans un cadre arrondi.
fn prompt(p: &Painter, rect: Rect, color: Color32) {
    let s = rect.width();
    let stroke = Stroke::new(s * 0.11, color);
    p.rect_stroke(rect.shrink(s * 0.06), s * 0.2, stroke, eframe::egui::StrokeKind::Inside);
    let o = rect.left_top();
    p.line_segment([o + vec2(s * 0.28, s * 0.34), o + vec2(s * 0.45, s * 0.5)], stroke);
    p.line_segment([o + vec2(s * 0.45, s * 0.5), o + vec2(s * 0.28, s * 0.66)], stroke);
    p.line_segment([o + vec2(s * 0.52, s * 0.68), o + vec2(s * 0.74, s * 0.68)], stroke);
}

/// Accolades « { } ».
fn braces(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let stroke = Stroke::new(s * 0.1, color);
    let (top, bottom) = (c.y - s * 0.38, c.y + s * 0.38);
    for dir in [-1.0f32, 1.0] {
        let end = c.x + dir * s * 0.1;
        let spine = c.x + dir * s * 0.22;
        let tip = c.x + dir * s * 0.36;
        let path = vec![
            pos2(end, top),
            pos2(spine, top + s * 0.08),
            pos2(spine, c.y - s * 0.1),
            pos2(tip, c.y),
            pos2(spine, c.y + s * 0.1),
            pos2(spine, bottom - s * 0.08),
            pos2(end, bottom),
        ];
        p.add(eframe::egui::Shape::line(path, stroke));
    }
}
