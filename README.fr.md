<p align="center">
  <img src="design/behance/01-couverture.png" alt="Kaury Clean : ton PC, tout propre." width="100%">
</p>

<p align="center">
  <a href="https://github.com/theoblondel/kaury-clean/releases/latest"><img src="https://img.shields.io/github/v/release/theoblondel/kaury-clean?style=for-the-badge&color=F56E2E&labelColor=1C1A1A&label=version" alt="Dernière version"></a>
  <a href="https://github.com/theoblondel/kaury-clean/releases"><img src="https://img.shields.io/github/downloads/theoblondel/kaury-clean/total?style=for-the-badge&color=F1E8CB&labelColor=1C1A1A&label=t%C3%A9l%C3%A9chargements" alt="Téléchargements"></a>
  <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-F1E8CB?style=for-the-badge&labelColor=1C1A1A" alt="Windows 10 et 11">
  <a href="https://github.com/theoblondel/kaury-clean/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/theoblondel/kaury-clean/build.yml?branch=main&style=for-the-badge&labelColor=1C1A1A&label=build" alt="Compilation"></a>
</p>

<p align="center">
  <b>Un nettoyeur de PC Windows simple, beau et honnête.</b><br>
  Il ne touche qu'à ce qui s'accumule pour rien, et te montre tout avant.<br>
  Fait en Rust avec Tauri : 1,6 Mo, sans pub, sans compte, sans télémétrie.
</p>

<p align="center">
  <a href="https://github.com/theoblondel/kaury-clean/releases/latest"><img src="https://img.shields.io/badge/T%C3%A9l%C3%A9charger%20pour%20Windows-F56E2E?style=for-the-badge&logo=windows&logoColor=1C1A1A" alt="Télécharger pour Windows" height="42"></a>
</p>

<p align="center"><a href="README.md">English</a> · <b>Français</b></p>

> L'appli parle français et anglais : elle suit la langue de Windows, et tu peux changer dans **À propos**.

<p align="center"><sub>Kaury Clean te plaît ? Une ⭐ en haut à droite aide les autres à le trouver.</sub></p>

---

## Entretien intelligent

Un seul bouton. Kaury Clean passe en revue les fichiers inutiles, les applis au démarrage, la mémoire, le disque, les applis installées et les vieux téléchargements, puis présente un bilan en cartes. **Lancer** nettoie ce qui est sûr, section par section, et vide le cache DNS. Le reste t'attend dans chaque section.

<table>
  <tr>
    <td width="50%"><img src="design/screens/home.jpg" alt="Accueil : Entretien intelligent"></td>
    <td width="50%"><img src="design/screens/results.jpg" alt="Bilan de l'analyse en cartes"></td>
  </tr>
  <tr>
    <td><img src="design/screens/running.jpg" alt="Nettoyage section par section"></td>
    <td><img src="design/screens/done.jpg" alt="Bravo ! Ton PC est en pleine forme."></td>
  </tr>
</table>

## Six sections

