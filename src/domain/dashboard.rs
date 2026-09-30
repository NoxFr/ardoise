use super::period::Period;
use super::subscription::Subscription;
use super::usage::{Bucket, Entry, Summary, histogram, histogram_by, summarize};
use std::cmp::Reverse;

/// Ce qu'un fournisseur a renvoyé au dernier chargement.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub entries: Vec<Entry>,
    pub subscription: Option<Subscription>,
}

pub struct Section {
    /// Indice du fournisseur dans les `snapshots` passés à `Dashboard::build`.
    pub provider: usize,
    pub summary: Summary,
    /// Part dans le total facturé, entre 0 et 1 ; 0 pour un abonnement.
    pub share: f64,
    /// En $, ou en tokens pour un abonnement.
    pub buckets: Vec<Bucket>,
}

/// Tout ce que l'écran affiche pour une période, calculé une fois par changement de données.
pub struct Dashboard {
    /// Triées par coût décroissant.
    pub sections: Vec<Section>,
    /// Dépense des fournisseurs facturés à l'appel : un abonnement n'entre ni dans le total, ni
    /// dans la barre empilée, ni dans le budget.
    pub total: f64,
    pub tokens: u64,
    /// Dépense facturée à l'appel sur 30 jours, pour le budget.
    pub month_spent: f64,
}

impl Dashboard {
    /// `shown` : indices des fournisseurs affichés.
    pub fn build(snapshots: &[Snapshot], shown: &[usize], period: Period) -> Self {
        let starts = period.bucket_starts();
        let mut sections: Vec<Section> = shown
            .iter()
            .map(|&i| {
                let s = &snapshots[i];
                let mut summary = summarize(&s.entries, starts[0]);
                let buckets = if s.subscription.is_some() {
                    summary.models.sort_by_key(|m| Reverse(m.tokens()));
                    histogram_by(&s.entries, &starts, |e| e.tokens() as f64)
                } else {
                    histogram(&s.entries, &starts)
                };
                Section { provider: i, summary, share: 0.0, buckets }
            })
            .collect();
        sections.sort_by(|a, b| b.summary.cost.total_cmp(&a.summary.cost));

        let metered = |s: &Section| snapshots[s.provider].subscription.is_none();
        let total: f64 = sections.iter().filter(|s| metered(s)).map(|s| s.summary.cost).sum();
        if total > 0.0 {
            for s in sections.iter_mut().filter(|s| metered(s)) {
                s.share = s.summary.cost / total;
            }
        }
        let month = Period::Month.start();
        let month_spent = sections
            .iter()
            .filter(|s| metered(s))
            .flat_map(|s| &snapshots[s.provider].entries)
            .filter(|e| e.time >= month)
            .map(|e| e.cost)
            .sum();
        let tokens = sections.iter().map(|s| s.summary.tokens).sum();
        Dashboard { sections, total, tokens, month_spent }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn entry(model: &str, ago: Duration, cost: f64) -> Entry {
        Entry { time: Utc::now() - ago, model: model.into(), input: 1_000, output: 100, cost }
    }

    fn snapshot(entries: Vec<Entry>, subscription: Option<Subscription>) -> Snapshot {
        Snapshot { entries, subscription }
    }

    fn plan() -> Option<Subscription> {
        Some(Subscription { plan: "pro".into(), primary: None, secondary: None })
    }

    #[test]
    fn sorts_sections_by_cost_and_shares_the_total() {
        let snapshots = [
            snapshot(vec![entry("a", Duration::hours(1), 1.0)], None),
            snapshot(vec![entry("b", Duration::hours(1), 3.0)], None),
        ];
        let d = Dashboard::build(&snapshots, &[0, 1], Period::Week);
        assert_eq!(d.sections.iter().map(|s| s.provider).collect::<Vec<_>>(), [1, 0]);
        assert_eq!(d.total, 4.0);
        assert_eq!(d.sections[0].share, 0.75);
    }

    #[test]
    fn hidden_providers_are_left_out() {
        let snapshots = [snapshot(vec![entry("a", Duration::hours(1), 1.0)], None), Snapshot::default()];
        let d = Dashboard::build(&snapshots, &[1], Period::Week);
        assert_eq!(d.sections.len(), 1);
        assert_eq!(d.total, 0.0);
    }

    #[test]
    fn subscription_is_counted_in_tokens_and_kept_out_of_spend() {
        let snapshots = [
            snapshot(vec![entry("a", Duration::hours(1), 2.0)], None),
            snapshot(vec![entry("x", Duration::hours(1), 1.0), entry("y", Duration::hours(2), 9.0)], plan()),
        ];
        let d = Dashboard::build(&snapshots, &[0, 1], Period::Week);
        assert_eq!((d.total, d.month_spent, d.tokens), (2.0, 2.0, 3_300));
        let sub = d.sections.iter().find(|s| s.provider == 1).unwrap();
        assert_eq!(sub.share, 0.0);
        assert_eq!(sub.buckets.iter().map(Bucket::total).sum::<f64>(), 2_200.0, "barres en tokens");
    }

    #[test]
    fn month_spent_ignores_the_selected_period() {
        let snapshots =
            [snapshot(vec![entry("a", Duration::minutes(1), 1.0), entry("a", Duration::days(20), 10.0)], None)];
        let d = Dashboard::build(&snapshots, &[0], Period::Today);
        assert_eq!((d.total, d.month_spent), (1.0, 11.0));
    }
}
