//! Script de build de l'application.
//!
//! `tauri_build::build()` lit `tauri.conf.json`, genere le contexte compile
//! dans le binaire et embarque les ressources Windows : icone, manifeste et
//! metadonnees de version.

fn main() {
    tauri_build::build();
}
