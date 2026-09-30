use crate::domain::dashboard::{Dashboard, Snapshot};
use crate::domain::period::Period;
use crate::providers::Provider;
use crate::settings::{Settings, ZOOM_MAX, ZOOM_MIN};
use crate::ui::components::{self, Agent, HeaderAction, HeaderState, Section, SettingsAction};
use crate::ui::theme::{self, white, MUTED, PANEL};
use chrono::{DateTime, Local, NaiveDate};
use eframe::egui::{self, Margin, ResizeDirection, RichText, Sense, Stroke, Ui, ViewportCommand};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const WIDTH: f32 = 440.0;
const PADDING: i8 = 18;
const GAP: f32 = 16.0;
const SECTION_GAP: f32 = 20.0;
const RESIZE_EVERY: Duration = Duration::from_millis(200);
const DRAG_EVERY: Duration = Duration::from_millis(500);
const MIN_HEIGHT: f32 = 120.0;
const MAX_HEIGHT: f32 = 1400.0;
/// Délai avant d'écrire une taille choisie à la souris : pas une écriture disque par frame.
const SAVE_DELAY: Duration = Duration::from_millis(500);

/// Par fournisseur ; `None` si son chargement a échoué.
type Loaded = Vec<Option<Snapshot>>;

struct Source {
    provider: Arc<dyn Provider>,
    /// Des données existent localement.
    available: bool,
}

/// Ce dont dépend le tableau de bord : il n'est recalculé que si l'un de ces éléments change.
#[derive(PartialEq)]
struct DashboardKey {
    generation: u64,
    period: Period,
    shown: Vec<usize>,
    today: NaiveDate,
}

pub struct Config {
    pub providers: Vec<Arc<dyn Provider>>,
    /// Budget sur 30 jours glissants, tous fournisseurs confondus.
    pub budget: Option<f64>,
    /// `None` : réglages non persistés (tests).
    pub settings_path: Option<PathBuf>,
}

pub struct App {
    sources: Vec<Source>,
    /// Dernier chargement réussi, dans l'ordre de `sources`.
    snapshots: Vec<Snapshot>,
    /// Incrémenté à chaque chargement reçu.
    generation: u64,
    dashboard: Option<(DashboardKey, Dashboard)>,
    budget: Option<f64>,
    settings_path: Option<PathBuf>,
    settings: Settings,
    save_due: Option<Instant>,
    show_settings: bool,
    updated: Option<DateTime<Local>>,
    loading: bool,
    last_resize: Option<Instant>,
    last_drag: Option<Instant>,
    /// Bord appuyé, en attente d'un glissé décidé pour lancer le redimensionnement.
    resize_armed: Option<ResizeDirection>,
    last_refresh: Instant,
    tx: Sender<Loaded>,
    rx: Receiver<Loaded>,
}

