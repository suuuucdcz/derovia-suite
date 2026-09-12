//! Ce que le moteur renvoie : une decision, deux bilans, et une trajectoire.

use serde::{Deserialize, Serialize};

/// La decision, une fois les deux patrimoines compares a l'horizon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Recommendation {
    /// Acheter laisse un patrimoine nettement superieur.
    Buy,
    /// Louer laisse un patrimoine nettement superieur.
    Rent,
    /// L'ecart est trop faible pour trancher sur des hypotheses incertaines.
    TooClose,
}

impl Recommendation {
    /// La decision, formulee comme on la dirait a l'utilisateur.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Buy => "Acheter",
            Self::Rent => "Louer",
            Self::TooClose => "Trop serré",
        }
    }
}

/// Le bilan complet d'une des deux options a l'horizon.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// Tout ce qui est sorti de la poche sur la periode, frais d'entree compris.
    pub total_outflow: f64,
    /// Le patrimoine net a l'horizon : capital place, plus le bien, moins la dette.
    pub net_worth: f64,
    /// La valeur du bien a l'horizon, nette des frais de revente.
    pub asset_value: f64,
    /// Le capital restant du a l'horizon.
    pub remaining_debt: f64,
    /// Le capital place, alimente chaque mois par l'ecart de cout entre les deux options.
    pub portfolio: f64,
    /// L'effort mensuel moyen de la premiere annee, ce que l'utilisateur ressent.
    pub monthly_effort_first_year: f64,
    /// Les interets d'emprunt payes sur la periode.
    pub interest_paid: f64,
    /// L'economie d'impot cumulee, quand le bien est professionnel.
    pub tax_saved: f64,
}

/// Un point annuel de la trajectoire, pour tracer les deux courbes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YearPoint {
    /// L'annee, comptee depuis l'operation.
    pub year: u32,
    /// Le patrimoine net en cas d'achat.
    pub buy_net_worth: f64,
    /// Le patrimoine net en cas de location.
    pub rent_net_worth: f64,
    /// Le cumul de ce qui est sorti de la poche en cas d'achat.
    pub buy_cumulative_cost: f64,
    /// Le cumul de ce qui est sorti de la poche en cas de location.
    pub rent_cumulative_cost: f64,
    /// La valeur du bien cette annee-la, nette des frais de revente.
    pub asset_value: f64,
    /// Le capital restant du cette annee-la.
    pub remaining_debt: f64,
}

/// Le resultat complet d'un arbitrage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    /// La decision.
    pub recommendation: Recommendation,
    /// L'horizon sur lequel elle a ete rendue.
    pub horizon_years: u32,
    /// L'annee a partir de laquelle acheter repasse devant, si elle existe.
    ///
    /// C'est le chiffre que retiennent les gens : « il faut rester au moins
    /// sept ans ». `None` signifie que sur l'horizon demande, acheter ne
    /// rattrape jamais son retard.
    pub break_even_year: Option<u32>,
    /// Le bilan de l'achat.
    pub buy: Outcome,
    /// Le bilan de la location.
    pub rent: Outcome,
    /// L'ecart de patrimoine net a l'horizon. Positif quand acheter gagne.
    pub net_advantage: f64,
    /// La trajectoire annuelle des deux options.
    pub timeline: Vec<YearPoint>,
}

impl Verdict {
    /// L'ecart rapporte au patrimoine le plus eleve des deux.
    ///
    /// Sert a decider si l'ecart merite une recommandation ferme : gagner
    /// 3 000 € sur un patrimoine de 400 000 € tient dans le bruit des hypotheses.
    #[must_use]
    pub fn relative_advantage(&self) -> f64 {
        let reference = self.buy.net_worth.abs().max(self.rent.net_worth.abs());
        if reference < 1.0 { 0.0 } else { self.net_advantage / reference }
    }
}
