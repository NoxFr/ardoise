/// $ par million de tokens : (entrée, entrée en cache, sortie). Le préfixe le plus spécifique d'abord.
const TABLE: &[(&str, [f64; 3])] = &[
    ("gpt-5-nano", [0.05, 0.005, 0.40]),
    ("gpt-5-mini", [0.25, 0.025, 2.00]),
    ("gpt-5.1-codex-mini", [0.25, 0.025, 2.00]),
    ("gpt-5-codex-mini", [0.25, 0.025, 2.00]),
    ("gpt-5", [1.25, 0.125, 10.00]),
    ("o4-mini", [1.10, 0.275, 4.40]),
    ("codex-mini", [1.50, 0.375, 6.00]),
];

/// `input` inclut les tokens en cache (convention OpenAI). Modèle inconnu : coût nul.
pub fn cost(model: &str, input: u64, cached: u64, output: u64) -> f64 {
    let Some((_, [i, c, o])) = TABLE.iter().find(|(p, _)| model.starts_with(p)) else { return 0.0 };
    (input.saturating_sub(cached) as f64 * i + cached as f64 * c + output as f64 * o) / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("gpt-5-codex", 1_000_000, 0, 0, 1.25)]
    #[case("gpt-5.1-codex", 1_000_000, 1_000_000, 1_000_000, 10.125)]
    #[case("gpt-5-mini", 0, 0, 1_000_000, 2.0)]
    #[case("gpt-5.1-codex-mini", 1_000_000, 0, 0, 0.25)]
    #[case("inconnu", 1_000_000, 0, 1_000_000, 0.0)]
    fn prices(#[case] model: &str, #[case] i: u64, #[case] c: u64, #[case] o: u64, #[case] expected: f64) {
        assert!((cost(model, i, c, o) - expected).abs() < 1e-9);
    }
}
