use ardoise::domain::usage::{histogram, summarize, Entry};
use ardoise::ui::format;
use chrono::{DateTime, Duration, TimeZone, Utc};
use proptest::prelude::*;

const MODELS: [&str; 4] = ["Opus 5.5", "Opus 5", "Sonnet 5", "qwen3.6"];

fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap()
}

fn arb_entry() -> impl Strategy<Value = Entry> {
    (0..MODELS.len(), 0i64..30 * 86_400, 0.0..50.0f64, 0u64..1_000_000, 0u64..100_000).prop_map(
        |(m, secs, cost, input, output)| Entry {
            time: start() + Duration::seconds(secs),
            model: MODELS[m].into(),
            input,
            output,
            cost,
        },
    )
}

proptest! {
    #[test]
    fn histogram_total_matches_summary(entries in prop::collection::vec(arb_entry(), 0..200)) {
        let summary = summarize(&entries, start());
        let starts: Vec<_> = (0..30).map(|d| start() + Duration::days(d)).collect();
        let buckets = histogram(&entries, &starts);
        let hist_total: f64 = buckets.iter().map(|b| b.total()).sum();
        prop_assert!((hist_total - summary.cost).abs() < 1e-6);
    }

    #[test]
    fn summary_is_sorted_by_cost(entries in prop::collection::vec(arb_entry(), 0..200)) {
        let s = summarize(&entries, start());
        prop_assert!(s.models.windows(2).all(|w| w[0].cost >= w[1].cost));
        prop_assert!((s.models.iter().map(|m| m.cost).sum::<f64>() - s.cost).abs() < 1e-6);
    }

    #[test]
    fn money_always_has_two_decimals(v in 0.0..1e7f64) {
        let s = format::money(v);
        let (_, decimals) = s.split_once(',').unwrap();
        prop_assert!(s.starts_with('$'));
        prop_assert_eq!(decimals.len(), 2);
    }
}
