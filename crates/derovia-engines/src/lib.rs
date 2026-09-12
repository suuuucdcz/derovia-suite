//! # derovia-engines
//!
//! Installation et pilotage des moteurs de conversion externes.
//!
//! Le moteur interne de la suite ([`derovia_convert`]) convertit sans rien
//! installer, mais sa fidelite a des limites : il ne restitue ni les tableaux
//! d'un document Word, ni ses notes de bas de page, ni ses styles.
//!
//! **Pandoc** fait tout cela, et c'est la reference du domaine. Il ne s'embarque
//! pas dans l'installeur — 223 Mo une fois decompresse — mais se telecharge a la
//! demande, une seule fois.
//!
//! ## Ce que Pandoc ne fait pas
//!
//! Deux limites lui sont propres, et l'outil continue donc de s'appuyer sur son
//! moteur interne dans ces cas :
//!
//! - il **ne lit pas le PDF**, aucun format d'entree PDF n'existe chez lui ;
//! - il **ne produit un PDF qu'avec un moteur LaTeX** installe a cote, ce que la
//!   suite n'embarque pas.
//!
//! ## Degradation choisie
//!
//! Tant que Pandoc n'est pas installe, les conversions continuent de passer par
//! le moteur interne. L'outil reste utilisable hors ligne ; Pandoc l'ameliore,
//! il ne le deverrouille pas.

pub mod install;
pub mod libreoffice;
pub mod pandoc;

pub use install::{
    ArchiveKind, ENGINES, EngineSpec, EngineStatus, LIBREOFFICE, PANDOC, all_statuses, find,
    install, install_pandoc, pandoc_status, status,
};
pub use pandoc::{handles, run_pandoc};
