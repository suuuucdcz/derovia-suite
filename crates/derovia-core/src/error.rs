//! Le type d'erreur commun a la suite.

use serde::Serialize;
use thiserror::Error;

/// Une donnee d'entree hors du domaine autorise.
///
/// Le type est serialisable pour traverser tel quel la frontiere Rust vers
/// l'interface : le message porte deja un texte lisible par l'utilisateur.
#[derive(Debug, Clone, Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoreError {
    /// Un champ contient une valeur impossible.
    #[error("{field} : {reason}")]
    Invalid {
        /// Nom du champ fautif, tel qu'il apparait dans l'interface.
        field: String,
        /// Explication destinee a l'utilisateur final, en francais.
        reason: String,
    },
    /// Une operation de traitement a echoue.
    #[error("{operation} : {message}")]
    Failure {
        /// Nom ou type de l'operation ayant echoue.
        operation: String,
        /// Explication destinee a l'utilisateur final, en francais.
        message: String,
    },
}

impl CoreError {
    /// Raccourci de construction d'une erreur de validation.
    pub fn invalid(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid { field: field.into(), reason: reason.into() }
    }

    /// Raccourci de construction d'une erreur d'operation ou de traitement.
    pub fn failure(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Failure { operation: operation.into(), message: message.into() }
    }
}

/// Le resultat standard de la suite.
pub type Result<T> = core::result::Result<T, CoreError>;
