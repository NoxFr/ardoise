mod common;

use ardoise::app::{App, Config};
use ardoise::domain::period::Period;
use ardoise::domain::refresh::AutoRefresh;
use ardoise::domain::usage::Entry;
use ardoise::settings::Settings;
use ardoise::ui::theme;
use chrono::Duration;
use common::{arc, entry, Fake};
use eframe::egui::{self, accesskit::Toggled, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use rstest::rstest;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

type H = Harness<'static, Option<App>>;

fn claude_sample() -> Vec<Entry> {
    vec![
        entry("Opus 5.5", Duration::minutes(1), 10.0),
        entry("Sonnet 5", Duration::days(3), 5.0),
        entry("Opus 5", Duration::days(20), 100.0),
        entry("Opus 4.5", Duration::days(200), 1000.0),
    ]
}

fn opencode_sample() -> Vec<Entry> {
    vec![entry("qwen3.6-35b-a3b", Duration::minutes(2), 0.42)]
}

/// Claude Code et OpenCode ont des données locales ; Codex n'est pas installé.
fn config(claude: Fake, opencode: Vec<Entry>, budget: Option<f64>, settings_path: Option<PathBuf>) -> Config {
    Config {
        providers: vec![
            arc(claude),
            arc(Fake::new("codex", "Codex", vec![]).unavailable()),
            arc(Fake::new("opencode", "OpenCode", opencode)),
        ],
        budget,
        settings_path,
    }
}

fn harness(budget: Option<f64>) -> (H, Arc<AtomicUsize>) {
    let claude = Fake::new("claude", "Claude Code", claude_sample());
    let calls = claude.calls.clone();
    (harness_with(config(claude, opencode_sample(), budget, None)), calls)
}

fn harness_with(config: Config) -> H {
    let mut h = Harness::builder().with_size(Vec2::new(ardoise::app::WIDTH, 1200.0)).build_ui_state(
        |ui: &mut egui::Ui, app: &mut Option<App>| {
            if let Some(app) = app {
                app.show(ui);
            }
        },
        None,
    );
    // Les polices ne sont prises en compte qu'au frame suivant.
    theme::install(&h.ctx);
    h.step();
    let ctx = h.ctx.clone();
    *h.state_mut() = Some(App::new(&ctx, config));
    wait_loaded(&mut h);
    h
}

fn wait_loaded(h: &mut H) {
    for _ in 0..200 {
        h.step();
        if h.state().as_ref().is_some_and(|a| !a.is_loading()) {
            h.step();
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("chargement jamais terminé");
}

fn click(h: &mut H, label: &str) {
    h.get_by_label(label).click();
    // Laisser la mise en page se stabiliser (changement d'échelle, redimensionnement).
    h.run_steps(4);
}

fn disabled(h: &H, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().is_disabled()
}

fn emitted(h: &H, pred: impl Fn(&egui::ViewportCommand) -> bool) -> bool {
    h.output().viewport_output[&egui::ViewportId::ROOT].commands.iter().any(pred)
}

fn first_rect(h: &H, label: &str) -> egui::Rect {
    h.get_all_by_label(label).next().unwrap_or_else(|| panic!("{label:?} introuvable")).rect()
}

fn open_settings(h: &mut H) {
    click(h, "Paramètres");
    h.get_by_label("Agents affichés");
}

/// Case « agent » du panneau de réglages (le nom apparaît aussi en titre de section).
fn agent_checkbox(h: &H, name: &str) -> Toggled {
    h.get_all_by_label(name)
        .find_map(|n| n.accesskit_node().toggled())
        .unwrap_or_else(|| panic!("case {name:?} introuvable"))
}

fn click_checkbox(h: &mut H, name: &str) {
    let node = h.get_all_by_label(name).find(|n| n.accesskit_node().toggled().is_some()).unwrap();
    node.click();
    h.run_steps(4);
}

// Affichage

#[test]
fn shows_every_provider_for_the_last_seven_days() {
    let (h, _) = harness(None);

    h.get_by_label("$15,42");
    h.get_by_label("Claude Code");
    h.get_by_label("OpenCode");
    h.get_by_label("Opus 5.5");
    h.get_by_label("Sonnet 5");
    h.get_by_label("qwen3.6-35b-a3b");
    assert!(h.query_by_label("Opus 5").is_none(), "hors période");
}

#[test]
fn providers_are_sorted_by_cost() {
    let (h, _) = harness(None);
    assert!(first_rect(&h, "Claude Code").top() < first_rect(&h, "OpenCode").top());

    let claude = Fake::new("claude", "Claude Code", vec![entry("Opus 5.5", Duration::minutes(1), 0.1)]);
    let h = harness_with(config(claude, opencode_sample(), None, None));
    assert!(first_rect(&h, "OpenCode").top() < first_rect(&h, "Claude Code").top());
}

#[test]
fn shows_shares_of_total_and_of_provider() {
    let (h, _) = harness(None);
    h.get_by_label("97%"); // Claude Code : 15 / 15,42
    h.get_by_label("3%"); // OpenCode
}

#[test]
fn selected_agent_is_shown_even_without_spend() {
    let claude = Fake::new("claude", "Claude Code", claude_sample());
    let h = harness_with(config(claude, vec![], None, None));
    h.get_by_label("OpenCode");
    h.get_by_label("Aucune dépense sur la période");
}

#[test]
fn agents_without_local_data_are_hidden_by_default() {
    let (h, _) = harness(None);
    assert!(h.query_by_label("Codex").is_none());
}

#[test]
fn switching_period_updates_totals() {
    let (mut h, _) = harness(None);

    click(&mut h, "30 j");
    h.get_by_label("$115,42");
    h.get_by_label("Opus 5");

    click(&mut h, "Jour");
    h.get_by_label("$10,42");
    assert_eq!(h.get_all_by_label("Dépenses par heure").count(), 2, "un graphique par fournisseur");
    assert!(h.query_by_label("Sonnet 5").is_none());
}

#[test]
fn year_period_groups_by_month() {
    let (mut h, _) = harness(None);

    click(&mut h, "1 an");

    h.get_by_label("$1115,42");
    h.get_by_label("Opus 4.5");
    assert_eq!(h.get_all_by_label("Dépenses par mois").count(), 2);
}

#[test]
fn shows_empty_sections_when_nothing_was_spent() {
    let h = harness_with(config(Fake::new("claude", "Claude Code", vec![]), vec![], None, None));
    assert_eq!(h.get_all_by_label("$0,00").count(), 3, "total + deux sections");
    assert_eq!(h.get_all_by_label("Aucune dépense sur la période").count(), 2);
}

#[test]
fn budget_covers_all_providers_over_thirty_days() {
    let (h, _) = harness(None);
    assert!(h.query_by_label("Budget 30 j").is_none());

    let (h, _) = harness(Some(1200.0));
    h.get_by_label("Budget 30 j");
    h.get_by_label("$115,42 / $1200,00");
}

#[test]
fn details_toggle_shows_token_breakdown() {
    let (mut h, _) = harness(None);
    assert!(h.query_by_label_contains("↓").is_none());

    click(&mut h, "Détails");

    assert!(h.get_all_by_label("↓ 1 k").count() >= 3);
    h.get_by_label("67%"); // Opus 5.5 : 10 / 15 de Claude Code
}

// Actions

#[test]
fn refresh_button_reloads() {
    let (mut h, calls) = harness(None);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    click(&mut h, "Rafraîchir");
    wait_loaded(&mut h);

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn refresh_is_disabled_while_loading() {
    let (claude, release) = Fake::new("claude", "Claude Code", claude_sample()).gated();
    let calls = claude.calls.clone();
    let mut h = harness_with(config(claude, opencode_sample(), None, None));

    click(&mut h, "Rafraîchir");
    assert!(disabled(&h, "Rafraîchir"));
    click(&mut h, "Rafraîchir");

    release.send(()).unwrap();
    wait_loaded(&mut h);
    assert!(!disabled(&h, "Rafraîchir"));
    assert_eq!(calls.load(Ordering::SeqCst), 2, "le second clic pendant le chargement est ignoré");
}

#[test]
fn close_button_closes_window() {
    let (mut h, _) = harness(None);

    h.get_by_label("Fermer").click();
    h.step();

    assert!(emitted(&h, |c| matches!(c, egui::ViewportCommand::Close)));
}

#[test]
fn selected_period_is_exposed_to_accessibility() {
    let (mut h, _) = harness(None);
    let toggled = |h: &H, l| h.get_by_label(l).accesskit_node().toggled();
    assert_eq!(toggled(&h, "7 j"), Some(Toggled::True));

    click(&mut h, "30 j");

    assert_eq!(toggled(&h, "30 j"), Some(Toggled::True));
    assert_eq!(toggled(&h, "7 j"), Some(Toggled::False));
}

// Réglages

#[test]
fn settings_panel_lists_agents_with_their_state() {
    let (mut h, _) = harness(None);
    assert!(h.query_by_label("Agents affichés").is_none());

    open_settings(&mut h);

    assert_eq!(agent_checkbox(&h, "Claude Code"), Toggled::True);
    assert_eq!(agent_checkbox(&h, "Codex"), Toggled::False);
    assert_eq!(agent_checkbox(&h, "OpenCode"), Toggled::True);

    click(&mut h, "Paramètres");
    assert!(h.query_by_label("Agents affichés").is_none());
}

#[test]
fn unchecking_an_agent_hides_it_and_excludes_it_from_totals() {
    let (mut h, _) = harness(Some(1200.0));
    open_settings(&mut h);

    click_checkbox(&mut h, "OpenCode");

    assert_eq!(agent_checkbox(&h, "OpenCode"), Toggled::False);
    assert_eq!(h.get_all_by_label("OpenCode").count(), 1, "plus que la case, plus de section");
    assert_eq!(h.get_all_by_label("$15,00").count(), 2, "total = seule section restante");
    h.get_by_label("$115,00 / $1200,00");
}

#[test]
fn checking_an_agent_without_data_shows_an_empty_section() {
    let (mut h, _) = harness(None);
    open_settings(&mut h);

    click_checkbox(&mut h, "Codex");

    assert_eq!(h.get_all_by_label("Codex").count(), 2, "case + section");
    h.get_by_label("Aucune dépense sur la période");
}

#[test]
fn unchecking_everything_shows_a_hint() {
    let (mut h, _) = harness(None);
    open_settings(&mut h);
    click_checkbox(&mut h, "Claude Code");
    click_checkbox(&mut h, "OpenCode");
    h.get_by_label("Aucun agent sélectionné");
}

#[test]
fn zoom_scales_the_widget() {
    let (mut h, _) = harness(None);
    open_settings(&mut h);
    h.get_by_label("100 %");
    assert!((h.ctx.zoom_factor() - 1.0).abs() < 1e-6);

    click(&mut h, "+");

    h.get_by_label("110 %");
    assert!((h.ctx.zoom_factor() - 1.1).abs() < 1e-6);
}

#[test]
fn zoom_is_bounded() {
    let (mut h, _) = harness(None);
    open_settings(&mut h);
    for _ in 0..4 {
        click(&mut h, "−");
    }
    h.get_by_label("60 %");
    assert!(disabled(&h, "−"));
    assert!(!disabled(&h, "+"));
}

#[test]
fn auto_refresh_interval_can_be_changed_and_is_persisted() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let make =
        || config(Fake::new("claude", "Claude Code", claude_sample()), opencode_sample(), None, Some(path.clone()));
    let mut h = harness_with(make());
    open_settings(&mut h);
    let toggled = |h: &H, l| h.get_by_label(l).accesskit_node().toggled();
    assert_eq!(toggled(&h, "1 min"), Some(Toggled::True), "réglage par défaut");

    click(&mut h, "Off");

    assert_eq!(toggled(&h, "Off"), Some(Toggled::True));
    assert_eq!(Settings::load(&path).auto_refresh, AutoRefresh::Off);

    let mut h = harness_with(make());
    open_settings(&mut h);
    assert_eq!(toggled(&h, "Off"), Some(Toggled::True), "restauré au redémarrage");
}

#[test]
fn settings_are_persisted_and_restored() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let make =
        || config(Fake::new("claude", "Claude Code", claude_sample()), opencode_sample(), None, Some(path.clone()));
    let mut h = harness_with(make());

    click(&mut h, "30 j");
    click(&mut h, "Détails");
    open_settings(&mut h);
    click_checkbox(&mut h, "Codex");
    click(&mut h, "+");

    let saved = Settings::load(&path);
    assert_eq!((saved.period, saved.details), (Period::Month, true));
    assert_eq!(saved.agents.as_deref(), Some(&["claude".to_string(), "opencode".into(), "codex".into()][..]));
    assert!((saved.zoom - 1.1).abs() < 1e-6);

    let h = harness_with(make());
    h.get_by_label("$115,42");
    h.get_by_label("Codex");
    assert!(h.query_all_by_label_contains("↓").count() > 0);
    assert!((h.ctx.zoom_factor() - 1.1).abs() < 1e-6);
}

// Souris : ni clic avalé, ni composant qui bouge

/// Appuie sans relâcher : sous X11, `StartDrag` confie la souris au gestionnaire de fenêtres,
/// qui avale le relâchement.
fn press_started_window_drag(h: &mut H, label: &str, moved: f32) -> bool {
    let pos = first_rect(h, label).center();
    h.event(egui::Event::PointerMoved(pos));
    h.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    if moved > 0.0 {
        h.event(egui::Event::PointerMoved(pos + egui::vec2(moved, moved)));
    }
    let mut started = false;
    for _ in 0..3 {
        h.step();
        started |= emitted(h, |c| matches!(c, egui::ViewportCommand::StartDrag));
    }
    started
}

#[rstest]
#[case("30 j")]
#[case("Détails")]
#[case("Rafraîchir")]
#[case("Paramètres")]
#[case("Fermer")]
fn pressing_a_button_does_not_start_window_drag(#[case] label: &str) {
    let (mut h, _) = harness(None);
    assert!(!press_started_window_drag(&mut h, label, 0.0));
}

#[test]
fn dragging_the_background_moves_the_window() {
    let (mut h, _) = harness(None);
    assert!(!press_started_window_drag(&mut h, "Dépenses par jour", 0.0));

    let (mut h, _) = harness(None);
    assert!(press_started_window_drag(&mut h, "Dépenses par jour", 20.0));
}

/// Après un `StartDrag`, egui croit le bouton toujours enfoncé : le renvoyer à chaque frame
/// gelait GNOME Shell.
#[test]
fn window_drag_is_started_once_per_gesture() {
    let (mut h, _) = harness(None);
    assert!(press_started_window_drag(&mut h, "Dépenses par jour", 20.0));
    let pos = first_rect(&h, "Dépenses par jour").center();
    let mut again = 0;
    for k in 0..30 {
        h.event(egui::Event::PointerMoved(pos + egui::vec2(k as f32, 0.0)));
        h.step();
        again += emitted(&h, |c| matches!(c, egui::ViewportCommand::StartDrag)) as usize;
    }
    assert_eq!(again, 0);
}

#[test]
fn layout_is_stable_across_frames() {
    let (mut h, _) = harness(None);
    let before = first_rect(&h, "OpenCode");
    h.run_steps(10);
    assert_eq!(first_rect(&h, "OpenCode"), before, "le contenu ne doit pas dériver d'un frame à l'autre");
}

fn resize_requests(h: &H) -> Vec<Vec2> {
    h.output().viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .filter_map(|c| match c {
            egui::ViewportCommand::InnerSize(s) => Some(*s),
            _ => None,
        })
        .collect()
}

/// kittest applique les `InnerSize` comme une vraie fenêtre : une boucle de redimensionnement
/// (contenu dépendant de la hauteur de fenêtre) se verrait par des demandes à chaque frame.
#[test]
fn window_settles_to_content_height() {
    let (mut h, _) = harness(Some(1200.0));
    for _ in 0..10 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    let settled = h.ctx.viewport_rect().height();

    let mut requests = 0;
    for _ in 0..20 {
        h.step();
        requests += resize_requests(&h).len();
        std::thread::sleep(std::time::Duration::from_millis(30));
    }

    assert_eq!(requests, 0, "plus aucune demande une fois stabilisé");
    let bottom = first_rect(&h, "Budget 30 j").bottom();
    assert!(bottom < settled, "le budget est visible ({bottom} < {settled})");
    assert!(settled - bottom < 60.0, "pas de vide en bas ({bottom} vs {settled})");
}

/// Fenêtre de largeur choisie à la main (sinon l'ajustement automatique la ramène à `WIDTH`).
fn harness_at_width(width: f32) -> (H, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    Settings { size: Some([width, 900.0]), ..Default::default() }.save(&path).unwrap();
    let config = config(Fake::new("claude", "Claude Code", claude_sample()), opencode_sample(), None, Some(path));
    let mut h = Harness::builder().with_size(Vec2::new(width, 900.0)).build_ui_state(
        |ui: &mut egui::Ui, app: &mut Option<App>| {
            if let Some(app) = app {
                app.show(ui);
            }
        },
        None,
    );
    theme::install(&h.ctx);
    h.step();
    let ctx = h.ctx.clone();
    *h.state_mut() = Some(App::new(&ctx, config));
    wait_loaded(&mut h);
    (h, dir)
}

#[rstest]
#[case(ardoise::app::WIDTH)]
#[case(360.0)]
#[case(320.0)]
fn header_items_do_not_overlap(#[case] width: f32) {
    let (h, _dir) = harness_at_width(width);
    let items = ["Ardoise", "Détails", "Jour", "7 j", "30 j", "1 an", "Paramètres", "Rafraîchir", "Fermer"];
    let rects: Vec<_> = items.iter().map(|l| (l, first_rect(&h, l))).collect();
    for (i, (a, ra)) in rects.iter().enumerate() {
        assert!(ra.left() >= 0.0 && ra.right() <= width, "{a:?} déborde de la fenêtre ({ra:?})");
        for (b, rb) in &rects[i + 1..] {
            assert!(ra.intersect(*rb).area() < 1.0 || !ra.intersects(*rb), "{a:?} chevauche {b:?}");
        }
    }
}

#[test]
fn header_fits_on_one_line_at_default_width() {
    let (h, _) = harness(None);
    let (title, jour) = (first_rect(&h, "Ardoise"), first_rect(&h, "Jour"));
    assert!(jour.top() < title.bottom(), "une seule ligne : {title:?} / {jour:?}");
    let (h, _dir) = harness_at_width(320.0);
    let (title, jour) = (first_rect(&h, "Ardoise"), first_rect(&h, "Jour"));
    assert!(jour.top() > title.bottom(), "période sous le titre");
    assert!(jour.top() - title.bottom() < 20.0, "juste sous le titre, pas au milieu de la fenêtre : {jour:?}");
}

#[test]
fn hovering_does_not_move_components() {
    let (mut h, _) = harness(None);
    let labels = ["Détails", "Jour", "7 j", "30 j", "Paramètres", "Rafraîchir", "Fermer", "Claude Code", "$15,42"];
    let rects = |h: &H| labels.map(|l| first_rect(h, l));
    let before = rects(&h);

    for label in labels {
        h.event(egui::Event::PointerMoved(first_rect(&h, label).center()));
        h.run_steps(2);
        assert_eq!(rects(&h), before, "survol de {label:?}");
    }
}

// Abonnement et chargement

fn team_plan() -> ardoise::domain::subscription::Subscription {
    ardoise::domain::subscription::Subscription { plan: "team_standard".into(), primary: None, secondary: None }
}

#[test]
fn subscribed_provider_shows_tokens_instead_of_cost() {
    let claude = Fake::new("claude", "Claude Code", claude_sample()).subscribed(team_plan());
    let h = harness_with(config(claude, opencode_sample(), Some(1200.0), None));

    h.get_by_label("Abonnement · team_standard");
    assert!(h.query_by_label("$15,42").is_none(), "l'abonnement sort du total");
    assert!(h.query_by_label("$15,00").is_none(), "pas de $ pour la section en abonnement");
    assert!(h.query_by_label("$10,00").is_none(), "ni pour ses modèles");
    h.get_by_label("2 k");
    h.get_by_label("$0,42 / $1200,00");
    h.get_by_label("Tokens par jour");
    assert!(h.query_by_label("max $10,00").is_none(), "graphique en tokens");
}

#[test]
fn subscribed_provider_details_share_tokens_not_cost() {
    let claude = Fake::new("claude", "Claude Code", claude_sample()).subscribed(team_plan());
    let mut h = harness_with(config(claude, opencode_sample(), None, None));

    click(&mut h, "Détails");

    assert_eq!(h.get_all_by_label("50%").count(), 2, "Opus 5.5 et Sonnet 5 : autant de tokens");
    assert!(h.query_by_label("67%").is_none(), "pas de part au coût");
}

/// Premier chargement bloqué jusqu'à `release`.
struct Slow(std::sync::Mutex<std::sync::mpsc::Receiver<()>>);

impl ardoise::providers::Provider for Slow {
    fn id(&self) -> &'static str {
        "claude"
    }
    fn name(&self) -> &'static str {
        "Claude Code"
    }
    fn available(&self) -> bool {
        true
    }
    fn load(&self, _: chrono::DateTime<chrono::Utc>) -> Vec<Entry> {
        self.0.lock().unwrap().recv().ok();
        claude_sample()
    }
}