| | Section | Ce qu'elle fait |
| --- | --- | --- |
| 🧹 | **Nettoyage** | Fichiers système (temporaires, Windows Update, rapports d'erreur, cache graphique), caches d'applis (Adobe, Spotify, Discord, Slack, Teams, WhatsApp, Steam, Epic Games, VS Code, Cursor, npm, Yarn, pip, uv), navigateurs (Chrome, Edge, Brave, Firefox, Opera, Vivaldi) et corbeille. |
| ⚡ | **Performances** | **Mémoire vive** : les applis qui remplissent la RAM, fermées proprement ou de force. **Démarrage** : active ou désactive les applis lancées avec Windows. |
| 🧩 | **Applications** | Toutes les applis installées avec leur taille, recherche et tri. Lance le désinstalleur officiel. |
| 📁 | **Mes fichiers** | **Place du disque** (la carte de ce qui prend de la place, en lecture seule), **gros fichiers**, **doublons** (comparés par leur contenu), **vieux téléchargements**, et **rangement** de Téléchargements ou du Bureau par type, annulable. |
| 🔧 | **Réparation** | Point de restauration, réparation de Windows (DISM + SFC), anciennes versions de Windows, optimisation et vérification du disque, veille prolongée, cache DNS, Explorateur, icônes, Microsoft Store. |
| ✦ | **Kaury Clean** | La page du studio (site, Behance, Instagram, contact) et les mises à jour. |

<table>
  <tr>
    <td width="33%"><img src="design/screens/clean.jpg" alt="Nettoyage"></td>
    <td width="33%"><img src="design/screens/memory.jpg" alt="Mémoire vive"></td>
    <td width="33%"><img src="design/screens/uninstall.jpg" alt="Désinstaller"></td>
  </tr>
  <tr>
    <td><img src="design/screens/organize.jpg" alt="Ranger mes fichiers"></td>
    <td><img src="design/screens/maintenance.jpg" alt="Réparation"></td>
    <td><img src="design/screens/about.jpg" alt="À propos"></td>
  </tr>
</table>

## Principes

- **Rien ne part sans toi.** Tu vois chaque élément avant de nettoyer. Gros fichiers et doublons passent par la corbeille.
- **Toujours réversible quand c'est possible.** Les applis au démarrage sont désactivées, pas supprimées. Le rangement s'annule d'un clic.
- **Pas de faux « nettoyage de RAM ».** Windows gère déjà la mémoire : vider la RAM ne fait que la remplir à nouveau en ralentissant le PC. Kaury Clean montre les applis gourmandes et te laisse les fermer.
- **Pas de nettoyage du registre.** Il n'accélère rien et peut casser Windows.
- **OneDrive respecté.** Les fichiers restés dans le cloud ne sont jamais lus, donc jamais téléchargés.
- **Les outils de Windows.** Les réparations passent par DISM, SFC, chkdsk et les points de restauration, pas par des recettes maison.

<details>
<summary><b>Sécurité, dans le détail</b></summary>

- Les dossiers nettoyés sont fixés dans le code Rust. L'interface n'envoie que des identifiants, jamais de chemins.
- Les fichiers temporaires de moins de 24 h sont gardés, et les fichiers utilisés par une appli ouverte sont ignorés.
- Les liens et jonctions ne sont jamais suivis : on ne sort jamais du dossier nettoyé. Chaque fichier est effacé par son vrai emplacement, vérifié au moment même de l'effacer : un dossier remplacé en douce par une jonction vers Windows ne mène nulle part.
- Gros fichiers et doublons : seulement dans tes dossiers perso, et impossible de supprimer toutes les copies d'un même fichier.
- Rangement : seuls les fichiers directement dans le dossier bougent, jamais les sous-dossiers ni les raccourcis, et aucun fichier n'est écrasé.
- Désinstaller et Réparation : seuls les désinstalleurs officiels et les outils de Windows sont lancés, avec des commandes fixées dans le code.
- Mémoire vive : les processus de Windows ne sont jamais proposés à la fermeture.
- Mises à jour : chaque installeur est signé avec une clé qui ne quitte jamais le PC de Kaury Studio. Sans signature valide, rien n'est lancé, même publié sur ce repo. L'installeur vérifié reste verrouillé jusqu'à son lancement.
- Droits administrateur : l'appli démarre sans. Les dossiers protégés de Windows sont signalés « admin » et décochés, avec un lien pour relancer en administrateur.
- En administrateur, l'appli ne peut pas servir de relais à un autre programme : les dossiers de ton compte ne sont touchés que s'ils sont vraiment dans ton dossier personnel (une variable TEMP ou un dossier Documents détourné vers Windows est ignoré), les outils de Windows sont lancés par leur chemin complet, les applis installées pour ton compte seul ne se désinstallent pas avec ces droits, et l'annulation d'un rangement refuse tout déplacement qu'elle n'a pas fait elle-même. Tes fichiers perso (corbeille, rangement) ne se déplacent jamais avec ces droits, et les variables `WEBVIEW2_*`, qui permettraient de remplacer le moteur d'affichage, sont ignorées.

</details>

## Installer

1. Télécharge `Kaury.Clean_…_x64-setup.exe` depuis la **[dernière release](https://github.com/theoblondel/kaury-clean/releases/latest)**.
2. Ouvre-le. Si Windows affiche « Windows a protégé votre ordinateur », clique sur **Informations complémentaires** puis **Exécuter quand même** : l'appli n'est pas encore signée avec un certificat.
3. Kaury Clean s'installe dans Programmes, avec un raccourci dans le menu Démarrer (dossier Kaury Studio) et sur le Bureau. Il se désinstalle depuis **Paramètres > Applications**.

**Mises à jour :** à chaque ouverture, Kaury Clean vérifie s'il existe une nouvelle version et propose de l'installer en un clic. L'installeur téléchargé par l'appli remplace l'ancienne version et garde tes réglages.

## Développer

Il faut [Node.js](https://nodejs.org) et [Rust](https://rustup.rs).

```bash
npm install
npm run dev      # lance l'appli
npm run build    # crée l'installeur Windows
cargo test --manifest-path src-tauri/Cargo.toml
```

L'interface est en HTML, CSS et JavaScript, sans framework. Ouverte directement dans un navigateur (`src/index.html`), elle tourne en **mode démo** avec des données d'exemple, pratique pour travailler le design. Le moteur est en Rust avec [Tauri 2](https://tauri.app).

```
src/
  index.html, styles.css
  main.js             navigation, entretien intelligent, nettoyage, fichiers, démarrage
  modules.js          vieux téléchargements, rangement, mémoire, désinstaller, réparation, À propos, mises à jour
src-tauri/src/
  junk.rs             fichiers inutiles : analyse et nettoyage
  files.rs            gros fichiers, doublons, vieux téléchargements, corbeille
  organize.rs         rangement des dossiers par type
  memory.rs           mémoire vive et fermeture d'applis
  startup.rs          applis au démarrage
  uninstall.rs        applis installées
  maintenance.rs      outils de réparation de Windows
  update.rs           vérification et installation des mises à jour
  elevation.rs        droits administrateur, ouverture des liens
  fsutil.rs           mesure et vidage de dossiers
  langue.rs           français ou anglais, choisi par l'interface
  garde.rs            garde-fous : vrais chemins, règles en administrateur, outils par chemin complet
  space.rs            place du disque (lecture seule)
design/
  logo.svg, k.svg     icône de l'appli et monogramme Kaury
  screens/            captures de l'appli
  behance/            visuels de présentation (FR, et EN dans behance/en)
```

### Publier une nouvelle version

1. Change le numéro de version dans `package.json`, `src-tauri/Cargo.toml` et `src-tauri/tauri.conf.json`, puis pousse sur `main`.
2. Sur GitHub : **Releases** > **Draft a new release** > **Choose a tag**, tape `v0.8.0` et choisis **Create new tag**. Clique sur **Publish release**.
3. GitHub Actions compile l'installeur et l'ajoute à la release (environ 5 minutes).
4. Une fois le build terminé, signe l'installeur depuis ton PC : `npm run signer`. Le fichier `.sig` est ajouté à la release, et les applis déjà installées proposent alors la mise à jour à leur prochaine ouverture. Sans cette étape, elles ne la proposent pas.

La clé de signature se crée une seule fois (`npm run cle`) et vit dans `%USERPROFILE%\.kaury-clean\`, jamais dans ce repo. Garde-en une sauvegarde : sans elle, les versions installées n'accepteront plus aucune mise à jour.

Chaque push sur `main` compile aussi l'installeur (onglet **Actions**, artefact `kaury-clean-windows`), pratique pour tester avant de publier.

---

## Licence

Kaury Clean est open source, sous [licence MIT](LICENSE).

<p align="center">
  <img src="design/logo.svg" width="56" alt=""><br>
  Conçu et développé par <a href="https://kaury.studio"><b>Kaury Studio</b></a>, Vevey<br>
  <a href="https://www.behance.net/kaurystudio">Behance</a> · <a href="https://www.instagram.com/kaury.studio/">Instagram</a> · <a href="mailto:hello@kaury.studio">hello@kaury.studio</a>
</p>
