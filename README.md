<p align="center"><img src="design/logo.svg" width="96" alt=""></p>

<h1 align="center">Kaury Clean</h1>

<p align="center">Un nettoyeur de PC Windows simple et beau, inspiré de CleanMyMac.<br>Par <a href="https://kaury.studio">Kaury Studio</a>.</p>

<p align="center"><img src="design/screenshot.png" alt="Kaury Clean après une analyse" width="820"></p>

## Ce qu'il fait

| Module | Ce qu'il fait |
| --- | --- |
| **Analyse intelligente** | Un clic pour voir tout ce qui peut partir, puis tu choisis. |
| **Fichiers système** | Fichiers temporaires, téléchargements Windows Update, rapports d'erreur, cache de la carte graphique, cache média Adobe (Premiere Pro, After Effects). |
| **Navigateurs** | Cache de Chrome, Edge, Brave et Firefox. Mots de passe, favoris, historique et sessions ne sont jamais touchés. |
| **Corbeille** | Vide la corbeille de tous les disques. |
| **Gros fichiers** | Liste les fichiers les plus lourds de tes dossiers perso, avec un bouton pour les afficher dans l'Explorateur. Rien n'est coché d'avance. |
| **Doublons** | Trouve les fichiers identiques (comparaison du contenu, pas seulement du nom) et garde la copie la plus récente. |
| **Démarrage** | Active ou désactive les applis lancées avec Windows (registre et dossiers Démarrage), comme le Gestionnaire des tâches. |

L'analyse affiche en direct ce qu'elle parcourt, et l'appli garde le compte de l'espace libéré depuis l'installation.

### Droits administrateur

Kaury Clean démarre sans droits particuliers. Les dossiers protégés de Windows (`C:\Windows\Temp`, Windows Update) sont alors signalés « admin » et décochés. Le lien **Relancer en administrateur** rouvre l'appli avec les droits, après la confirmation de Windows.

## Sécurité

- Les dossiers nettoyés sont fixés dans le code Rust. L'interface envoie seulement des identifiants, jamais de chemins.
- Les fichiers temporaires de moins de 24 h sont gardés, et les fichiers utilisés par une appli ouverte sont ignorés.
- Les liens et jonctions ne sont jamais suivis : on ne sort jamais du dossier nettoyé.
- Gros fichiers et doublons partent **à la corbeille**, jamais supprimés directement, et seulement depuis tes dossiers perso.
- Démarrage : rien n'est supprimé, l'appli est juste marquée « désactivée ». Un clic pour la réactiver.
- Pas de « nettoyage du registre » : ça ne rend pas le PC plus rapide et ça peut casser Windows.

## Télécharger

Chaque push sur `main` compile l'appli sur GitHub Actions. L'installeur (`.exe`) se trouve dans l'onglet **Actions**, dans l'artefact `kaury-clean-windows`.
Pour une version officielle, crée un tag `v0.1.0` : l'installeur est alors publié dans **Releases**.

## Développer

Il faut [Node.js](https://nodejs.org) et [Rust](https://rustup.rs).

```bash
npm install
npm run dev      # lance l'appli
npm run build    # crée l'installeur Windows
cargo test --manifest-path src-tauri/Cargo.toml
```

L'interface (`src/`) est en HTML, CSS et JavaScript sans framework. Ouverte directement dans un navigateur, elle tourne en **mode démo** avec des données d'exemple, pratique pour travailler le design.
Le moteur (`src-tauri/src/`) est en Rust avec [Tauri 2](https://tauri.app).

```
src/                interface
src-tauri/src/
  junk.rs           fichiers inutiles : analyse et nettoyage
  files.rs          gros fichiers, doublons, corbeille
  startup.rs        applis au démarrage
  fsutil.rs         mesure et vidage de dossiers
design/             logo et captures
```
