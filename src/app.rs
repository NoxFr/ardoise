use crate::domain::period::Period;
use crate::domain::subscription::Subscription;
use crate::domain::usage::{histogram, summarize, Entry};
use crate::providers::Provider;
use crate::settings::{Settings, ZOOM_MAX, ZOOM_MIN};
use crate::ui::components::{self, Agent, HeaderAction, HeaderState, Section, SettingsAction};
use crate::ui::theme::{self, white, MUTED, PANEL};
use chrono::{DateTime, Local};
use eframe::egui::{self, Margin, ResizeDirection, RichText, Sense, Stroke, Ui, ViewportCommand};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const REFRESH_EVERY: Duration = Duration::from_secs(60);
pub const WIDTH: f32 = 440.0;
const PADDING: i8 = 18;
const GAP: f32 = 16.0;
const SECTION_GAP: f32 = 20.0;
const RESIZE_EVERY: Duration = Duration::from_millis(200);
const DRAG_EVERY: Duration = Duration::from_millis(500);
const MIN_HEIGHT: f32 = 120.0;
const MAX_HEIGHT: f32 = 1400.0;

/// Par fournisseur : appels facturés et état d'abonnement courant.
type Loaded = (Vec<Vec<Entry>>, Vec<Option<Subscription>>);

pub struct Config {
    pub providers: Vec<Arc<dyn Provider>>,
    /// Budget sur 30 jours glissants, tous fournisseurs confondus.
    pub budget: Option<f64>,
    /// `None` : réglages non persistés (tests).
    pub settings_path: Option<PathBuf>,
}

pub struct App {
    providers: Vec<Arc<dyn Provider>>,
    budget: Option<f64>,
    settings_path: Option<PathBuf>,
    settings: Settings,
    /// Par fournisseur, dans l'ordre de `providers`.
    entries: Vec<Vec<Entry>>,
    /// Par fournisseur : état d'abonnement courant, si le fournisseur en expose un.
    subscriptions: Vec<Option<Subscription>>,
    available: Vec<bool>,
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
            entries: vec![Vec::new(); config.providers.len()],
            subscriptions: vec![None; config.providers.len()],
            available: config.providers.iter().map(|p| p.available()).collect(),
            show_settings: false,
            providers: config.providers,
            budget: config.budget.filter(|b| *b > 0.0),
            settings_path: config.settings_path,
            settings,
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
            Some(ids) => ids.iter().any(|id| id == self.providers[i].id()),
            None => self.available[i],
        }
    }

    fn toggle_agent(&mut self, i: usize) {
        let mut ids: Vec<String> =
            (0..self.providers.len()).filter(|&k| self.shown(k)).map(|k| self.providers[k].id().to_string()).collect();
        let id = self.providers[i].id();
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
        let (tx, ctx, providers) = (self.tx.clone(), ctx.clone(), self.providers.clone());
        std::thread::spawn(move || {
            let since = Period::earliest();
            let entries = providers.iter().map(|p| p.load(since)).collect();
            let subscriptions = providers.iter().map(|p| p.subscription()).collect();
            let _ = tx.send((entries, subscriptions));
            ctx.request_repaint();
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if let Ok((entries, subscriptions)) = self.rx.try_recv() {
            self.entries = entries;
            self.subscriptions = subscriptions;
            self.updated = Some(Local::now());
            self.loading = false;
        }
        if !self.loading && self.last_refresh.elapsed() >= REFRESH_EVERY {
            self.refresh(ctx);
        }
        ctx.request_repaint_after(REFRESH_EVERY);
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

    fn save_settings(&self) {
        if let Some(path) = &self.settings_path {
            let _ = self.settings.save(path);
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
                Some(HeaderAction::Close) => ctx.send_viewport_cmd(ViewportCommand::Close),
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
                        .providers
                        .iter()
                        .enumerate()
                        .map(|(i, p)| Agent {
                            id: p.id(),
                            name: p.name(),
                            shown: self.shown(i),
                            available: self.available[i],
                        })
                        .collect();
                    let z = self.settings.zoom;
                    match components::settings_panel(ui, &agents, z, z > ZOOM_MIN + 1e-3, z < ZOOM_MAX - 1e-3) {
                        Some(SettingsAction::Toggle(i)) => self.toggle_agent(i),
                        Some(SettingsAction::Zoom(steps)) => self.settings.zoom_by(steps),
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

                let since = period.start();
                // Un agent sélectionné garde sa section même sans dépense.
                let mut sections: Vec<_> = (0..self.providers.len())
                    .filter(|&i| self.shown(i))
                    .map(|i| {
                        let mut summary = summarize(&self.entries[i], since);
                        if self.subscriptions[i].is_some() {
                            summary.models.sort_by_key(|m| std::cmp::Reverse(m.input + m.output));
                        }
                        (&self.providers[i], &self.entries[i], summary, &self.subscriptions[i])
                    })
                    .collect();
                sections.sort_by(|a, b| b.2.cost.total_cmp(&a.2.cost));
                // Un fournisseur en abonnement n'est pas facturé à l'appel : son estimation $ ne
                // rentre ni dans le total, ni dans la barre empilée, ni dans le budget.
                let metered = || sections.iter().filter(|(_, _, _, sub)| sub.is_none());
                let total: f64 = metered().map(|s| s.2.cost).sum();
                let tokens: u64 = sections.iter().map(|s| s.2.tokens).sum();

                components::total(ui, total, tokens, &updated);
                ui.add_space(GAP);
                let parts: Vec<_> = metered().map(|(p, _, s, _)| (s.cost as f32, theme::shade(p.id(), 0))).collect();
                components::stacked_bar(ui, &parts);
                ui.add_space(GAP);

                if sections.is_empty() {
                    ui.label(RichText::new("Aucun agent sélectionné").size(12.0).color(*MUTED));
                    ui.add_space(GAP);
                }
                let starts = period.bucket_starts();
                for (p, entries, summary, subscription) in &sections {
                    components::provider_section(
                        ui,
                        Section {
                            id: p.id(),
                            name: p.name(),
                            summary,
                            share: if subscription.is_some() || total <= 0.0 { 0.0 } else { summary.cost / total },
                            buckets: &if subscription.is_some() {
                                let tokens: Vec<Entry> = entries
                                    .iter()
                                    .map(|e| Entry { cost: (e.input + e.output) as f64, ..e.clone() })
                                    .collect();
                                histogram(&tokens, &starts)
                            } else {
                                histogram(entries, &starts)
                            },
                            granularity: period.granularity(),
                            details: self.settings.details,
                            subscription: subscription.as_ref(),
                        },
                    );
                    ui.add_space(SECTION_GAP);
                }

                if let Some(budget) = self.budget {
                    let month = Period::Month.start();
                    let spent: f64 = sections
                        .iter()
                        .filter(|(_, _, _, sub)| sub.is_none())
                        .map(|(_, e, _, _)| summarize(e, month).cost)
                        .sum();
                    components::budget(ui, spent, budget);
                }
            });
            // Pas d'ajustement pendant le premier chargement (le loader rétrécirait la fenêtre)
            // ni quand la taille a été choisie à la main.
            if self.updated.is_some() && self.settings.size.is_none() {
                self.fit_window(&ctx, header_bottom + scroll.content_size.y + PADDING as f32);
            }
        });

        if self.settings != before {
            ctx.set_zoom_factor(self.settings.scale());
            self.save_settings();
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
}