#[test]
fn shows_a_loader_until_the_first_load() {
    let (release, rx) = std::sync::mpsc::channel();
    let config =
        Config { providers: vec![Arc::new(Slow(std::sync::Mutex::new(rx)))], budget: None, settings_path: None };
    let mut h = Harness::builder().with_size(Vec2::new(ardoise::app::WIDTH, 1200.0)).build_ui_state(
        |ui: &mut egui::Ui, app: &mut Option<App>| {
            if let Some(app) = app {
                app.show(ui);
            }
        },
        None,
    );
    theme::install(&h.ctx);
    h.step();
    let ctx = h.ctx.clone();
    *h.state_mut() = Some(App::new(&ctx, config));
    h.run_steps(3);
    h.get_by_label("Chargement…");
    assert!(h.query_by_label("Claude Code").is_none());

    release.send(()).unwrap();
    wait_loaded(&mut h);
    assert!(h.query_by_label("Chargement…").is_none());
    h.get_by_label("Claude Code");
}

// Redimensionnement

#[test]
fn dragging_an_edge_resizes_and_keeps_the_size() {
    let (mut h, _) = harness(None);
    assert!(h.state().as_ref().unwrap().settings().size.is_none());
    let corner = egui::pos2(ardoise::app::WIDTH - 2.0, 1198.0);
    h.event(egui::Event::PointerMoved(corner));
    h.event(egui::Event::PointerButton {
        pos: corner,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    h.event(egui::Event::PointerMoved(corner + egui::vec2(20.0, 20.0)));
    let (mut resize, mut drag) = (false, false);
    for _ in 0..3 {
        h.step();
        resize |= emitted(&h, |c| matches!(c, egui::ViewportCommand::BeginResize(egui::ResizeDirection::SouthEast)));
        drag |= emitted(&h, |c| matches!(c, egui::ViewportCommand::StartDrag));
    }
    assert!(resize && !drag);
    assert!(h.state().as_ref().unwrap().settings().size.is_some());

    // Le relâchement est avalé par le gestionnaire de fenêtres : pas de nouveau BeginResize.
    let mut again = 0;
    for k in 0..30 {
        h.event(egui::Event::PointerMoved(corner + egui::vec2(k as f32, 0.0)));
        h.step();
        again += emitted(&h, |c| matches!(c, egui::ViewportCommand::BeginResize(_))) as usize;
    }
    assert_eq!(again, 0);
}

// Robustesse

/// Premier chargement normal, puis panique à chaque rechargement.
struct Flaky(AtomicUsize);

impl ardoise::providers::Provider for Flaky {
    fn id(&self) -> &'static str {
        "claude"
    }
    fn name(&self) -> &'static str {
        "Claude Code"
    }
    fn available(&self) -> bool {
        true
    }
    fn load(&self, _: chrono::DateTime<chrono::Utc>) -> Vec<Entry> {
        if self.0.fetch_add(1, Ordering::SeqCst) > 0 {
            panic!("transcript illisible");
        }
        claude_sample()
    }
}

