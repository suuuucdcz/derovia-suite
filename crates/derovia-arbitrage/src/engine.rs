//! La simulation, mois par mois.
//!
//! Le principe tient en une phrase : deux personnes ont le meme budget mensuel,
//! l'une achete, l'autre loue, et celle dont l'option coute le moins cher place
//! la difference. A l'horizon on compare ce qu'il reste a chacune.

use derovia_core::{Rate, Result, finance::Loan};

use crate::{
    model::{Arbitrage, Tax},
    result::{Outcome, Recommendation, Verdict, YearPoint},
};

/// En dessous de cet ecart relatif, le moteur refuse de trancher.
///
/// Un avantage de 2 % sur un horizon de vingt ans se joue entierement dans la
/// marge d'erreur des hypotheses. Annoncer une decision ferme serait mentir sur
/// la precision reelle du calcul.
const DECISIVE_MARGIN: f64 = 0.03;

/// Lance un arbitrage et renvoie le verdict.
///
/// # Erreurs
///
/// Renvoie une erreur de validation si les hypotheses sont inexploitables :
/// horizon nul, prix negatif, credit impossible. Voir [`Arbitrage::validate`].
pub fn run(arbitrage: &Arbitrage) -> Result<Verdict> {
    arbitrage.validate()?;
    Ok(simulate(arbitrage))
}

/// Le coefficient de passage aux montants hors taxes.
///
/// Vaut 1 pour un particulier : la TVA payee est definitivement perdue et fait
/// donc partie du cout reel.
fn vat_coefficient(tax: Option<&Tax>) -> f64 {
    match tax {
        Some(tax) if tax.vat_recoverable => 1.0 / (1.0 + tax.vat_rate.as_decimal()),
        _ => 1.0,
    }
}

/// La valeur du bien apres `years` annees, nette des frais de revente.
fn net_asset_value(arbitrage: &Arbitrage, initial_value: f64, years: f64) -> f64 {
    let gross = initial_value * (1.0 + arbitrage.buy.value_change.as_decimal()).powf(years);
    gross * (1.0 - arbitrage.buy.resale_fees.as_decimal())
}