impl App {
    pub fn new(ctx: &egui::Context, config: Config) -> Self {
        theme::install(ctx);
        let settings = config.settings_path.as_deref().map(Settings::load).unwrap_or_default();
        let (tx, rx) = channel();
        let mut app = App {
            snapshots: vec![Snapshot::default(); config.providers.len()],
            sources: config.providers.into_iter().map(|p| Source { available: p.available(), provider: p }).collect(),
            generation: 0,
            dashboard: None,
            show_settings: false,
            budget: config.budget.filter(|b| *b > 0.0),
            settings_path: config.settings_path,
            settings,
            save_due: None,
            updated: None,
            loading: false,
            last_resize: None,
            last_drag: None,
            resize_armed: None,
            last_refresh: Instant::now(),
            tx,
            rx,
        };
        ctx.set_zoom_factor(app.settings.scale());
        app.refresh(ctx);
        app
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Agent affiché : choisi dans les réglages, sinon présent localement.
    fn shown(&self, i: usize) -> bool {
        match &self.settings.agents {
            Some(ids) => ids.iter().any(|id| id == self.sources[i].provider.id()),
            None => self.sources[i].available,
        }
    }

    fn shown_indices(&self) -> Vec<usize> {
        (0..self.sources.len()).filter(|&i| self.shown(i)).collect()
    }

    fn toggle_agent(&mut self, i: usize) {
        let mut ids: Vec<String> =
            self.shown_indices().into_iter().map(|k| self.sources[k].provider.id().to_string()).collect();
        let id = self.sources[i].provider.id();
        match ids.iter().position(|x| x == id) {
            Some(pos) => {
                ids.remove(pos);
            }
            None => ids.push(id.into()),
        }
        self.settings.agents = Some(ids);
    }

    fn refresh(&mut self, ctx: &egui::Context) {
        self.loading = true;
        self.last_refresh = Instant::now();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let providers: Vec<_> = self.sources.iter().map(|s| s.provider.clone()).collect();
        std::thread::spawn(move || {
            let since = Period::earliest();
            let loaded = providers.iter().map(|p| load(p.as_ref(), since)).collect();
            let _ = tx.send(loaded);
            ctx.request_repaint();
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if let Ok(loaded) = self.rx.try_recv() {
            for (snapshot, fresh) in self.snapshots.iter_mut().zip(loaded) {
                if let Some(fresh) = fresh {
                    *snapshot = fresh;
                }
            }
            self.generation += 1;
            self.updated = Some(Local::now());
            self.loading = false;
        }
        if let Some(every) = self.settings.auto_refresh.duration() {
            if !self.loading && self.last_refresh.elapsed() >= every {
                self.refresh(ctx);
            }
            ctx.request_repaint_after(every.saturating_sub(self.last_refresh.elapsed()));
        }
    }

    /// Ajuste la hauteur de fenêtre au contenu, bornée à l'écran, au plus une fois par
    /// `RESIZE_EVERY` : même si quelque chose dépendait de la hauteur, pas d'emballement.
    fn fit_window(&mut self, ctx: &egui::Context, content: f32) {
        let screen = ctx.input(|i| i.viewport().monitor_size).map_or(MAX_HEIGHT, |m| m.y - 80.0);
        let wanted = content.ceil().clamp(MIN_HEIGHT, screen.min(MAX_HEIGHT));
        if (wanted - ctx.viewport_rect().height()).abs() <= 1.0 {
            return;
        }
        match self.last_resize {
            Some(t) if t.elapsed() < RESIZE_EVERY => ctx.request_repaint_after(RESIZE_EVERY - t.elapsed()),
            _ => {
                self.last_resize = Some(Instant::now());
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(WIDTH, wanted)));
            }
        }
    }

    /// Fenêtre sans décoration : les bords (hors haut, réservé au déplacement) lancent un
    /// redimensionnement natif. Une taille choisie à la main désactive l'ajustement au contenu ;
    /// un double-clic sur un bord le rétablit. Comme `StartDrag`, `BeginResize` confie la souris
    /// au gestionnaire de fenêtres : une seule fois par appui, sinon GNOME Shell gèle.
    fn handle_resize(&mut self, ctx: &egui::Context) {
        let window = ctx.viewport_rect();
        // En points hors zoom, comme `with_inner_size` au démarrage.
        let unzoomed = window.size() * ctx.zoom_factor();
        let current = [unzoomed.x.round(), unzoomed.y.round()];
        let (hover, origin, pressed, released, dragging, double) = ctx.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.press_origin(),
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
                i.pointer.is_decidedly_dragging(),
                i.pointer.button_double_clicked(egui::PointerButton::Primary),
            )
        });
        if let Some(dir) = hover.and_then(|p| resize_direction(window, p)) {
            ctx.set_cursor_icon(resize_cursor(dir));
        }
        let edge = origin.and_then(|p| resize_direction(window, p));
        if double && edge.is_some() {
            self.settings.size = None;
            self.resize_armed = None;
        } else if pressed {
            self.resize_armed = edge;
        }
        if let Some(dir) = self.resize_armed.filter(|_| dragging) {
            self.resize_armed = None;
            if self.last_drag.is_none_or(|t| t.elapsed() >= DRAG_EVERY) {
                self.last_drag = Some(Instant::now());
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                self.settings.size = Some(current);
            }
        }
        if released {
            self.resize_armed = None;
        }
        if let Some(size) = &mut self.settings.size {
            *size = current;
        }
    }

    fn save_settings(&mut self) {
        self.save_due = None;
        let Some(path) = &self.settings_path else { return };
        if let Err(e) = self.settings.save(path) {
            log::warn!("réglages non enregistrés dans {} : {e}", path.display());
        }
    }

    /// Enregistre tout de suite un réglage, mais après `SAVE_DELAY` une taille qui suit la souris.
    fn persist(&mut self, ctx: &egui::Context, before: &Settings) {
        if self.settings != *before {
            ctx.set_zoom_factor(self.settings.scale());
            let resized_only = Settings { size: before.size, ..self.settings.clone() } == *before;
            if resized_only {
                self.save_due = Some(Instant::now() + SAVE_DELAY);
            } else {
                self.save_settings();
            }
        }
        if let Some(due) = self.save_due {
            if Instant::now() >= due || ctx.input(|i| i.viewport().close_requested()) {
                self.save_settings();
            } else {
                ctx.request_repaint_after(due - Instant::now());
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        let before = self.settings.clone();
        self.handle_resize(&ctx);
        let period = self.settings.period;
        let updated = self.updated.map(|t| t.format("%H:%M").to_string()).unwrap_or_default();

        let frame = egui::Frame::NONE
            .fill(*PANEL)
            .stroke(Stroke::new(1.0_f32, white(0.08)))
            .corner_radius(16.0)
            .inner_margin(Margin::same(PADDING));
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            ui.style_mut().interaction.selectable_labels = false;
            // Une seule fois par glissé, quand la souris bouge : le gestionnaire de fenêtres avale le
            // relâchement, egui croit le bouton toujours enfoncé et un StartDrag répété gèle GNOME Shell.
            let bg = ui.interact(ui.max_rect(), ui.id().with("background"), Sense::click_and_drag());
            if bg.drag_started() && self.last_drag.is_none_or(|t| t.elapsed() >= DRAG_EVERY) {
                self.last_drag = Some(Instant::now());
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }
            let status = if self.loading { "Actualisation…".to_string() } else { format!("Mis à jour {updated}") };
            let header = HeaderState {
                period,
                details: self.settings.details,
                settings: self.show_settings,
                loading: self.loading,
                status: &status,
            };
            match components::header(ui, header) {
                Some(HeaderAction::Refresh) => self.refresh(&ctx),
                Some(HeaderAction::Close) => {
                    if self.save_due.is_some() {
                        self.save_settings();
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                Some(HeaderAction::ToggleDetails) => self.settings.details = !self.settings.details,
                Some(HeaderAction::ToggleSettings) => self.show_settings = !self.show_settings,
                Some(HeaderAction::Period(p)) => self.settings.period = p,
                None => {}
            }
            ui.add_space(GAP);
            let header_bottom = ui.cursor().top();

            // Le contenu défile au lieu de dépendre de la hauteur de fenêtre : sa taille mesurée
            // ne change pas quand la fenêtre est redimensionnée, donc pas de boucle.
            let scroll = egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                if self.show_settings {
                    let agents: Vec<_> = self
                        .sources
                        .iter()
                        .enumerate()
                        .map(|(i, s)| Agent {
                            id: s.provider.id(),
                            name: s.provider.name(),
                            shown: self.shown(i),
                            available: s.available,
                        })
                        .collect();
                    let z = self.settings.zoom;
                    let panel = components::settings_panel(
                        ui,
                        &agents,
                        z,
                        z > ZOOM_MIN + 1e-3,
                        z < ZOOM_MAX - 1e-3,
                        self.settings.auto_refresh,
                    );
                    match panel {
                        Some(SettingsAction::Toggle(i)) => self.toggle_agent(i),
                        Some(SettingsAction::Zoom(steps)) => self.settings.zoom_by(steps),
                        Some(SettingsAction::AutoRefresh(a)) => self.settings.auto_refresh = a,
                        None => {}
                    }
                    ui.add_space(GAP);
                }

                if self.updated.is_none() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.add(egui::Spinner::new().size(24.0).color(*MUTED));
                        ui.add_space(8.0);
                        ui.label(RichText::new("Chargement…").size(12.0).color(*MUTED));
                    });
                    return;
                }

                let key = DashboardKey {
                    generation: self.generation,
                    period,
                    shown: self.shown_indices(),
                    today: Local::now().date_naive(),
                };
                let dash = cached_dashboard(&mut self.dashboard, key, &self.snapshots);
                let subscription = |i: usize| self.snapshots[i].subscription.as_ref();

                components::total(ui, dash.total, dash.tokens, &updated);
                ui.add_space(GAP);
                let parts: Vec<_> = dash
                    .sections
                    .iter()
                    .filter(|s| subscription(s.provider).is_none())
                    .map(|s| (s.summary.cost as f32, theme::shade(self.sources[s.provider].provider.id(), 0)))
                    .collect();
                components::stacked_bar(ui, &parts);
                ui.add_space(GAP);

                if dash.sections.is_empty() {
                    ui.label(RichText::new("Aucun agent sélectionné").size(12.0).color(*MUTED));
                    ui.add_space(GAP);
                }
                for s in &dash.sections {
                    let provider = &self.sources[s.provider].provider;
                    components::provider_section(
                        ui,
                        Section {
                            id: provider.id(),
                            name: provider.name(),
                            summary: &s.summary,
                            share: s.share,
                            buckets: &s.buckets,
                            granularity: period.granularity(),
                            details: self.settings.details,
                            subscription: subscription(s.provider),
                        },
                    );
                    ui.add_space(SECTION_GAP);
                }

                if let Some(budget) = self.budget {
                    components::budget(ui, dash.month_spent, budget);
                }
            });
            // Pas d'ajustement pendant le premier chargement (le loader rétrécirait la fenêtre)
            // ni quand la taille a été choisie à la main.
            if self.updated.is_some() && self.settings.size.is_none() {
                self.fit_window(&ctx, header_bottom + scroll.content_size.y + PADDING as f32);
            }
        });

        self.persist(&ctx, &before);
    }
}

