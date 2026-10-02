#!/bin/bash
# Installe (ou met à jour) Kaury Clean sur un Mac, en une ligne :
#
#   curl -fsSL https://clean.kaury.studio/installer-mac.sh | bash
#
# Ce que fait ce script, et rien d'autre :
#   1. demande à GitHub l'adresse du dernier .dmg de Kaury Clean ;
#   2. le télécharge dans un dossier temporaire ;
#   3. copie « Kaury Clean.app » dans /Applications (l'ancienne version est remplacée) ;
#   4. démonte le .dmg, supprime le dossier temporaire et ouvre l'appli.
# Un fichier téléchargé par le Terminal n'est pas marqué « venu d'internet » : macOS ouvre donc
# l'appli sans l'alerte réservée aux applis non validées par Apple.
set -euo pipefail

REPO="theoblondel/kaury-clean"
APP="Kaury Clean.app"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Ce script est pour macOS. Sur Windows : https://clean.kaury.studio/telecharger/" >&2
  exit 1
fi

echo "Recherche de la dernière version de Kaury Clean…"
URL=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep -o '"browser_download_url": *"[^"]*\.dmg"' | head -1 | sed 's/.*"\(https[^"]*\)"/\1/')
case "$URL" in
  https://github.com/$REPO/releases/download/*) ;;
  *) echo "Aucune version Mac trouvée pour l'instant." >&2; exit 1 ;;
esac

TMP=$(mktemp -d)
MNT="$TMP/volume"
trap 'hdiutil detach "$MNT" -quiet 2>/dev/null || true; rm -rf "$TMP"' EXIT

echo "Téléchargement de $(basename "$URL")…"
curl -fL --progress-bar "$URL" -o "$TMP/kaury-clean.dmg"

hdiutil attach "$TMP/kaury-clean.dmg" -nobrowse -readonly -mountpoint "$MNT" -quiet
if [ ! -d "$MNT/$APP" ]; then
  echo "Le .dmg ne contient pas $APP : installation annulée." >&2
  exit 1
fi

# L'appli ouverte serait remplacée en cours d'utilisation : on la ferme d'abord.
osascript -e 'tell application "Kaury Clean" to quit' >/dev/null 2>&1 || true

echo "Installation dans Applications…"
rm -rf "/Applications/$APP"
cp -R "$MNT/$APP" /Applications/

echo "Kaury Clean est installé. Ouverture…"
open "/Applications/$APP"
