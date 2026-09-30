use crate::domain::period::Period;
use crate::domain::refresh::AutoRefresh;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Échelle réelle correspondant à « 100 % » : la taille de la maquette.
pub const BASE_ZOOM: f32 = 1.0;
pub const ZOOM_MIN: f32 = 0.6;
pub const ZOOM_MAX: f32 = 2.0;
pub const ZOOM_STEP: f32 = 0.1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub period: Period,
    /// Lignes de modèle détaillées (tokens, barre pleine largeur).
    pub details: bool,
    /// Identifiants des agents affichés ; `None` : ceux dont des données existent localement.
    pub agents: Option<Vec<String>>,
    /// 1.0 = 100 %.
    pub zoom: f32,
    /// Taille choisie à la souris ; `None` : la hauteur suit le contenu.
    pub size: Option<[f32; 2]>,
    pub auto_refresh: AutoRefresh,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            period: Period::Week,
            details: false,
            agents: None,
            zoom: 1.0,
            size: None,
            auto_refresh: AutoRefresh::default(),
        }
    }
}

impl Settings {
    pub fn default_path() -> Option<PathBuf> {
        let config = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
            .ok()?;
        Some(config.join("ardoise/settings.json"))
    }

    /// Fichier absent ou invalide : réglages par défaut.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }

    pub fn zoom_by(&mut self, steps: i32) {
        let z = self.zoom + steps as f32 * ZOOM_STEP;
        self.zoom = ((z * 10.0).round() / 10.0).clamp(ZOOM_MIN, ZOOM_MAX);
    }

    pub fn scale(&self) -> f32 {
        BASE_ZOOM * self.zoom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempfile::TempDir;

    #[test]
    fn roundtrips_through_disk() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub/settings.json");
        let s = Settings {
            period: Period::Month,
            details: true,
            agents: Some(vec!["codex".into()]),
            zoom: 1.3,
            size: Some([500.0, 700.0]),
            auto_refresh: AutoRefresh::S10,
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
    }

    #[test]
    fn invalid_or_missing_file_gives_defaults() {
        let dir = TempDir::new().unwrap();
        assert_eq!(Settings::load(&dir.path().join("absent.json")), Settings::default());
        std::fs::write(dir.path().join("bad.json"), "{oops").unwrap();
        assert_eq!(Settings::load(&dir.path().join("bad.json")), Settings::default());
    }

    #[rstest]
    #[case(1.0, 1, 1.1)]
    #[case(1.0, -3, 0.7)]
    #[case(0.6, -1, 0.6)]
    #[case(2.0, 1, 2.0)]
    fn zoom_steps_are_clamped(#[case] start: f32, #[case] steps: i32, #[case] expected: f32) {
        let mut s = Settings { zoom: start, ..Default::default() };
        s.zoom_by(steps);
        assert!((s.zoom - expected).abs() < 1e-6, "{} != {expected}", s.zoom);
    }

    #[test]
    fn hundred_percent_is_the_mockup_scale() {
        assert_eq!(Settings::default().scale(), 1.0);
    }
}
