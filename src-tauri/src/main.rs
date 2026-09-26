// Pas de fenêtre de console derrière l'appli en version finale.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kaury_clean_lib::run()
}
