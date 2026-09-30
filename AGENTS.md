# AGENTS.md

**Ardoise** : widget de bureau Linux (Rust + egui/eframe 0.36) qui affiche la consommation des
agents de code (Claude Code, Codex, OpenCode) à partir de leurs données locales.
La maquette de référence est un fichier HTML (thème sombre, couleurs OKLCH) reproduit dans `ui/theme.rs`.

## Commandes

```bash
cargo run --release                     # lance le widget
RUST_LOG=ardoise=debug cargo run        # journal des chargements (durée, nombre d'entrées)
ARDOISE_BUDGET=1200 cargo run           # avec barre « Budget 30 j » (tous agents affichés)
cargo test                              # unitaires + intégration + propriétés + UI
cargo clippy --all-targets              # doit rester sans warning
cargo fmt                               # rustfmt.toml : max_width = 120
cargo install --path . --root ~/.local --locked --force   # installe ~/.local/bin/ardoise
```

## Architecture

```
src/
  main.rs                 fenêtre, icône, Config (fournisseurs, budget, chemin des réglages)
  lib.rs                  expose les modules pour les tests d'intégration
  app.rs                  état, chargement en thread, assemblage des composants
  settings.rs             réglages persistés (~/.config/ardoise/settings.json) : période, détails,
                          agents affichés, zoom (1.0 = taille de la maquette), taille manuelle
  domain/                 logique pure, sans I/O ni egui
    dashboard.rs          Snapshot (données d'un fournisseur), Dashboard::build : sections triées,
                          parts, total facturé, dépense 30 j ; calculé une fois par changement
    usage.rs              Entry, summarize (par modèle), histogram(_by) (tranches aux dates données)
    period.rs             Jour / 7 j / 30 j / 1 an, débuts des barres (heures, jours, mois civils)
    subscription.rs       abonnement forfaitaire : plan + fenêtres de quota (used_percent, reset)
  providers/              une source de données par agent, derrière le trait `Provider`
    mod.rs                trait Provider { id, name, available, load, subscription } + all()
    files.rs              parcours des .jsonl, FileCache (analyse réutilisée tant que date et taille
                          du fichier sont inchangées)
    claude/               ~/.claude/projects/**/*.jsonl, tarif API public Anthropic
    codex/                ~/.codex/sessions/**/rollout-*.jsonl, tarif API public OpenAI
    opencode/             ~/.local/share/opencode/opencode.db (SQLite, lecture seule), coût fourni
  ui/
    theme.rs              jetons de la maquette (OKLCH → sRGB), nuances par modèle, Inter embarquée
    format.rs             formatage fr ($399,70 · 56,1 M · 85% · « oct. 25 »)
    components/           un fichier par composant : header, segmented, settings_panel, total,
                          stacked_bar, provider_section, model_row, chart, budget, subscription,
                          gauge, logos, cell
```

Dépendances : `domain` ne dépend de rien ; `providers` dépend de `domain` ; `ui` dépend de
`domain` ; seul `app.rs` assemble tout.

## Conventions

- Composant UI = fonction libre `fn nom(ui: &mut Ui, …)` dans son propre fichier, réexportée par
  `ui/components/mod.rs`. Les actions remontent par valeur de retour (`HeaderAction`,
  `SettingsAction`), pas par callback.
- Aucune I/O dans `domain`. Les fournisseurs sont injectés via `Config::providers` : les tests
  utilisent un `Fake` (`tests/common/mod.rs`).
- Nouveau fournisseur : un module sous `providers/`, l'ajouter dans `providers::all()`, une teinte
  dans `theme::provider_hue`, un pictogramme dans `components/logos.rs`, des tests d'intégration.
- Nouveau modèle : ajouter le préfixe dans la `TABLE` de prix du fournisseur, le plus spécifique
  en premier, plus un cas `rstest`. Un modèle inconnu garde ses tokens mais coûte 0.
- Textes UI en français, code et identifiants en anglais. Peu de commentaires.
- Les libellés doivent rester des widgets (via `cell`) et non du texte peint, pour être visibles
  d'AccessKit et donc des tests.

## Tests

| Couche | Emplacement | Outil |
|---|---|---|
| Unitaires | `#[cfg(test)]` dans chaque module | `rstest` (cas paramétrés) |
| Intégration I/O | `tests/claude_provider.rs`, `tests/opencode_provider.rs` | `tempfile`, `rusqlite` |
| Propriétés | `tests/properties.rs` | `proptest` |
| UI | `tests/ui.rs` | `egui_kittest` (requêtes AccessKit, clics) |

