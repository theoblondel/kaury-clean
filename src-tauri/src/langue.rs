//! Langue des textes envoyés à l'interface : français ou anglais.
//! L'interface la choisit au démarrage (langue de Windows, ou choix fait dans À propos).

use std::sync::atomic::{AtomicBool, Ordering};

static ANGLAIS: AtomicBool = AtomicBool::new(false);

pub fn choisir(langue: &str) {
    ANGLAIS.store(langue == "en", Ordering::Relaxed);
}

pub fn anglais() -> bool {
    ANGLAIS.load(Ordering::Relaxed)
}

/// Le texte dans la langue choisie.
pub fn tr(fr: &'static str, en: &'static str) -> &'static str {
    if anglais() { en } else { fr }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn francais_par_defaut() {
        assert_eq!(tr("Corbeille", "Recycle Bin"), "Corbeille");
    }
}
