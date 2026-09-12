//! # derovia-core
//!
//! Primitives partagees par tous les outils de la suite Derovia.
//!
//! Ce crate ne connait aucun produit en particulier : il fournit les briques
//! que chaque outil de la suite reutilise, pour que deux outils differents ne
//! calculent jamais une mensualite ou n'affichent jamais un montant de deux
//! manieres differentes.
//!
//! - [`rate`] : les taux annuels et leurs conversions mensuelles ;
//! - [`finance`] : prets amortissables et capitalisation ;
//! - [`money`] : mise en forme monetaire francaise ;
//! - [`error`] : le type d'erreur commun, serialisable vers l'interface.

pub mod error;
pub mod finance;
pub mod money;
pub mod rate;

pub use error::{CoreError, Result};
pub use money::{format_bytes, format_eur, format_eur_precise, format_number};
pub use rate::Rate;