#[allow(clippy::too_many_lines, reason = "une simulation lineaire se lit mieux d'un seul bloc")]
fn simulate(arbitrage: &Arbitrage) -> Verdict {
    let tax = arbitrage.tax.as_ref();
    let net_of_vat = vat_coefficient(tax);

    // --- Position de depart -------------------------------------------------
    let price = arbitrage.buy.price * net_of_vat;
    let acquisition = price * (1.0 + arbitrage.buy.acquisition_fees.as_decimal());
    let down_payment = arbitrage.buy.down_payment.min(acquisition);
    let loan_principal = (acquisition - down_payment).max(0.0);

    let mut loan = Loan::new(loan_principal, arbitrage.buy.loan_rate, arbitrage.buy.loan_years);
    let monthly_insurance = loan_principal * arbitrage.buy.loan_insurance.as_decimal() / 12.0;

    // La decote immediate est subie des la signature, pas etalee sur l'annee.
    let initial_value = price * (1.0 - arbitrage.buy.first_year_drop.as_decimal());

    let entry_fees = arbitrage.rent.entry_fees * net_of_vat;
    let deposit = arbitrage.rent.deposit;

    // Les deux profils demarrent avec le meme capital liquide : c'est ce qui rend
    // la comparaison honnete. Celui qui n'immobilise pas son argent le place.
    let initial_capital = down_payment.max(entry_fees + deposit);
    let mut buy_portfolio = initial_capital - down_payment;
    let mut rent_portfolio = initial_capital - entry_fees - deposit;

    let monthly_return = arbitrage.market.investment_return.monthly_effective();

    // --- Compteurs ----------------------------------------------------------
    let mut buy_outflow = down_payment;
    // Le depot de garantie est immobilise, pas depense : il n'entre pas ici.
    let mut rent_outflow = entry_fees;
    let mut interest_paid = 0.0;
    let mut buy_tax_saved = 0.0;
    let mut rent_tax_saved = 0.0;
    let mut buy_first_year = 0.0;
    let mut rent_first_year = 0.0;

    let capacity = usize::try_from(arbitrage.horizon_years).unwrap_or(0) + 1;
    let mut timeline = Vec::with_capacity(capacity);
    let opening_asset = net_asset_value(arbitrage, initial_value, 0.0);
    timeline.push(YearPoint {
        year: 0,
        buy_net_worth: buy_portfolio + opening_asset - loan.balance(),
        rent_net_worth: rent_portfolio + deposit,
        buy_cumulative_cost: buy_outflow,
        rent_cumulative_cost: rent_outflow,
        asset_value: opening_asset,
        remaining_debt: loan.balance(),
    });

    // --- Simulation ---------------------------------------------------------
    let months = arbitrage.horizon_years.saturating_mul(12);
    for month in 1..=months {
        let year_index = (month - 1) / 12;
        let elapsed = f64::from(year_index);
        let cost_drift = (1.0 + arbitrage.market.inflation.as_decimal()).powf(elapsed);
        let rent_drift = (1.0 + arbitrage.rent.rent_indexation.as_decimal()).powf(elapsed);

        // Cote achat : echeance, assurance emprunteur, entretien, charges fixes.
        let installment = loan.advance_month();
        interest_paid += installment.interest;
        let insurance = if loan.is_settled() { 0.0 } else { monthly_insurance };
        let maintenance = price * arbitrage.buy.maintenance.as_decimal() / 12.0 * cost_drift;
        let charges = arbitrage.buy.yearly_charges / 12.0 * cost_drift;
        let mut buy_cost = installment.total() + insurance + maintenance + charges;

        // Cote location : loyer indexe et charges.
        let rent_amount = arbitrage.rent.monthly_rent * net_of_vat * rent_drift;
        let rent_charges = arbitrage.rent.monthly_charges * cost_drift;
        let mut rent_cost = rent_amount + rent_charges;

        // Fiscalite : les charges deductibles reviennent en economie d'impot.
        if let Some(tax) = tax {
            let profit_tax = tax.profit_tax.as_decimal();
            let depreciation = if tax.depreciation_years > 0 && year_index < tax.depreciation_years
            {
                price / f64::from(tax.depreciation_years) / 12.0
            } else {
                0.0
            };
            let deductible =
                installment.interest + insurance + maintenance + charges + depreciation;
            let buy_saving = deductible * profit_tax;
            buy_tax_saved += buy_saving;
            buy_cost -= buy_saving;

            let rent_saving = rent_cost * profit_tax;
            rent_tax_saved += rent_saving;
            rent_cost -= rent_saving;
        }

        buy_outflow += buy_cost;
        rent_outflow += rent_cost;
        if year_index == 0 {
            buy_first_year += buy_cost;
            rent_first_year += rent_cost;
        }

        // Le budget mensuel commun est celui de l'option la plus chere : l'autre
        // place l'ecart. C'est la seule facon de valoriser l'argent economise.
        let budget = buy_cost.max(rent_cost);
        buy_portfolio = buy_portfolio.mul_add(1.0 + monthly_return, budget - buy_cost);
        rent_portfolio = rent_portfolio.mul_add(1.0 + monthly_return, budget - rent_cost);

        if month.is_multiple_of(12) {
            let year = month / 12;
            let asset = net_asset_value(arbitrage, initial_value, f64::from(year));
            timeline.push(YearPoint {
                year,
                buy_net_worth: buy_portfolio + asset - loan.balance(),
                rent_net_worth: rent_portfolio + deposit,
                buy_cumulative_cost: buy_outflow,
                rent_cumulative_cost: rent_outflow,
                asset_value: asset,
                remaining_debt: loan.balance(),
            });
        }
    }

    // --- Position finale ----------------------------------------------------
    let horizon = f64::from(arbitrage.horizon_years);
    let final_asset = net_asset_value(arbitrage, initial_value, horizon);
    let remaining_debt = loan.balance();

    let buy = Outcome {
        total_outflow: buy_outflow,
        net_worth: buy_portfolio + final_asset - remaining_debt,
        asset_value: final_asset,
        remaining_debt,
        portfolio: buy_portfolio,
        monthly_effort_first_year: buy_first_year / 12.0,
        interest_paid,
        tax_saved: buy_tax_saved,
    };

    // En location avec option d'achat, l'option n'existe qu'au terme du contrat :
    // elle est levee a l'horizon et pas avant, d'ou la marche sur le dernier point.
    let (rent_asset, rent_buyout) = match arbitrage.rent.buyout_option {
        Some(buyout) => (final_asset, buyout),
        None => (0.0, 0.0),
    };
    let rent = Outcome {
        total_outflow: rent_outflow + rent_buyout,
        net_worth: rent_portfolio + deposit + rent_asset - rent_buyout,
        asset_value: rent_asset,
        remaining_debt: 0.0,
        portfolio: rent_portfolio,
        monthly_effort_first_year: rent_first_year / 12.0,
        interest_paid: 0.0,
        tax_saved: rent_tax_saved,
    };

    if let Some(last) = timeline.last_mut() {
        last.rent_net_worth = rent.net_worth;
        last.rent_cumulative_cost = rent.total_outflow;
    }

    let net_advantage = buy.net_worth - rent.net_worth;
    let break_even_year = timeline
        .iter()
        .find(|point| point.year > 0 && point.buy_net_worth >= point.rent_net_worth)
        .map(|point| point.year);

    let mut verdict = Verdict {
        recommendation: Recommendation::TooClose,
        horizon_years: arbitrage.horizon_years,
        break_even_year,
        buy,
        rent,
        net_advantage,
        timeline,
    };
    verdict.recommendation = decide(&verdict);
    verdict
}

