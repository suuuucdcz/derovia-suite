//! Affiche le detail d'un arbitrage en console.
//!
//! Sert a calibrer les presets et a verifier a l'oeil ce que produit le moteur,
//! sans passer par l'interface.
//!
//! ```sh
//! cargo run -p derovia-arbitrage --example rapport -- immobilier 25
//! ```

use derovia_arbitrage::{engine, presets::Preset};
use derovia_core::{format_eur, format_eur_precise};

fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "immobilier".to_owned());
    let horizon: u32 = args.next().and_then(|value| value.parse().ok()).unwrap_or(25);

    let Some(preset) = Preset::from_id(&id) else {
        eprintln!("Preset inconnu : {id}. Attendu : immobilier, vehicule ou materiel.");
        std::process::exit(1);
    };

    let mut scenario = preset.scenario();
    scenario.horizon_years = horizon;

    let verdict = match engine::run(&scenario) {
        Ok(verdict) => verdict,
        Err(error) => {
            eprintln!("Hypotheses inexploitables : {error}");
            std::process::exit(1);
        }
    };

    println!("\n{} — {} ans", preset.title(), horizon);
    println!("{}", "-".repeat(52));
    println!("{:<28}{:>10}{:>14}", "", preset.buy_label(), preset.rent_label());
    let line = |label: &str, left: f64, right: f64| {
        println!("{:<28}{:>10}{:>14}", label, format_eur(left), format_eur(right));
    };
    line("Sorties de tresorerie", verdict.buy.total_outflow, verdict.rent.total_outflow);
    line("Capital place", verdict.buy.portfolio, verdict.rent.portfolio);
    line("Valeur du bien (nette)", verdict.buy.asset_value, verdict.rent.asset_value);
    line("Dette residuelle", verdict.buy.remaining_debt, verdict.rent.remaining_debt);
    line("Interets payes", verdict.buy.interest_paid, verdict.rent.interest_paid);
    line("Economie d'impot", verdict.buy.tax_saved, verdict.rent.tax_saved);
    println!("{}", "-".repeat(52));
    line("Patrimoine net", verdict.buy.net_worth, verdict.rent.net_worth);
    println!(
        "{:<28}{:>10}{:>14}",
        "Effort mensuel (an 1)",
        format_eur_precise(verdict.buy.monthly_effort_first_year),
        format_eur_precise(verdict.rent.monthly_effort_first_year),
    );

    println!("\nVerdict : {}", verdict.recommendation.label());
    println!("Ecart   : {}", format_eur(verdict.net_advantage));
    match verdict.break_even_year {
        Some(year) => println!("Bascule : annee {year}"),
        None => println!("Bascule : jamais sur cet horizon"),
    }
    match engine::indifference_return(&scenario) {
        Some(rate) => println!("Seuil   : louer repasse devant au-dela de {rate} de rendement"),
        None => println!("Seuil   : aucun rendement entre 0 et 30 % ne fait basculer la decision"),
    }
    println!();
}
