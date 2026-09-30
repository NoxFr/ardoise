use chrono::{DateTime, Datelike, Duration, Local, Months, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Period {
    Today,
    #[default]
    Week,
    Month,
    Year,
}

/// Largeur d'une barre du graphique.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Granularity {
    Hour,
    Day,
    Month,
}

impl Period {
    pub const ALL: [Period; 4] = [Period::Today, Period::Week, Period::Month, Period::Year];

    pub fn label(self) -> &'static str {
        match self {
            Period::Today => "Jour",
            Period::Week => "7 j",
            Period::Month => "30 j",
            Period::Year => "1 an",
        }
    }

    pub fn granularity(self) -> Granularity {
        match self {
            Period::Today => Granularity::Hour,
            Period::Week | Period::Month => Granularity::Day,
            Period::Year => Granularity::Month,
        }
    }

    pub fn start(self) -> DateTime<Utc> {
        self.bucket_starts()[0]
    }

    /// Début de chaque barre, du plus ancien au plus récent : 24 heures, 7 ou 30 jours, 12 mois civils.
    pub fn bucket_starts(self) -> Vec<DateTime<Utc>> {
        let today = Local::now().date_naive();
        match self {
            Period::Today => (0..24).map(|h| midnight(today) + Duration::hours(h)).collect(),
            Period::Week => (0..7).rev().map(|d| midnight(today - Duration::days(d))).collect(),
            Period::Month => (0..30).rev().map(|d| midnight(today - Duration::days(d))).collect(),
            Period::Year => {
                let first = today.with_day(1).unwrap();
                (0..12).rev().map(|m| midnight(first - Months::new(m))).collect()
            }
        }
    }

    /// Début de la plus longue période : tout ce qu'il faut charger.
    pub fn earliest() -> DateTime<Utc> {
        Period::ALL.iter().map(|p| p.start()).min().unwrap()
    }
}

fn midnight(day: NaiveDate) -> DateTime<Utc> {
    Local.from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap()).earliest().unwrap().with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Period::Today, 24)]
    #[case(Period::Week, 7)]
    #[case(Period::Month, 30)]
    #[case(Period::Year, 12)]
    fn bucket_counts(#[case] p: Period, #[case] n: usize) {
        let starts = p.bucket_starts();
        assert_eq!(starts.len(), n);
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
        assert!(starts[0] <= Utc::now(), "la période commence dans le passé");
    }

    #[test]
    fn year_buckets_are_calendar_months() {
        let starts = Period::Year.bucket_starts();
        for s in &starts {
            assert_eq!(s.with_timezone(&Local).day(), 1);
        }
        let last = starts.last().unwrap().with_timezone(&Local);
        assert_eq!((last.year(), last.month()), (Local::now().year(), Local::now().month()));
    }

    #[test]
    fn earliest_is_the_year_start() {
        assert_eq!(Period::earliest(), Period::Year.start());
    }
}