Toute modification doit s'accompagner du test correspondant. Vérifier aussi le rendu réel par
capture d'écran : certains défauts (chevauchements, contrastes) ne se voient qu'à l'œil.

## Pièges connus

- Une somme `f64` vide vaut `-0.0` : `format::money` la normalise.
- Claude Code répète une même réponse sur plusieurs lignes (streaming) et entre sessions reprises :
  on dédoublonne (message.id + requestId) et on garde la dernière ligne.
- Codex écrit des compteurs **cumulés** (`total_token_usage`), parfois plusieurs fois par tour, et
  un rollout repris hérite du total parent : on compte les différences, et seulement
  `last_token_usage` pour le premier événement d'un fichier.
- Hauteur de fenêtre auto = boucle mesure → redimensionnement → nouveau rendu. Si le contenu
  dépend de la hauteur de fenêtre (ex. `Layout::left_to_right(Align::Max)` qui prend toute la
  hauteur), la fenêtre grandit à chaque frame et l'app gèle. Garde-fous dans `App::fit_window` :
  contenu dans une `ScrollArea` (taille mesurée indépendante de la fenêtre), comparaison à la taille
  réelle (`viewport_rect`), au plus un redimensionnement toutes les 200 ms, plafond à l'écran.
  `window_settles_to_content_height` et `layout_is_stable_across_frames` le vérifient.
- `set_fonts` et `set_zoom_factor` ne s'appliquent qu'au frame suivant : dans les tests, appeler
  `theme::install` et faire un `step()` avant de créer l'App, et laisser quelques frames après un
  changement de taille.
- Le style egui par défaut agrandit les widgets au survol (`expansion`) : `theme::install` le
  neutralise ; `hovering_does_not_move_components` le vérifie.
- Fenêtre sans décoration : n'envoyer `StartDrag` qu'une fois le glissé décidé
  (`is_decidedly_dragging`). Sous X11 le gestionnaire de fenêtres capture la souris et avale le
  relâchement : un `StartDrag` dès l'appui casse tous les boutons. Labels non sélectionnables.
  Comme le relâchement est avalé, egui croit ensuite le bouton enfoncé : `StartDrag` une seule fois
  par geste (`drag_started`, plus un délai minimal), sinon il part à chaque frame et gèle GNOME Shell
  (`window_drag_is_started_once_per_gesture`). Même règle pour `BeginResize` (bords gauche, droit,
  bas) : armé à l'appui, envoyé une fois. Une taille manuelle (`Settings::size`, en points hors
  zoom) coupe `fit_window` ; double-clic sur un bord pour revenir à la hauteur auto.
- kittest ne garde que la sortie du dernier frame d'un `step()` : pour tester une commande de
  viewport émise à l'appui, injecter les événements à la main.
- AccessKit expose l'état « sélectionné » d'egui comme `toggled`, pas `selected`.
- Les coûts Claude et Codex sont des estimations au tarif API public ; ils ne couvrent pas les
  interfaces web ni les autres machines, et diffèrent d'une facturation Enterprise.
- Abonnement : un fournisseur dont `subscription()` renvoie `Some` affiche des tokens au lieu de $
  et sort du total, de la barre empilée et du budget. Codex lit `rate_limits` des rollouts (vraies
  jauges) ; Claude Code n'a que `~/.claude.json` → `oauthAccount.billingType` du compte *courant* :
  les transcripts ne portent aucun identifiant de compte, impossible de séparer l'historique.
- Rendu : egui redessine à chaque mouvement de souris. Rien de proportionnel à l'historique dans
  `App::show` : tout passe par `Dashboard`, mis en cache par (chargement, période, agents, jour).
- Chargement : chaque fournisseur tourne sous `catch_unwind` ; s'il panique, il garde ses données
  précédentes et les autres s'affichent (`a_panicking_provider_keeps_its_data_…`). Pas de
  `panic = "abort"` en release, sinon une panique ferme le widget.
- Les fournisseurs à fichiers gardent un `FileCache` : un rafraîchissement ne relit que les
  transcripts ajoutés ou modifiés (≈ 8 ms au lieu de ≈ 300 ms pour 300 Mo).
- Réglages : écrits tout de suite, sauf une taille qui suit la souris (après 500 ms de calme, ou à
  la fermeture) ; `manual_size_is_saved_once_the_resize_settles`.
- Premier chargement : loader, et pas de `fit_window` tant que rien n'est chargé (sinon la fenêtre
  se réduit au loader).
