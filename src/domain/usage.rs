use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Un appel facturé, quel que soit le fournisseur.
#[derive(Clone, Debug)]
pub struct Entry {
    pub time: DateTime<Utc>,
    /// Nom affiché du modèle, ex. "Opus 5.5".
    pub model: String,
    pub input: u64,
    pub output: u64,
    pub cost: f64,
}

pub struct ModelStat {
    pub name: String,
    pub cost: f64,
    pub input: u64,
    pub output: u64,
}

/// `models` est trié du plus cher au moins cher.
pub struct Summary {
    pub cost: f64,
    pub tokens: u64,
    pub models: Vec<ModelStat>,
}

pub fn summarize(entries: &[Entry], since: DateTime<Utc>) -> Summary {
    let mut by: HashMap<&str, ModelStat> = HashMap::new();
    for e in entries.iter().filter(|e| e.time >= since) {
        let s =
            by.entry(&e.model).or_insert_with(|| ModelStat { name: e.model.clone(), cost: 0.0, input: 0, output: 0 });
        s.cost += e.cost;
        s.input += e.input;
        s.output += e.output;
    }
    let mut models: Vec<_> = by.into_values().collect();
    models.sort_by(|a, b| b.cost.total_cmp(&a.cost).then_with(|| a.name.cmp(&b.name)));
    Summary {
        cost: models.iter().map(|m| m.cost).sum(),
        tokens: models.iter().map(|m| m.input + m.output).sum(),
        models,
    }
}

pub struct Bucket {
    pub start: DateTime<Utc>,
    pub by_model: HashMap<String, f64>,
}

impl Bucket {
    pub fn total(&self) -> f64 {
        self.by_model.values().sum()
    }
}

/// Répartit les coûts dans des tranches commençant à `starts` (croissants) ; la dernière est ouverte.
pub fn histogram(entries: &[Entry], starts: &[DateTime<Utc>]) -> Vec<Bucket> {
    let mut buckets: Vec<Bucket> = starts.iter().map(|s| Bucket { start: *s, by_model: HashMap::new() }).collect();
    for e in entries {
        let Some(i) = starts.partition_point(|s| *s <= e.time).checked_sub(1) else { continue };
        *buckets[i].by_model.entry(e.model.clone()).or_default() += e.cost;
    }
    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn mk(time: &str, model: &str, cost: f64) -> Entry {
        Entry { time: t(time), model: model.into(), input: 10, output: 1, cost }
    }

    #[test]
    fn summarize_filters_and_sorts_by_cost() {
        let entries = vec![
            mk("2026-09-01T00:00:00Z", "Opus 5", 100.0),
            mk("2026-09-29T00:00:00Z", "Opus 5", 1.0),
            mk("2026-09-29T00:00:00Z", "Opus 5.5", 3.0),
            mk("2026-09-29T00:00:00Z", "Opus 5.5", 1.5),
            mk("2026-09-29T00:00:00Z", "Sonnet 5", 0.5),
        ];
        let s = summarize(&entries, t("2026-09-28T00:00:00Z"));
        assert_eq!(s.cost, 6.0);
        assert_eq!(s.tokens, 44);
        let names: Vec<_> = s.models.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Opus 5.5", "Opus 5", "Sonnet 5"]);
    }

    #[test]
    fn histogram_fills_uneven_buckets() {
        let entries = vec![
            mk("2026-07-31T12:00:00Z", "Opus 5", 50.0),
            mk("2026-08-01T00:00:00Z", "Opus 5", 2.0),
            mk("2026-08-31T23:00:00Z", "Sonnet 5", 1.0),
            mk("2026-09-30T10:00:00Z", "Haiku 4.5", 4.0),
        ];
        let starts = [t("2026-08-01T00:00:00Z"), t("2026-09-01T00:00:00Z"), t("2026-10-01T00:00:00Z")];
        let h = histogram(&entries, &starts);
        assert_eq!(h.iter().map(Bucket::total).collect::<Vec<_>>(), [3.0, 4.0, 0.0], "juillet exclu");
        assert_eq!(h[0].by_model["Sonnet 5"], 1.0);
    }
}