fn cached_dashboard<'a>(
    cache: &'a mut Option<(DashboardKey, Dashboard)>,
    key: DashboardKey,
    snapshots: &[Snapshot],
) -> &'a Dashboard {
    if cache.as_ref().is_some_and(|(k, _)| *k != key) {
        *cache = None;
    }
    &cache
        .get_or_insert_with(|| {
            let dashboard = Dashboard::build(snapshots, &key.shown, key.period);
            (key, dashboard)
        })
        .1
}

/// Isole chaque fournisseur : s'il panique, il garde ses données précédentes et les autres
/// sont affichés normalement.
fn load(provider: &dyn Provider, since: DateTime<chrono::Utc>) -> Option<Snapshot> {
    let started = Instant::now();
    let snapshot = std::panic::catch_unwind(AssertUnwindSafe(|| Snapshot {
        entries: provider.load(since),
        subscription: provider.subscription(),
    }));
    match snapshot {
        Ok(s) => {
            log::debug!("{} : {} entrées en {:?}", provider.name(), s.entries.len(), started.elapsed());
            Some(s)
        }
        Err(_) => {
            log::error!("{} : échec du chargement, données précédentes conservées", provider.name());
            None
        }
    }
}

const RESIZE_EDGE: f32 = 8.0;

fn resize_direction(window: egui::Rect, p: egui::Pos2) -> Option<ResizeDirection> {
    let left = p.x <= window.left() + RESIZE_EDGE;
    let right = p.x >= window.right() - RESIZE_EDGE;
    let bottom = p.y >= window.bottom() - RESIZE_EDGE;
    match (left, right, bottom) {
        (_, true, true) => Some(ResizeDirection::SouthEast),
        (true, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, false) => Some(ResizeDirection::East),
        (true, _, false) => Some(ResizeDirection::West),
        (false, false, true) => Some(ResizeDirection::South),
        (false, false, false) => None,
    }
}

