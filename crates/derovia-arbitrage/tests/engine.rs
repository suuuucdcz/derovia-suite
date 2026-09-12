#![allow(
    clippy::expect_used,
    clippy::float_cmp,
    clippy::indexing_slicing,
    reason = "un test a le droit de paniquer et de comparer des flottants exacts"
)]

//! Tests d'integration du moteur d'arbitrage.
//!
//! Ces tests ne verifient pas des montants au centime pres — ils dependraient
//! alors du moindre ajustement d'hypothese. Ils verifient les proprietes que le
//! moteur doit respecter quoi qu'il arrive : le sens des variations, la
//! coherence comptable des bilans, et le fait que les cas extremes ne cassent rien.

use derovia_arbitrage::{
    Recommendation,
    engine::{self, indifference_return},
    presets,
};
use derovia_core::Rate;

/// Le bilan doit se recomposer exactement : patrimoine = placement + bien - dette.
#[test]
fn the_balance_sheet_adds_up() {
    for preset in presets::Preset::ALL {
        let verdict = engine::run(&preset.scenario()).expect("preset valide");
        let recomposed =
            verdict.buy.portfolio + verdict.buy.asset_value - verdict.buy.remaining_debt;
        assert!(
            (verdict.buy.net_worth - recomposed).abs() < 0.01,
            "{} : bilan achat incoherent",
            preset.id()
        );
        assert!(
            (verdict.net_advantage - (verdict.buy.net_worth - verdict.rent.net_worth)).abs() < 0.01,
            "{} : ecart incoherent",
            preset.id()
        );
    }
}

/// La trajectoire couvre l'annee zero puis chaque annee jusqu'a l'horizon.
#[test]
fn the_timeline_covers_the_whole_horizon() {
    let scenario = presets::real_estate();
    let verdict = engine::run(&scenario).expect("preset valide");
    assert_eq!(verdict.timeline.len(), scenario.horizon_years as usize + 1);
    assert_eq!(verdict.timeline.first().map(|p| p.year), Some(0));
    assert_eq!(verdict.timeline.last().map(|p| p.year), Some(scenario.horizon_years));
}

/// Sous les hypotheses par defaut, rester plus longtemps favorise l'achat.
///
/// Les frais d'acquisition se payent une fois et se diluent avec le temps. Ce
/// n'est pourtant pas une loi generale : voir
/// [`past_the_indifference_threshold_time_stops_helping_the_buyer`], qui montre
/// le cas ou l'allongement de l'horizon aggrave au contraire la position.
#[test]
fn holding_longer_favours_buying_under_the_default_assumptions() {
    let mut short = presets::real_estate();
    short.horizon_years = 3;
    let mut long = presets::real_estate();
    long.horizon_years = 25;

    let short_advantage = engine::run(&short).expect("valide").net_advantage;
    let long_advantage = engine::run(&long).expect("valide").net_advantage;

    assert!(
        long_advantage > short_advantage,
        "sur 25 ans l'avantage ({long_advantage:.0}) devrait depasser celui sur 3 ans ({short_advantage:.0})"
    );
}

/// Sur trois ans, les frais de notaire et d'agence ne sont pas amortis.
#[test]
fn buying_a_home_for_three_years_is_a_bad_idea() {
    let mut scenario = presets::real_estate();
    scenario.horizon_years = 3;
    let verdict = engine::run(&scenario).expect("valide");
    assert_eq!(verdict.recommendation, Recommendation::Rent);
}

/// Sur vingt-cinq ans, le credit est solde et le bien est acquis.
#[test]
fn buying_a_home_for_twenty_five_years_is_a_good_idea() {
    let mut scenario = presets::real_estate();
    scenario.horizon_years = 25;
    let verdict = engine::run(&scenario).expect("valide");
    assert_eq!(verdict.recommendation, Recommendation::Buy);
    assert!(verdict.buy.remaining_debt < 0.01, "le credit sur 20 ans devrait etre solde");
}