/// Tranche, ou refuse de trancher quand l'ecart tient dans le bruit.
fn decide(verdict: &Verdict) -> Recommendation {
    let relative = verdict.relative_advantage();
    if relative > DECISIVE_MARGIN {
        Recommendation::Buy
    } else if relative < -DECISIVE_MARGIN {
        Recommendation::Rent
    } else {
        Recommendation::TooClose
    }
}

/// Le rendement de placement au-dela duquel la location repasse devant.
///
/// Toutes choses egales par ailleurs, c'est le rendement que doit atteindre le
/// capital place pour que louer devienne le meilleur choix. C'est la facon la
/// plus parlante de dire a quel point la conclusion est fragile : si le seuil
/// est a 12 %, la decision d'acheter est solide ; s'il est a 4 %, elle ne l'est pas.
///
/// Renvoie `None` quand aucun rendement entre 0 et 30 % ne fait basculer la decision.
#[must_use]
pub fn indifference_return(arbitrage: &Arbitrage) -> Option<Rate> {
    let mut probe = arbitrage.clone();
    let advantage_at = |rate: f64, probe: &mut Arbitrage| {
        probe.market.investment_return = Rate::from_decimal(rate);
        simulate(probe).net_advantage
    };

    let (mut low, mut high) = (0.0_f64, 0.30_f64);
    // Acheter doit gagner a rendement nul et perdre a 30 % : sinon le seuil est
    // hors de l'intervalle exploré et la question ne se pose pas.
    if advantage_at(low, &mut probe) < 0.0 || advantage_at(high, &mut probe) > 0.0 {
        return None;
    }
    // Trente dichotomies ramenent l'intervalle bien en dessous du point de base.
    for _ in 0..30 {
        let middle = f64::midpoint(low, high);
        if advantage_at(middle, &mut probe) > 0.0 {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(Rate::from_decimal(f64::midpoint(low, high)))
}