fn resize_cursor(dir: ResizeDirection) -> egui::CursorIcon {
    match dir {
        ResizeDirection::East => egui::CursorIcon::ResizeEast,
        ResizeDirection::West => egui::CursorIcon::ResizeWest,
        ResizeDirection::South => egui::CursorIcon::ResizeSouth,
        ResizeDirection::SouthWest => egui::CursorIcon::ResizeSouthWest,
        _ => egui::CursorIcon::ResizeSouthEast,
    }
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, Rect};
    use rstest::rstest;

    #[rstest]
    #[case(pos2(200.0, 300.0), None)]
    #[case(pos2(200.0, 2.0), None)]
    #[case(pos2(398.0, 300.0), Some(ResizeDirection::East))]
    #[case(pos2(2.0, 300.0), Some(ResizeDirection::West))]
    #[case(pos2(200.0, 598.0), Some(ResizeDirection::South))]
    #[case(pos2(398.0, 598.0), Some(ResizeDirection::SouthEast))]
    #[case(pos2(2.0, 598.0), Some(ResizeDirection::SouthWest))]
    fn edges_map_to_resize_directions(#[case] p: egui::Pos2, #[case] expected: Option<ResizeDirection>) {
        let window = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(400.0, 600.0));
        assert_eq!(resize_direction(window, p), expected);
    }

    #[test]
    fn dashboard_is_rebuilt_only_when_its_inputs_change() {
        let key = |generation| DashboardKey {
            generation,
            period: Period::Week,
            shown: vec![0],
            today: Local::now().date_naive(),
        };
        let spent = Snapshot {
            entries: vec![crate::domain::usage::Entry {
                time: chrono::Utc::now(),
                model: "a".into(),
                input: 1,
                output: 1,
                cost: 2.0,
            }],
            subscription: None,
        };
        let mut cache = None;
        assert_eq!(cached_dashboard(&mut cache, key(0), &[Snapshot::default()]).total, 0.0);
        assert_eq!(cached_dashboard(&mut cache, key(0), std::slice::from_ref(&spent)).total, 0.0, "en cache");
        assert_eq!(cached_dashboard(&mut cache, key(1), std::slice::from_ref(&spent)).total, 2.0, "nouvelles données");
    }
}