#[test]
fn a_panicking_provider_keeps_its_data_and_does_not_block_loading() {
    let config = Config {
        providers: vec![
            Arc::new(Flaky(AtomicUsize::new(0))),
            arc(Fake::new("opencode", "OpenCode", opencode_sample())),
        ],
        budget: None,
        settings_path: None,
    };
    let mut h = harness_with(config);
    h.get_by_label("$15,42");

    click(&mut h, "Rafraîchir");
    wait_loaded(&mut h);

    assert!(!disabled(&h, "Rafraîchir"), "le chargement se termine malgré la panique");
    h.get_by_label("$15,42");
}

#[test]
fn manual_size_is_saved_once_the_resize_settles() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let mut h = harness_with(config(
        Fake::new("claude", "Claude Code", claude_sample()),
        opencode_sample(),
        None,
        Some(path.clone()),
    ));
    let corner = egui::pos2(ardoise::app::WIDTH - 2.0, 1198.0);
    h.event(egui::Event::PointerMoved(corner));
    h.event(egui::Event::PointerButton {
        pos: corner,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    h.event(egui::Event::PointerMoved(corner + egui::vec2(20.0, 20.0)));
    h.run_steps(3);
    assert!(h.state().as_ref().unwrap().settings().size.is_some());
    assert!(Settings::load(&path).size.is_none(), "pas d'écriture pendant le redimensionnement");

    std::thread::sleep(std::time::Duration::from_millis(600));
    h.step();

    assert!(Settings::load(&path).size.is_some());
}
