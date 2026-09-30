use crate::domain::period::Granularity;
use chrono::{DateTime, Datelike, Local, Utc};

pub fn money(v: f64) -> String {
    // Une somme f64 vide vaut -0.0.
    let v = if v == 0.0 { 0.0 } else { v };
    format!("${v:.2}").replace('.', ",")
}

pub fn tokens(n: u64) -> String {
    let n = n as f64;
    let (v, unit) = if n >= 1e9 {
        (n / 1e9, " Md")
    } else if n >= 1e6 {
        (n / 1e6, " M")
    } else if n >= 1e3 {
        return format!("{:.0} k", n / 1e3);
    } else {
        return format!("{n}");
    };
    format!("{v:.1}{unit}").replace('.', ",")
}

const MONTHS: [&str; 12] =
    ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];

/// Libellé d'axe : "14 h", "24/09" ou "oct. 25".
pub fn bucket_label(t: DateTime<Local>, g: Granularity) -> String {
    match g {
        Granularity::Hour => t.format("%H h").to_string(),
        Granularity::Day => t.format("%d/%m").to_string(),
        Granularity::Month => format!("{} {}", MONTHS[t.month0() as usize], t.format("%y")),
    }
}

/// Part entre 0 et 1, au format de la maquette : "85%".
pub fn share(ratio: f64) -> String {
    percent(ratio * 100.0)
}

/// Pourcentage déjà sur 0-100 (ex. un quota d'abonnement), au même format que `share`.
pub fn percent(p: f64) -> String {
    let pct = p.round();
    format!("{:.0}%", if pct == 0.0 { 0.0 } else { pct })
}

/// Fenêtre de quota en durée lisible : "5 h", "7 j".
pub fn window_label(minutes: i64) -> String {
    if minutes < 60 {
        format!("{minutes} min")
    } else if minutes < 24 * 60 {
        format!("{} h", minutes / 60)
    } else {
        format!("{} j", minutes / (24 * 60))
    }
}

/// Temps restant avant réinitialisation, depuis `now` : "12 min", "3 h", "2 j".
pub fn resets_in(resets_at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let minutes = (resets_at - now).num_minutes().max(0);
    window_label(minutes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(399.7, "$399,70")]
    #[case(-0.0, "$0,00")]
    fn formats_money_fr(#[case] v: f64, #[case] expected: &str) {
        assert_eq!(money(v), expected);
    }

    #[rstest]
    #[case(950, "950")]
    #[case(12_300, "12 k")]
    #[case(900_000, "900 k")]
    #[case(56_100_000, "56,1 M")]
    #[case(2_300_000_000, "2,3 Md")]
    fn formats_tokens(#[case] n: u64, #[case] expected: &str) {
        assert_eq!(tokens(n), expected);
    }

    #[rstest]
    #[case("2026-09-30T14:05:00", Granularity::Hour, "14 h")]
    #[case("2026-09-04T14:05:00", Granularity::Day, "04/09")]
    #[case("2025-10-01T00:00:00", Granularity::Month, "oct. 25")]
    #[case("2026-02-01T00:00:00", Granularity::Month, "févr. 26")]
    fn formats_bucket_labels(#[case] t: &str, #[case] g: Granularity, #[case] expected: &str) {
        let t = t.parse::<chrono::NaiveDateTime>().unwrap().and_local_timezone(Local).unwrap();
        assert_eq!(bucket_label(t, g), expected);
    }

    #[rstest]
    #[case(0.851, "85%")]
    #[case(0.0, "0%")]
    #[case(-0.0, "0%")]
    #[case(-0.001, "0%")]
    fn formats_share(#[case] ratio: f64, #[case] expected: &str) {
        assert_eq!(share(ratio), expected);
    }

    #[rstest]
    #[case(42.4, "42%")]
    #[case(0.0, "0%")]
    #[case(100.0, "100%")]
    fn formats_percent(#[case] p: f64, #[case] expected: &str) {
        assert_eq!(percent(p), expected);
    }

    #[rstest]
    #[case(30, "30 min")]
    #[case(300, "5 h")]
    #[case(10_080, "7 j")]
    fn formats_window_label(#[case] minutes: i64, #[case] expected: &str) {
        assert_eq!(window_label(minutes), expected);
    }

    #[test]
    fn formats_time_left_before_reset() {
        let now = t("2026-09-30T00:00:00Z");
        assert_eq!(resets_in(t("2026-09-30T03:00:00Z"), now), "3 h");
        assert_eq!(resets_in(t("2026-09-29T23:00:00Z"), now), "0 min", "déjà passé, borné à 0");
    }

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }
}