/// Mieux le capital place rapporte, plus la location devient attractive.
#[test]
fn a_better_investment_return_favours_renting() {
    let mut scenario = presets::real_estate();
    scenario.horizon_years = 15;

    scenario.market.investment_return = Rate::from_percent(1.0);
    let cautious = engine::run(&scenario).expect("valide").net_advantage;

    scenario.market.investment_return = Rate::from_percent(12.0);
    let aggressive = engine::run(&scenario).expect("valide").net_advantage;

    assert!(
        aggressive < cautious,
        "a 12 % l'avantage a l'achat ({aggressive:.0}) devrait etre inferieur qu'a 1 % ({cautious:.0})"
    );
}

/// Un loyer plus cher rend l'achat plus interessant, sans exception.
#[test]
fn a_higher_rent_favours_buying() {
    let base = presets::real_estate();
    let mut expensive = base.clone();
    expensive.rent.monthly_rent *= 1.5;

    let base_advantage = engine::run(&base).expect("valide").net_advantage;
    let expensive_advantage = engine::run(&expensive).expect("valide").net_advantage;
    assert!(expensive_advantage > base_advantage);
}

/// Un logement gratuit ne se bat pas : la location gagne forcement.
#[test]
fn a_free_rental_always_wins() {
    let mut scenario = presets::real_estate();
    scenario.rent.monthly_rent = 0.0;
    scenario.rent.monthly_charges = 0.0;
    scenario.rent.entry_fees = 0.0;
    let verdict = engine::run(&scenario).expect("valide");
    assert_eq!(verdict.recommendation, Recommendation::Rent);
    assert_eq!(verdict.break_even_year, None);
}

/// La decote immediate d'un vehicule neuf se retrouve dans sa valeur de revente.
#[test]
fn a_new_vehicle_loses_value_immediately() {
    let scenario = presets::vehicle();
    let verdict = engine::run(&scenario).expect("valide");
    let opening = verdict.timeline.first().expect("l'annee zero existe");
    assert!(
        opening.asset_value < scenario.buy.price * 0.82,
        "la valeur a la signature ({:.0}) devrait deja avoir chute",
        opening.asset_value
    );
}

/// L'amortissement et la TVA recuperable produisent une economie d'impot reelle.
#[test]
fn the_professional_case_generates_tax_savings() {
    let verdict = engine::run(&presets::professional_equipment()).expect("valide");
    assert!(verdict.buy.tax_saved > 0.0, "l'amortissement doit produire une economie");
    assert!(verdict.rent.tax_saved > 0.0, "les loyers sont deductibles");
}

/// Sans profil fiscal, aucune economie d'impot ne doit apparaitre.
#[test]
fn a_private_case_never_saves_tax() {
    let verdict = engine::run(&presets::real_estate()).expect("valide");
    assert_eq!(verdict.buy.tax_saved, 0.0);
    assert_eq!(verdict.rent.tax_saved, 0.0);
}

/// Le point de bascule, quand il existe, est un vrai croisement des courbes.
#[test]
fn the_break_even_year_is_a_real_crossing() {
    let mut scenario = presets::real_estate();
    scenario.horizon_years = 30;
    let verdict = engine::run(&scenario).expect("valide");
    let year = verdict.break_even_year.expect("l'achat finit par gagner sur 30 ans");
    assert!(year >= 1);

    let crossing = verdict.timeline.iter().find(|p| p.year == year).expect("le point existe");
    assert!(crossing.buy_net_worth >= crossing.rent_net_worth);

    if year > 1 {
        let before = verdict.timeline.iter().find(|p| p.year == year - 1).expect("le point existe");
        assert!(
            before.buy_net_worth < before.rent_net_worth,
            "l'annee precedente doit etre perdante"
        );
    }
}

/// Le seuil d'indifference est le rendement qui annule exactement l'ecart.
#[test]
fn the_indifference_return_zeroes_the_advantage() {
    let mut scenario = presets::real_estate();
    scenario.horizon_years = 20;
    let threshold = indifference_return(&scenario).expect("un seuil existe sur ce scenario");

    assert!(
        threshold.as_percent() > 0.0 && threshold.as_percent() < 30.0,
        "seuil hors bornes : {threshold}"
    );

    scenario.market.investment_return = threshold;
    let advantage = engine::run(&scenario).expect("valide").net_advantage;
    let reference = engine::run(&scenario).expect("valide").buy.net_worth.abs();
    assert!(advantage.abs() < reference * 1e-4, "ecart residuel trop grand : {advantage:.2}");
}

