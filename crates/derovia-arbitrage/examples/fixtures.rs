//! Enregistre de vraies sorties du moteur, pour le mode developpement du frontend.
//!
//! Quand on ouvre le serveur Vite dans un navigateur plutot que dans la fenetre
//! Tauri, le moteur Rust n'existe pas. L'interface sert alors ce fichier : ce
//! sont de vrais resultats, figes, et non une reimplementation du calcul en
//! TypeScript — qui finirait fatalement par diverger du moteur.
//!
//! ```sh
//! cargo run -p derovia-arbitrage --example fixtures > apps/derovia-arbitrage/src/dev-fixtures.json
//! ```

use derovia_arbitrage::{engine, presets};
use serde_json::{Map, json};

fn main() {
    let mut analyses = Map::new();
    for preset in presets::Preset::ALL {
        let scenario = preset.scenario();
        let Ok(verdict) = engine::run(&scenario) else {
            eprintln!("preset invalide : {}", preset.id());
            std::process::exit(1);
        };
        let indifference = engine::indifference_return(&scenario).map(|rate| rate.as_percent());
        analyses.insert(
            preset.id().to_owned(),
            json!({ "verdict": verdict, "indifferenceReturn": indifference }),
        );
    }

    let document = json!({ "catalogue": presets::catalog(), "analyses": analyses });
    match serde_json::to_string_pretty(&document) {
        Ok(text) => println!("{text}"),
        Err(error) => {
            eprintln!("serialisation impossible : {error}");
            std::process::exit(1);
        }
    }
}
