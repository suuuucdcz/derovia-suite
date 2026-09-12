//! # derovia-arbitrage
//!
//! Le moteur de decision « acheter ou louer » de la suite Derovia.
//!
//! Un seul moteur repond a trois questions qui sont la meme question : faut-il
//! acheter un logement, une voiture, ou une machine ? Ce qui change d'un cas a
//! l'autre n'est pas la mecanique mais les hypotheses — une maison prend de la
//! valeur, une voiture en perd un cinquieme en sortant de la concession, une
//! machine s'amortit fiscalement. Le module [`presets`] fournit ces trois jeux
//! d'hypotheses ; le module [`engine`] fait tourner la simulation.
//!
//! ## Comment la comparaison est menee
//!
//! Comparer une mensualite de credit a un loyer ne veut rien dire : l'une
//! construit du patrimoine, l'autre non, et l'apport immobilise aurait pu etre
//! place. Le moteur compare donc deux personnes ayant le meme budget mensuel :
//! chaque mois, celle dont l'option coute le moins cher place la difference au
//! taux de rendement du marche. A l'horizon, on additionne pour chacune son
//! capital place, la valeur de revente du bien et ce qu'elle doit encore.
//! C'est cet ecart de patrimoine net qui tranche.
//!
//! ```
//! use derovia_arbitrage::{engine, presets};
//!
//! let scenario = presets::real_estate();
//! let verdict = engine::run(&scenario).expect("le preset est valide");
//! println!("{:?} — ecart : {} €", verdict.recommendation, verdict.net_advantage);
//! ```

pub mod engine;
pub mod model;
pub mod presets;
pub mod result;

pub use engine::run;
pub use model::{Arbitrage, Buy, Market, Rent, Tax};
pub use result::{Outcome, Recommendation, Verdict, YearPoint};
