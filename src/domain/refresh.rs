use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Intervalle d'actualisation automatique ; `Off` désactive le rafraîchissement périodique.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum AutoRefresh {
    Off,
    S5,
    S10,
    S30,
    #[default]
    M1,
    M5,
}

impl AutoRefresh {
    pub const ALL: [AutoRefresh; 6] =
        [AutoRefresh::Off, AutoRefresh::S5, AutoRefresh::S10, AutoRefresh::S30, AutoRefresh::M1, AutoRefresh::M5];

    pub fn label(self) -> &'static str {
        match self {
            AutoRefresh::Off => "Off",
            AutoRefresh::S5 => "5 s",
            AutoRefresh::S10 => "10 s",
            AutoRefresh::S30 => "30 s",
            AutoRefresh::M1 => "1 min",
            AutoRefresh::M5 => "5 min",
        }
    }

    /// `None` : pas de rafraîchissement automatique.
    pub fn duration(self) -> Option<Duration> {
        match self {
            AutoRefresh::Off => None,
            AutoRefresh::S5 => Some(Duration::from_secs(5)),
            AutoRefresh::S10 => Some(Duration::from_secs(10)),
            AutoRefresh::S30 => Some(Duration::from_secs(30)),
            AutoRefresh::M1 => Some(Duration::from_secs(60)),
            AutoRefresh::M5 => Some(Duration::from_secs(5 * 60)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn off_disables_periodic_refresh() {
        assert_eq!(AutoRefresh::Off.duration(), None);
    }

    #[rstest]
    #[case(AutoRefresh::S5, 5)]
    #[case(AutoRefresh::S10, 10)]
    #[case(AutoRefresh::S30, 30)]
    #[case(AutoRefresh::M1, 60)]
    #[case(AutoRefresh::M5, 300)]
    fn duration_matches_label(#[case] value: AutoRefresh, #[case] secs: u64) {
        assert_eq!(value.duration(), Some(Duration::from_secs(secs)));
    }
}
