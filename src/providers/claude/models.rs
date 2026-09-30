/// "claude-opus-5-5" -> "Opus 5.5", "claude-haiku-4-5-20251001" -> "Haiku 4.5".
pub fn display_name(model: &str) -> String {
    let parts: Vec<&str> = model.trim_start_matches("claude-").split('-').collect();
    let is_version = |p: &&str| p.len() < 8 && p.chars().all(|c| c.is_ascii_digit());
    let version: Vec<&str> = parts.iter().filter(|p| is_version(p)).copied().collect();
    let words: Vec<String> = parts
        .iter()
        .filter(|p| !p.chars().all(|c| c.is_ascii_digit()))
        .map(|w| w[..1].to_uppercase() + &w[1..])
        .collect();
    format!("{} {}", words.join(" "), version.join(".")).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("claude-opus-5-5", "Opus 5.5")]
    #[case("claude-opus-5", "Opus 5")]
    #[case("claude-haiku-4-5-20251001", "Haiku 4.5")]
    #[case("claude-3-7-sonnet-20250219", "Sonnet 3.7")]
    #[case("claude-fable-5-1", "Fable 5.1")]
    fn display_names(#[case] id: &str, #[case] expected: &str) {
        assert_eq!(display_name(id), expected);
    }
}
