//! Mise en forme monetaire francaise.
//!
//! Toute la suite affiche ses montants via ce module : un seul endroit decide
//! du separateur de milliers, de la virgule decimale et de la place du symbole.

/// Espace insecable etroite, le separateur de milliers de la typographie francaise.
const THIN_NBSP: char = '\u{202f}';

/// Formate un nombre a la francaise : `1 234 567,89`.
///
/// Les milliers sont separes par une espace insecable etroite et la partie
/// decimale par une virgule.
#[must_use]
pub fn format_number(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }

    let rendered = format!("{:.*}", decimals, value.abs());
    let (integer, fraction) = match rendered.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (rendered.as_str(), None),
    };

    let mut grouped = String::with_capacity(integer.len() * 2);
    for (index, digit) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index).is_multiple_of(3) {
            grouped.push(THIN_NBSP);
        }
        grouped.push(digit);
    }

    // Un montant arrondi a zero ne doit jamais s'afficher « -0 ».
    let is_zero = !rendered.chars().any(|c| ('1'..='9').contains(&c));
    let mut out = String::with_capacity(grouped.len() + 4);
    if value < 0.0 && !is_zero {
        out.push('-');
    }
    out.push_str(&grouped);
    if let Some(fraction) = fraction {
        out.push(',');
        out.push_str(fraction);
    }
    out
}

/// Formate un montant en euros, arrondi a l'unite : `1 234 568 €`.
///
/// C'est la forme par defaut : sur un arbitrage a plusieurs annees, les
/// centimes sont du bruit.
#[must_use]
pub fn format_eur(amount: f64) -> String {
    format!("{}{THIN_NBSP}€", format_number(amount, 0))
}

/// Formate un montant en euros au centime pres : `1 234 567,89 €`.
///
/// Reserve aux mensualites, ou le centime est visible sur le releve bancaire.
#[must_use]
pub fn format_eur_precise(amount: f64) -> String {
    format!("{}{THIN_NBSP}€", format_number(amount, 2))
}

/// Formate une taille de fichier a la francaise : `14,50 Mo`.
///
/// Les paliers sont binaires — 1 Ko vaut 1024 octets — et les etiquettes sont
/// celles de l'explorateur Windows (o, Ko, Mo, Go). C'est volontairement la
/// convention de Windows plutot que celle du Systeme international : un
/// utilisateur qui compare la taille affichee ici avec celle de l'explorateur
/// doit lire le meme nombre.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "la conversion u64 -> f64 n'entraine aucune perte perceptible pour l'affichage de tailles"
)]
pub fn format_bytes(bytes: u64) -> String {
    const KO: f64 = 1024.0;
    const MO: f64 = 1024.0 * 1024.0;
    const GO: f64 = 1024.0 * 1024.0 * 1024.0;

    let value = bytes as f64;
    if value < KO {
        format!("{bytes}{THIN_NBSP}o")
    } else if value < MO {
        format!("{}{THIN_NBSP}Ko", format_number(value / KO, 1))
    } else if value < GO {
        format!("{}{THIN_NBSP}Mo", format_number(value / MO, 2))
    } else {
        format!("{}{THIN_NBSP}Go", format_number(value / GO, 2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_are_grouped_by_three() {
        assert_eq!(format_number(1_234_567.0, 0), "1\u{202f}234\u{202f}567");
        assert_eq!(format_number(999.0, 0), "999");
        assert_eq!(format_number(1_000.0, 0), "1\u{202f}000");
    }

    #[test]
    fn decimals_use_a_comma() {
        assert_eq!(format_number(1_234.5, 2), "1\u{202f}234,50");
    }

    #[test]
    fn negative_amounts_keep_their_sign() {
        assert_eq!(format_number(-4_200.0, 0), "-4\u{202f}200");
    }

    #[test]
    fn a_value_rounding_to_zero_never_displays_minus_zero() {
        assert_eq!(format_number(-0.4, 0), "0");
    }

    #[test]
    fn euro_amounts_carry_the_symbol() {
        assert_eq!(format_eur(1_500.0), "1\u{202f}500\u{202f}€");
        assert_eq!(format_eur_precise(1_500.0), "1\u{202f}500,00\u{202f}€");
    }

    #[test]
    fn byte_sizes_are_formatted_cleanly() {
        assert_eq!(format_bytes(512), "512\u{202f}o");
        assert_eq!(format_bytes(1024), "1,0\u{202f}Ko");
        assert_eq!(format_bytes(1_536), "1,5\u{202f}Ko");
        assert_eq!(format_bytes(10_485_760), "10,00\u{202f}Mo");
        assert_eq!(format_bytes(1_073_741_824), "1,00\u{202f}Go");
    }

    #[test]
    fn non_finite_values_degrade_gracefully() {
        assert_eq!(format_number(f64::NAN, 0), "—");
        assert_eq!(format_number(f64::INFINITY, 2), "—");
    }
}
