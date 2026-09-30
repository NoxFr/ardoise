# Ardoise

[![CI](https://github.com/NoxFr/ardoise/actions/workflows/ci.yml/badge.svg)](https://github.com/NoxFr/ardoise/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-2021-orange?logo=rust)
![Linux](https://img.shields.io/badge/plateforme-Linux-blue?logo=linux&logoColor=white)
![Licence](https://img.shields.io/badge/licence-MIT%20%2F%20Apache--2.0-green)

Petit widget de bureau pour suivre ce que coûtent vos agents de code : Claude Code, Codex
et OpenCode. Tout est lu en local, rien ne part sur le réseau.

<p align="center"><img src="assets/screenshot.png" width="360" alt="Ardoise, période 30 jours, trois agents"></p>

## Ce qu'il lit

| Agent | Source | Coût |
|---|---|---|
| Claude Code | `~/.claude/projects/**/*.jsonl` | estimé au tarif API Anthropic |
| Codex | `~/.codex/sessions/**/rollout-*.jsonl` | estimé au tarif API OpenAI |
| OpenCode | `~/.local/share/opencode/opencode.db` | celui qu'enregistre OpenCode |

Les chemins suivent `CLAUDE_CONFIG_DIR`, `CODEX_HOME` et `XDG_DATA_HOME` s'ils sont définis.

## Installation

### Depuis une release

Télécharge `ardoise-linux-x86_64.tar.gz` sur la [page des releases](https://github.com/NoxFr/ardoise/releases),
puis :

```bash
tar xzf ardoise-linux-x86_64.tar.gz
./install.sh
```

Le script installe le binaire, l'icône et le lanceur selon la spécification XDG (`~/.local/bin`,
`~/.local/share/icons`, `~/.local/share/applications`, sans droits root). Ardoise apparaît ensuite
dans le menu des applications, ou se lance avec `ardoise`.

### Depuis les sources

Il faut une toolchain Rust 1.95 ou plus récente ([rustup](https://rustup.rs)).

```bash
git clone https://github.com/NoxFr/ardoise && cd ardoise
cargo install --path . --root ~/.local --locked
install -Dm644 assets/icon.png ~/.local/share/icons/hicolor/256x256/apps/ardoise.png
install -Dm644 assets/ardoise.desktop ~/.local/share/applications/ardoise.desktop
```

## Utilisation

- **Jour · 7 j · 30 j · 1 an** : la période affichée, barres par heure, jour ou mois.
- **Détails** : tokens entrants et sortants par modèle.
- **Roue crantée** : agents affichés et taille du widget.
- La fenêtre se déplace en glissant n'importe où sur le fond.

Pour une barre de budget sur 30 jours glissants, tous agents confondus :

```bash
ARDOISE_BUDGET=400 ardoise
```

Les réglages sont gardés dans `~/.config/ardoise/settings.json`.

## Limites

Les montants Claude Code et Codex sont des estimations au prix public de l'API. Ils ne
reflètent pas un forfait, une remise entreprise ni l'usage depuis une autre machine ou le
navigateur. Un modèle absent de la grille garde ses tokens mais compte pour 0 $.

## Développement

```bash
cargo test                    # unitaires, intégration, propriétés, interface (egui_kittest)
cargo clippy --all-targets
cargo fmt
RUST_LOG=ardoise=debug cargo run   # durée et volume de chaque chargement
```

L'architecture et les pièges connus sont décrits dans [AGENTS.md](AGENTS.md).

## Licence

Au choix, [MIT](LICENSE-MIT) ou [Apache 2.0](LICENSE-APACHE). La police Inter embarquée est sous
licence [SIL OFL 1.1](assets/fonts/OFL.txt).
