//! Prets amortissables et capitalisation.
//!
//! Le moteur d'arbitrage simule mois par mois plutot que par formule fermee :
//! c'est le seul moyen de melanger un pret, des charges indexees, un placement
//! et une revente sans empiler les approximations.

use crate::rate::Rate;

/// La mensualite constante d'un pret amortissable.
///
/// `monthly_rate` est un taux **mensuel** (voir [`Rate::monthly_nominal`]).
/// Un taux nul degenere proprement en un simple remboursement lineaire.
#[must_use]
pub fn monthly_payment(principal: f64, monthly_rate: f64, months: u32) -> f64 {
    if months == 0 || principal <= 0.0 {
        return 0.0;
    }
    if monthly_rate.abs() < 1e-12 {
        return principal / f64::from(months);
    }
    principal * monthly_rate / (1.0 - (1.0 + monthly_rate).powf(-f64::from(months)))
}

/// Une echeance de pret, decomposee entre interets et capital.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Installment {
    /// La part d'interets, definitivement perdue.
    pub interest: f64,
    /// La part de capital, qui reconstitue du patrimoine.
    pub principal: f64,
}

impl Installment {
    /// Le montant total decaisse sur le mois.
    #[must_use]
    pub fn total(self) -> f64 {
        self.interest + self.principal
    }
}

/// Un pret amortissable a mensualites constantes, simule mois par mois.
#[derive(Debug, Clone)]
pub struct Loan {
    balance: f64,
    monthly_rate: f64,
    payment: f64,
    months_left: u32,
}

impl Loan {
    /// Ouvre un pret de `principal` euros sur `years` annees.
    #[must_use]
    pub fn new(principal: f64, rate: Rate, years: u32) -> Self {
        let months = years.saturating_mul(12);
        let principal = principal.max(0.0);
        let monthly_rate = rate.monthly_nominal();
        Self {
            balance: principal,
            monthly_rate,
            payment: monthly_payment(principal, monthly_rate, months),
            months_left: months,
        }
    }

    /// La mensualite, assurance emprunteur exclue.
    #[must_use]
    pub fn payment(&self) -> f64 {
        self.payment
    }

    /// Le capital restant du a cet instant de la simulation.
    #[must_use]
    pub fn balance(&self) -> f64 {
        self.balance
    }

    /// Vrai des que le pret est solde.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        self.balance <= 0.005 || self.months_left == 0
    }

    /// Avance d'un mois et renvoie l'echeance correspondante.
    ///
    /// Une fois le pret solde, les echeances suivantes sont nulles : la
    /// simulation peut continuer jusqu'a l'horizon sans cas particulier.
    pub fn advance_month(&mut self) -> Installment {
        if self.is_settled() {
            self.balance = 0.0;
            return Installment::default();
        }
        let interest = self.balance * self.monthly_rate;
        let principal = (self.payment - interest).min(self.balance);
        self.balance = (self.balance - principal).max(0.0);
        self.months_left = self.months_left.saturating_sub(1);
        Installment { interest, principal }
    }
}

/// Fait croitre un capital d'un mois au taux annuel donne.
///
/// La composition est actuarielle : douze appels equivalent exactement a une
/// annee au taux affiche.
#[must_use]
pub fn grow_one_month(value: f64, rate: Rate) -> f64 {
    value * (1.0 + rate.monthly_effective())
}

/// La valeur d'un capital apres `years` annees de croissance composee.
///
/// Un taux negatif modelise une decote — un vehicule ou du materiel.
#[must_use]
pub fn compound(value: f64, rate: Rate, years: f64) -> f64 {
    value * (1.0 + rate.as_decimal()).powf(years)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_rate_loan_repays_linearly() {
        assert!((monthly_payment(12_000.0, 0.0, 12) - 1_000.0).abs() < 1e-9);
    }

    #[test]
    fn the_payment_matches_the_standard_annuity_formula() {
        // 200 000 € a 3 %/an sur 20 ans : mensualite de reference 1 109,20 €.
        let rate = Rate::from_percent(3.0);
        let payment = monthly_payment(200_000.0, rate.monthly_nominal(), 240);
        assert!((payment - 1_109.20).abs() < 0.05, "mensualite obtenue : {payment}");
    }

    #[test]
    fn a_loan_amortises_exactly_to_zero() {
        let mut loan = Loan::new(200_000.0, Rate::from_percent(3.0), 20);
        for _ in 0..240 {
            loan.advance_month();
        }
        assert!(loan.balance() < 0.01, "capital restant : {}", loan.balance());
        assert!(loan.is_settled());
    }

    #[test]
    fn total_interest_paid_is_the_difference_with_the_principal() {
        let mut loan = Loan::new(200_000.0, Rate::from_percent(3.0), 20);
        let mut interest = 0.0;
        let mut principal = 0.0;
        for _ in 0..240 {
            let installment = loan.advance_month();
            interest += installment.interest;
            principal += installment.principal;
        }
        assert!((principal - 200_000.0).abs() < 0.01);
        assert!((interest - 66_207.0).abs() < 50.0, "interets obtenus : {interest}");
    }

    #[test]
    fn a_settled_loan_stops_charging() {
        let mut loan = Loan::new(1_000.0, Rate::from_percent(5.0), 1);
        for _ in 0..12 {
            loan.advance_month();
        }
        assert_eq!(loan.advance_month(), Installment::default());
    }

    #[test]
    fn twelve_monthly_growths_equal_one_annual_compounding() {
        let rate = Rate::from_percent(5.0);
        let mut monthly = 1_000.0;
        for _ in 0..12 {
            monthly = grow_one_month(monthly, rate);
        }
        assert!((monthly - compound(1_000.0, rate, 1.0)).abs() < 1e-9);
    }

    #[test]
    fn a_negative_rate_models_depreciation() {
        let value = compound(30_000.0, Rate::from_percent(-15.0), 3.0);
        assert!(value < 30_000.0 && value > 18_000.0, "valeur obtenue : {value}");
    }
}
