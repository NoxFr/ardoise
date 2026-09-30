//! Jetons de la maquette « Conso IA » (couleurs définies en OKLCH dans la maquette HTML).

use eframe::egui::{Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Visuals};
use std::sync::{Arc, LazyLock};

/// OKLCH → sRGB, `hue` en degrés, `alpha` dans [0, 1].
pub fn oklch(l: f32, c: f32, hue: f32, alpha: f32) -> Color32 {
    let (a, b) = (c * hue.to_radians().cos(), c * hue.to_radians().sin());
    let l_ = (l + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m_ = (l - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s_ = (l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    let lin = [
        4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_,
        -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_,
    ];
    let [r, g, b] = lin.map(|x| {
        let x = x.clamp(0.0, 1.0);
        let v = if x <= 0.003_130_8 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 };
        (v * 255.0).round() as u8
    });
    Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0).round() as u8)
}

/// Blanc translucide, pour les pistes, bordures et survols.
pub fn white(alpha: f32) -> Color32 {
    Color32::from_white_alpha((alpha * 255.0).round() as u8)
}

pub static PANEL: LazyLock<Color32> = LazyLock::new(|| oklch(0.21, 0.008, 260.0, 1.0));
pub static TEXT: LazyLock<Color32> = LazyLock::new(|| oklch(0.95, 0.005, 260.0, 1.0));
pub static TEXT_SOFT: LazyLock<Color32> = LazyLock::new(|| oklch(0.82, 0.008, 260.0, 1.0));
pub static TEXT_SUB: LazyLock<Color32> = LazyLock::new(|| oklch(0.7, 0.01, 260.0, 1.0));
pub static MUTED: LazyLock<Color32> = LazyLock::new(|| oklch(0.65, 0.01, 260.0, 1.0));
pub static FAINT: LazyLock<Color32> = LazyLock::new(|| oklch(0.6, 0.01, 260.0, 1.0));
pub static ICON: LazyLock<Color32> = LazyLock::new(|| oklch(0.75, 0.01, 260.0, 1.0));
pub static TAB_ON: LazyLock<Color32> = LazyLock::new(|| oklch(0.97, 0.0, 0.0, 1.0));
pub static TAB_OFF: LazyLock<Color32> = LazyLock::new(|| oklch(0.72, 0.01, 260.0, 1.0));
pub static BUDGET: LazyLock<Color32> = LazyLock::new(|| oklch(0.72, 0.12, 250.0, 1.0));

fn provider_hue(id: &str) -> f32 {
    match id {
        "claude" => 250.0,
        "codex" => 165.0,
        "opencode" => 80.0,
        _ => 310.0,
    }
}

/// `shade(hue, k)` de la maquette : k = rang du modèle (0 = couleur du fournisseur).
/// La maquette prévoit trois modèles ; au-delà on s'arrête à une teinte encore lisible sur le fond.
pub fn shade(provider: &str, k: usize) -> Color32 {
    let k = k as f32;
    oklch((0.78 - k * 0.14).max(0.42), (0.12 - k * 0.02).max(0.06), provider_hue(provider), 1.0)
}

/// Inter embarquée (licence OFL, `assets/fonts/OFL.txt`) : même rendu quelle que soit la
/// distribution, et des tests d'interface qui ne dépendent pas des polices installées.
const REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.otf");
const SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.otf");

pub fn install(ctx: &Context) {
    install_fonts(ctx);
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
    ctx.set_visuals(Visuals::dark());
    ctx.global_style_mut(|style| {
        style.visuals.override_text_color = Some(*TEXT);
        style.spacing.item_spacing.y = 0.0;
        let w = &mut style.visuals.widgets;
        for v in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            // Le style par défaut agrandit les widgets au survol et décale leurs voisins.
            v.expansion = 0.0;
            v.corner_radius = CornerRadius::same(6);
        }
        w.hovered.weak_bg_fill = white(0.1);
        w.hovered.bg_stroke = Stroke::NONE;
        w.active.weak_bg_fill = white(0.16);
        w.active.bg_stroke = Stroke::NONE;
        // Par défaut la barre de défilement flottante prend la couleur du texte (trop voyante) :
        // un gris discret et plus fin s'accorde mieux au thème sombre.
        style.spacing.scroll.foreground_color = false;
        style.spacing.scroll.bar_width = 6.0;
    });
}

fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("regular".into(), Arc::new(FontData::from_static(REGULAR)));
    fonts.font_data.insert("semibold".into(), Arc::new(FontData::from_static(SEMIBOLD)));
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    let family = |name: &str| std::iter::once(name.to_string()).chain(fallback.iter().cloned()).collect();
    fonts.families.insert(FontFamily::Proportional, family("regular"));
    fonts.families.insert(FontFamily::Name("semibold".into()), family("semibold"));
    ctx.set_fonts(fonts);
}

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1.0, 0.0, 0.0, [255, 255, 255])]
    #[case(0.0, 0.0, 0.0, [0, 0, 0])]
    // Références calculées avec les matrices d'Ottosson (implémentation Python indépendante).
    #[case(0.7, 0.1, 250.0, [109, 163, 218])]
    #[case(0.78, 0.12, 165.0, [98, 208, 164])]
    #[case(0.78, 0.12, 80.0, [224, 174, 87])]
    fn converts_oklch_to_srgb(#[case] l: f32, #[case] c: f32, #[case] h: f32, #[case] rgb: [u8; 3]) {
        let got = oklch(l, c, h, 1.0);
        for (a, b) in [got.r(), got.g(), got.b()].into_iter().zip(rgb) {
            assert!(a.abs_diff(b) <= 2, "{got:?} vs {rgb:?}");
        }
    }

    #[test]
    fn model_shades_darken_with_rank() {
        let lum = |c: Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
        assert!(lum(shade("claude", 0)) > lum(shade("claude", 1)));
        assert!(lum(shade("claude", 1)) > lum(shade("claude", 2)));
        assert_eq!(shade("claude", 5), shade("claude", 9), "plancher de lisibilité");
    }
}
