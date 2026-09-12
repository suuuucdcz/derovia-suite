//! Les hypotheses d'entree d'un arbitrage.
//!
//! Tous les types sont serialisables : l'interface envoie exactement cette
//! structure, sans couche de traduction intermediaire.

use derovia_core::{CoreError, Rate, Result};
use serde::{Deserialize, Serialize};

/// Un arbitrage complet : l'horizon, le marche, et les deux options comparees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Arbitrage {
    /// Sur combien d'annees on projette. C'est le parametre le plus decisif :
    /// acheter gagne presque toujours a long terme et perd presque toujours a
    /// court terme, parce que les frais d'acquisition sont payes une fois.
    pub horizon_years: u32,
    /// Les hypotheses de marche, communes aux deux options.
    pub market: Market,
    /// L'option « acheter ».
    pub buy: Buy,
    /// L'option « louer ».
    pub rent: Rent,
    /// La fiscalite, quand le bien sert a une activite professionnelle.
    #[serde(default)]
    pub tax: Option<Tax>,
}

/// Les hypotheses de marche, appliquees aux deux options.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    /// Le rendement annuel net de ce que rapporte l'argent qu'on ne depense pas.
    ///
    /// C'est le cout d'opportunite : sans lui, la location parait toujours
    /// perdante alors qu'elle libere un apport qui peut travailler ailleurs.
    pub investment_return: Rate,
    /// L'inflation generale, appliquee aux charges recurrentes.
    pub inflation: Rate,
}

/// L'option « acheter » : le prix, le financement, les charges, la revente.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Buy {
    /// Le prix d'achat affiche, taxes comprises.
    pub price: f64,
    /// Les frais d'acquisition, en pourcentage du prix.
    ///
    /// Frais de notaire dans l'immobilier ancien, carte grise pour un vehicule,
    /// installation et mise en service pour une machine.
    pub acquisition_fees: Rate,
    /// L'apport verse comptant. Le reste est emprunte.
    pub down_payment: f64,
    /// Le taux nominal annuel du credit.
    pub loan_rate: Rate,
    /// La duree du credit, en annees. Elle peut differer de l'horizon.
    pub loan_years: u32,
    /// L'assurance emprunteur, en pourcentage annuel du capital emprunte.
    pub loan_insurance: Rate,
    /// L'entretien annuel, en pourcentage du prix d'achat.
    pub maintenance: Rate,
    /// Les charges fixes annuelles : taxe fonciere, assurance, copropriete.
    pub yearly_charges: f64,
    /// L'evolution annuelle de la valeur du bien. Negatif pour une decote.
    pub value_change: Rate,
    /// La perte de valeur immediate, subie des l'achat.
    ///
    /// Nulle dans l'immobilier, massive pour un vehicule neuf.
    pub first_year_drop: Rate,
    /// Les frais de revente, en pourcentage du prix de vente.
    pub resale_fees: Rate,
}

/// L'option « louer » : le loyer, son indexation, les frais d'entree et de sortie.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rent {
    /// Le loyer mensuel de depart.
    pub monthly_rent: f64,
    /// L'indexation annuelle du loyer.
    pub rent_indexation: Rate,
    /// Les charges mensuelles non incluses dans le loyer.
    pub monthly_charges: f64,
    /// Les frais d'entree non recuperables : agence, dossier, premier loyer majore.
    pub entry_fees: f64,
    /// Le depot de garantie, immobilise puis recupere a la sortie.
    pub deposit: f64,
    /// L'option d'achat de fin de contrat, en location avec option d'achat.
    ///
    /// Quand elle est renseignee, le moteur considere qu'elle est levee a
    /// l'horizon : le locataire paie ce montant et recupere le bien.
    #[serde(default)]
    pub buyout_option: Option<f64>,
}

/// La fiscalite d'un bien professionnel.
///
/// Absente pour un particulier : un logement de residence principale ne donne
/// droit ni a recuperation de TVA ni a deduction de charges.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tax {
    /// Vrai si la TVA est recuperable sur l'achat comme sur les loyers.
    ///
    /// Les montants saisis sont alors compris comme des montants toutes taxes
    /// comprises, et le moteur raisonne sur leur equivalent hors taxes.
    pub vat_recoverable: bool,
    /// Le taux de TVA applicable.
    pub vat_rate: Rate,
    /// Le taux d'imposition du benefice, applique aux charges deductibles.
    pub profit_tax: Rate,
    /// La duree d'amortissement comptable du bien, en annees. Zero si non amortissable.
    pub depreciation_years: u32,
}

impl Arbitrage {
    /// Verifie que les hypotheses sont exploitables.
    ///
    /// Le moteur reste numeriquement stable sur des valeurs absurdes, mais un
    /// resultat calcule sur un horizon nul ou un prix negatif n'aurait aucun
    /// sens : mieux vaut le dire a l'utilisateur que lui afficher un nombre.
    pub fn validate(&self) -> Result<()> {
        if self.horizon_years == 0 {
            return Err(CoreError::invalid("Horizon", "il faut projeter sur au moins un an"));
        }
        if self.horizon_years > 60 {
            return Err(CoreError::invalid(
                "Horizon",
                "au-delà de 60 ans la projection n'a plus de sens",
            ));
        }
        if self.buy.price <= 0.0 {
            return Err(CoreError::invalid("Prix d'achat", "doit être strictement positif"));
        }
        if self.buy.down_payment < 0.0 {
            return Err(CoreError::invalid("Apport", "ne peut pas être négatif"));
        }
        if self.rent.monthly_rent < 0.0 {
            return Err(CoreError::invalid("Loyer", "ne peut pas être négatif"));
        }
        if self.buy.loan_years == 0 && self.buy.down_payment < self.total_acquisition_cost() {
            return Err(CoreError::invalid(
                "Durée du crédit",
                "sans crédit, l'apport doit couvrir le prix et les frais",
            ));
        }
        if self.buy.loan_years > 40 {
            return Err(CoreError::invalid("Durée du crédit", "40 ans au maximum"));
        }
        if let Some(tax) = &self.tax
            && tax.vat_recoverable
            && tax.vat_rate.as_decimal() <= -1.0
        {
            return Err(CoreError::invalid("Taux de TVA", "valeur impossible"));
        }
        Ok(())
    }

    /// Le cout total d'acquisition, frais compris.
    #[must_use]
    pub fn total_acquisition_cost(&self) -> f64 {
        self.buy.price * (1.0 + self.buy.acquisition_fees.as_decimal())
    }

    /// Le capital effectivement emprunte, une fois l'apport deduit.
    #[must_use]
    pub fn loan_principal(&self) -> f64 {
        (self.total_acquisition_cost() - self.buy.down_payment).max(0.0)
    }
}
