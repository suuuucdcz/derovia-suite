//! Trois jeux d'hypotheses par defaut.
//!
//! Le moteur est generique ; ce qui distingue un logement d'une voiture, c'est
//! seulement la maniere dont leur valeur evolue et la fiscalite qui s'y
//! applique. Ces presets encodent ces differences et donnent a l'utilisateur un
//! point de depart credible plutot qu'un formulaire vide.
//!
//! Les chiffres sont des ordres de grandeur du marche francais, destines a etre
//! ajustes. Ils ne constituent pas un conseil en investissement.

use derovia_core::Rate;
use serde::Serialize;

use crate::model::{Arbitrage, Buy, Market, Rent, Tax};

/// Les trois situations couvertes par l'outil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    /// Un logement de residence principale.
    RealEstate,
    /// Un vehicule particulier.
    Vehicle,
    /// Du materiel professionnel.
    ProfessionalEquipment,
}

impl Preset {
    /// Les trois presets, dans l'ordre d'affichage.
    pub const ALL: [Self; 3] = [Self::RealEstate, Self::Vehicle, Self::ProfessionalEquipment];

    /// L'identifiant stable, utilise par l'interface et la sauvegarde.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::RealEstate => "immobilier",
            Self::Vehicle => "vehicule",
            Self::ProfessionalEquipment => "materiel",
        }
    }

    /// Retrouve un preset depuis son identifiant.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.id() == id)
    }

    /// Le titre affiche sur la carte de selection.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::RealEstate => "Logement",
            Self::Vehicle => "Véhicule",
            Self::ProfessionalEquipment => "Matériel pro",
        }
    }

    /// La phrase qui situe le cas, sous le titre.
    #[must_use]
    pub fn subtitle(self) -> &'static str {
        match self {
            Self::RealEstate => "Acheter sa résidence principale ou rester locataire",
            Self::Vehicle => "Acheter à crédit ou passer en location longue durée",
            Self::ProfessionalEquipment => "Investir dans l'outil ou le prendre en leasing",
        }
    }

    /// Le nom de l'option d'achat, tel qu'on le dit dans ce contexte.
    #[must_use]
    pub fn buy_label(self) -> &'static str {
        match self {
            Self::RealEstate | Self::Vehicle => "Acheter",
            Self::ProfessionalEquipment => "Investir",
        }
    }

    /// Le nom de l'option de location, tel qu'on le dit dans ce contexte.
    #[must_use]
    pub fn rent_label(self) -> &'static str {
        match self {
            Self::RealEstate => "Louer",
            Self::Vehicle => "LOA / LLD",
            Self::ProfessionalEquipment => "Leasing",
        }
    }

    /// Le jeu d'hypotheses complet.
    #[must_use]
    pub fn scenario(self) -> Arbitrage {
        match self {
            Self::RealEstate => real_estate(),
            Self::Vehicle => vehicle(),
            Self::ProfessionalEquipment => professional_equipment(),
        }
    }
}

/// Une carte de preset prete a etre affichee, hypotheses incluses.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCard {
    /// L'identifiant stable.
    pub id: &'static str,
    /// Le titre de la carte.
    pub title: &'static str,
    /// La phrase de situation.
    pub subtitle: &'static str,
    /// Le nom de l'option d'achat dans ce contexte.
    pub buy_label: &'static str,
    /// Le nom de l'option de location dans ce contexte.
    pub rent_label: &'static str,
    /// Les hypotheses de depart.
    pub scenario: Arbitrage,
}

/// Le catalogue complet, tel que l'interface l'affiche au demarrage.
#[must_use]
pub fn catalog() -> Vec<PresetCard> {
    Preset::ALL
        .into_iter()
        .map(|preset| PresetCard {
            id: preset.id(),
            title: preset.title(),
            subtitle: preset.subtitle(),
            buy_label: preset.buy_label(),
            rent_label: preset.rent_label(),
            scenario: preset.scenario(),
        })
        .collect()
}

/// Acheter sa residence principale ou rester locataire.
///
/// Le cas ou l'achat est le plus favorable a long terme : le bien prend de la
/// valeur, mais les frais de notaire imposent de rester plusieurs annees avant
/// de les avoir amortis.
#[must_use]
pub fn real_estate() -> Arbitrage {
    Arbitrage {
        horizon_years: 10,
        market: Market {
            investment_return: Rate::from_percent(5.0),
            inflation: Rate::from_percent(2.0),
        },
        buy: Buy {
            price: 300_000.0,
            // Frais de notaire dans l'ancien.
            acquisition_fees: Rate::from_percent(8.0),
            down_payment: 60_000.0,
            loan_rate: Rate::from_percent(3.5),
            loan_years: 20,
            loan_insurance: Rate::from_percent(0.34),
            // Provision d'entretien courant, regle du pouce du batiment.
            maintenance: Rate::from_percent(1.0),
            // Taxe fonciere, assurance habitation et charges de copropriete.
            yearly_charges: 2_400.0,
            // Tendance longue du marche francais, hors cycles. C'est le
            // parametre le plus decisif de tout l'arbitrage : un point de plus
            // ou de moins ici renverse la conclusion sur vingt ans.
            value_change: Rate::from_percent(2.5),
            first_year_drop: Rate::ZERO,
            // Commission d'agence a la revente.
            resale_fees: Rate::from_percent(5.0),
        },
        rent: Rent {
            monthly_rent: 1_100.0,
            rent_indexation: Rate::from_percent(2.0),
            monthly_charges: 80.0,
            entry_fees: 1_100.0,
            deposit: 1_100.0,
            buyout_option: None,
        },
        tax: None,
    }
}

