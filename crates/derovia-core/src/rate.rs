//! Taux annuels et leurs conversions mensuelles.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Un taux annuel, stocke sous forme decimale (`0.035` pour 3,5 %).
///
/// L'interface manipule des pourcentages, le moteur des decimales. Ce type est
/// la frontiere entre les deux et supprime toute une classe d'erreurs de
/// facteur 100.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rate(f64);

impl Rate {
    /// Un taux nul.
    pub const ZERO: Self = Self(0.0);

    /// Construit depuis un pourcentage : `Rate::from_percent(3.5)` vaut 3,5 %/an.
    #[must_use]
    pub const fn from_percent(percent: f64) -> Self {
        Self(percent / 100.0)
    }

    /// Construit depuis une decimale : `Rate::from_decimal(0.035)` vaut 3,5 %/an.
    #[must_use]
    pub const fn from_decimal(decimal: f64) -> Self {
        Self(decimal)
    }

    /// La valeur decimale, `0.035`.
    #[must_use]
    pub const fn as_decimal(self) -> f64 {
        self.0
    }

    /// La valeur en pourcentage, `3.5`.
    #[must_use]
    pub fn as_percent(self) -> f64 {
        self.0 * 100.0
    }

    /// Le taux mensuel proportionnel, soit le taux annuel divise par douze.
    ///
    /// C'est la convention bancaire francaise pour les prets amortissables :
    /// un pret affiche a 3,5 % applique 3,5/12 % par mois.
    #[must_use]
    pub fn monthly_nominal(self) -> f64 {
        self.0 / 12.0
    }

    /// Le taux mensuel equivalent actuariel, `(1 + taux)^(1/12) - 1`.
    ///
    /// A utiliser pour capitaliser un placement ou faire evoluer un prix :
    /// douze mois composes redonnent exactement le taux annuel affiche.
    #[must_use]
    pub fn monthly_effective(self) -> f64 {
        (1.0 + self.0).powf(1.0 / 12.0) - 1.0
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} %", self.as_percent())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_and_decimal_are_two_views_of_the_same_value() {
        let rate = Rate::from_percent(3.5);
        assert!((rate.as_decimal() - 0.035).abs() < 1e-12);
        assert!((rate.as_percent() - 3.5).abs() < 1e-12);
    }

    #[test]
    fn twelve_effective_months_reconstitute_the_annual_rate() {
        let rate = Rate::from_percent(6.0);
        let compounded = (1.0 + rate.monthly_effective()).powi(12) - 1.0;
        assert!((compounded - 0.06).abs() < 1e-12);
    }

    #[test]
    fn nominal_monthly_rate_follows_the_banking_convention() {
        let rate = Rate::from_percent(3.6);
        assert!((rate.monthly_nominal() - 0.003).abs() < 1e-12);
    }
}
