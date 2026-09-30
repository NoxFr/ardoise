#[derive(Default)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_write_5m: u64,
    pub cache_write_1h: u64,
    pub cache_read: u64,
}

/// Prix en $ par million de tokens.
#[derive(Clone, Copy)]
pub struct Price {
    input: f64,
    output: f64,
    cache_read: f64,
}

const fn p(input: f64, output: f64, cache_read: f64) -> Price {
    Price { input, output, cache_read }
}

const TABLE: &[(&str, Price)] = &[
    ("claude-fable-5-1", p(10.0, 50.0, 0.25)),
    ("claude-mythos-5-1", p(10.0, 50.0, 0.25)),
    ("claude-fable", p(10.0, 50.0, 1.0)),
    ("claude-mythos", p(10.0, 50.0, 1.0)),
    ("claude-opus-5-5", p(4.0, 20.0, 0.20)),
    ("claude-opus-5", p(5.0, 25.0, 0.50)),
    ("claude-opus-4-5", p(5.0, 25.0, 0.50)),
    ("claude-opus-4-6", p(5.0, 25.0, 0.50)),
    ("claude-opus-4-7", p(5.0, 25.0, 0.50)),
    ("claude-opus-4-8", p(5.0, 25.0, 0.50)),
    ("claude-opus-4", p(15.0, 75.0, 1.50)),
    ("claude-sonnet-5", p(2.0, 10.0, 0.20)),
    ("claude-sonnet-4", p(3.0, 15.0, 0.30)),
    ("claude-3-7-sonnet", p(3.0, 15.0, 0.30)),
    ("claude-haiku-4-5", p(1.0, 5.0, 0.10)),
    ("claude-3-5-haiku", p(0.8, 4.0, 0.08)),
];

pub fn price(model: &str) -> Option<Price> {
    TABLE.iter().find(|(prefix, _)| model.starts_with(prefix)).map(|(_, pr)| *pr)
}

impl Price {
    pub fn cost(&self, u: &TokenUsage) -> f64 {
        (u.input as f64 * self.input
            + u.output as f64 * self.output
            + u.cache_write_5m as f64 * self.input * 1.25
            + u.cache_write_1h as f64 * self.input * 2.0
            + u.cache_read as f64 * self.cache_read)
            / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("claude-opus-5-5", 4.0, 20.0)]
    #[case("claude-opus-5", 5.0, 25.0)]
    #[case("claude-opus-4-8", 5.0, 25.0)]
    #[case("claude-opus-4-1-20250805", 15.0, 75.0)]
    #[case("claude-sonnet-5", 2.0, 10.0)]
    #[case("claude-sonnet-5-5", 2.0, 10.0)]
    #[case("claude-fable-5-1", 10.0, 50.0)]
    #[case("claude-sonnet-4-5-20250929", 3.0, 15.0)]
    #[case("claude-haiku-4-5-20251001", 1.0, 5.0)]
    fn most_specific_price_wins(#[case] model: &str, #[case] input: f64, #[case] output: f64) {
        let p = price(model).unwrap();
        assert_eq!((p.input, p.output), (input, output));
    }

    #[rstest]
    #[case("<synthetic>")]
    #[case("gpt-x")]
    fn unknown_models_have_no_price(#[case] model: &str) {
        assert!(price(model).is_none());
    }

    #[test]
    fn cost_applies_cache_multipliers() {
        let u = TokenUsage {
            input: 1_000_000,
            output: 1_000_000,
            cache_write_5m: 1_000_000,
            cache_write_1h: 1_000_000,
            cache_read: 1_000_000,
        };
        // 2 + 10 + 2.5 + 4 + 0.2
        assert!((price("claude-sonnet-5").unwrap().cost(&u) - 18.7).abs() < 1e-9);
    }
}