/// Un achat comptant ne genere aucun interet et solde immediatement la dette.
#[test]
fn paying_cash_costs_no_interest() {
    let mut scenario = presets::real_estate();
    scenario.buy.down_payment = scenario.total_acquisition_cost();
    scenario.buy.loan_years = 0;
    let verdict = engine::run(&scenario).expect("valide");
    assert_eq!(verdict.buy.interest_paid, 0.0);
    assert_eq!(verdict.buy.remaining_debt, 0.0);
}

/// Les hypotheses inexploitables sont refusees avec un message lisible.
#[test]
fn invalid_assumptions_are_rejected() {
    let mut zero_horizon = presets::real_estate();
    zero_horizon.horizon_years = 0;
    assert!(engine::run(&zero_horizon).is_err());

    let mut negative_price = presets::real_estate();
    negative_price.buy.price = -1.0;
    let error = engine::run(&negative_price).expect_err("un prix negatif est refuse");
    assert!(error.to_string().contains("Prix d'achat"), "message obtenu : {error}");

    let mut impossible_cash = presets::real_estate();
    impossible_cash.buy.loan_years = 0;
    assert!(engine::run(&impossible_cash).is_err(), "sans credit ni apport suffisant");
}

/// Le scenario traverse serde sans rien perdre : l'interface envoie ce type tel quel.
#[test]
fn a_scenario_survives_a_json_round_trip() {
    let scenario = presets::professional_equipment();
    let encoded = serde_json::to_string(&scenario).expect("serialisation");
    let decoded = serde_json::from_str(&encoded).expect("deserialisation");
    assert_eq!(scenario, decoded);
}

/// Le verdict aussi : c'est lui qui repart vers l'interface.
#[test]
fn a_verdict_survives_a_json_round_trip() {
    let verdict = engine::run(&presets::vehicle()).expect("valide");
    let encoded = serde_json::to_string(&verdict).expect("serialisation");
    let decoded = serde_json::from_str(&encoded).expect("deserialisation");
    assert_eq!(verdict, decoded);
}

/// Un horizon d'un an ne doit pas produire de division par zero ni de NaN.
#[test]
fn a_one_year_horizon_stays_finite() {
    for preset in presets::Preset::ALL {
        let mut scenario = preset.scenario();
        scenario.horizon_years = 1;
        let verdict = engine::run(&scenario).expect("valide");
        assert!(verdict.buy.net_worth.is_finite(), "{} : achat non fini", preset.id());
        assert!(verdict.rent.net_worth.is_finite(), "{} : location non finie", preset.id());
    }
}

/// Au-dela du seuil d'indifference, allonger l'horizon ne sauve plus l'achat.
///
/// C'est le resultat contre-intuitif du modele, et il est reel : quand le
/// capital place croit plus vite que le bien, l'ecart se creuse avec le temps
/// au lieu de se resorber. Les frais d'acquisition s'amortissent, mais un
/// ecart de taux ne s'amortit jamais.
#[test]
fn past_the_indifference_threshold_time_stops_helping_the_buyer() {
    let mut scenario = presets::real_estate();
    scenario.market.investment_return = Rate::from_percent(9.0);

    scenario.horizon_years = 15;
    let medium = engine::run(&scenario).expect("valide").net_advantage;
    scenario.horizon_years = 35;
    let long = engine::run(&scenario).expect("valide").net_advantage;

    assert!(
        medium < 0.0 && long < medium,
        "l'ecart devrait se creuser : {medium:.0} puis {long:.0}"
    );
}

/// Quand la location gagne deja a rendement nul, aucun seuil n'existe.
#[test]
fn there_is_no_threshold_when_renting_wins_outright() {
    let mut scenario = presets::real_estate();
    scenario.horizon_years = 3;
    assert_eq!(indifference_return(&scenario), None);
}

/// Une valorisation plus forte du bien favorise l'achat, sans exception.
#[test]
fn faster_appreciation_favours_buying() {
    let base = presets::real_estate();
    let mut booming = base.clone();
    booming.buy.value_change = Rate::from_percent(5.0);

    let base_advantage = engine::run(&base).expect("valide").net_advantage;
    let booming_advantage = engine::run(&booming).expect("valide").net_advantage;
    assert!(booming_advantage > base_advantage);
}