/// Acheter un vehicule a credit ou passer en location longue duree.
///
/// Le cas inverse du logement : la decote est immediate et brutale, et l'ecart
/// se joue davantage sur l'entretien et l'horizon de detention que sur le prix.
#[must_use]
pub fn vehicle() -> Arbitrage {
    Arbitrage {
        horizon_years: 5,
        market: Market {
            investment_return: Rate::from_percent(4.0),
            inflation: Rate::from_percent(2.0),
        },
        buy: Buy {
            price: 35_000.0,
            // Carte grise et mise a la route.
            acquisition_fees: Rate::from_percent(1.0),
            down_payment: 5_000.0,
            loan_rate: Rate::from_percent(5.5),
            loan_years: 5,
            loan_insurance: Rate::from_percent(0.2),
            // Revisions, pneumatiques, pieces d'usure.
            maintenance: Rate::from_percent(2.5),
            // Assurance tous risques.
            yearly_charges: 900.0,
            value_change: Rate::from_percent(-12.0),
            // La decote de sortie de concession.
            first_year_drop: Rate::from_percent(20.0),
            resale_fees: Rate::from_percent(2.0),
        },
        rent: Rent {
            monthly_rent: 450.0,
            rent_indexation: Rate::ZERO,
            // Assurance restant a la charge du locataire ; l'entretien est inclus.
            monthly_charges: 75.0,
            // Premier loyer majore.
            entry_fees: 3_000.0,
            deposit: 0.0,
            buyout_option: None,
        },
        tax: None,
    }
}

/// Investir dans du materiel professionnel ou le prendre en leasing.
///
/// Le seul des trois cas ou la fiscalite pese autant que le prix : la TVA est
/// recuperable des deux cotes, mais l'achat ouvre un amortissement et le
/// leasing rend le loyer integralement deductible.
#[must_use]
pub fn professional_equipment() -> Arbitrage {
    Arbitrage {
        horizon_years: 5,
        market: Market {
            // Le capital d'une entreprise a un cout d'opportunite plus eleve :
            // il pourrait financer son propre developpement.
            investment_return: Rate::from_percent(6.0),
            inflation: Rate::from_percent(2.0),
        },
        buy: Buy {
            price: 60_000.0,
            // Livraison, installation, mise en service.
            acquisition_fees: Rate::from_percent(2.0),
            down_payment: 12_000.0,
            loan_rate: Rate::from_percent(4.5),
            loan_years: 5,
            loan_insurance: Rate::from_percent(0.1),
            // Contrat de maintenance et consommables.
            maintenance: Rate::from_percent(4.0),
            yearly_charges: 600.0,
            value_change: Rate::from_percent(-20.0),
            first_year_drop: Rate::ZERO,
            resale_fees: Rate::from_percent(5.0),
        },
        rent: Rent {
            monthly_rent: 1_100.0,
            rent_indexation: Rate::ZERO,
            // Maintenance incluse au contrat de leasing.
            monthly_charges: 0.0,
            entry_fees: 2_000.0,
            deposit: 0.0,
            buyout_option: None,
        },
        tax: Some(Tax {
            vat_recoverable: true,
            vat_rate: Rate::from_percent(20.0),
            profit_tax: Rate::from_percent(25.0),
            depreciation_years: 5,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_valid() {
        for preset in Preset::ALL {
            let scenario = preset.scenario();
            assert!(scenario.validate().is_ok(), "preset invalide : {}", preset.id());
        }
    }

    #[test]
    fn identifiers_round_trip() {
        for preset in Preset::ALL {
            assert_eq!(Preset::from_id(preset.id()), Some(preset));
        }
        assert_eq!(Preset::from_id("bateau"), None);
    }

    #[test]
    fn the_catalog_exposes_the_three_cases() {
        let catalog = catalog();
        assert_eq!(catalog.len(), 3);
        assert_eq!(catalog.first().map(|card| card.id), Some("immobilier"));
    }

    #[test]
    fn only_the_professional_case_carries_a_tax_profile() {
        assert!(real_estate().tax.is_none());
        assert!(vehicle().tax.is_none());
        assert!(professional_equipment().tax.is_some());
    }
}
