#!/bin/bash
# Inventaire des dossiers qu'une version Mac de Kaury Clean pourrait nettoyer.
# LECTURE SEULE : ce script mesure des tailles et liste des noms, il ne supprime
# et ne modifie rien. Le résultat va dans un fichier texte sur le Bureau.
#
#   bash inventaire-mac.sh
#
# Règle de Kaury Clean : on n'ajoute une cible qu'après avoir vu ce qu'elle
# contient vraiment, sur un vrai Mac.

OUT="$HOME/Desktop/inventaire-kaury-clean.txt"
L="$HOME/Library"

taille() { # taille lisible d'un dossier, ou rien s'il n'existe pas
  [ -e "$1" ] && printf "%8s  %s\n" "$(du -sh "$1" 2>/dev/null | cut -f1)" "${1/#$HOME/~}"
}

{
  echo "== Système =="
  sw_vers 2>/dev/null
  echo "Processeur : $(uname -m)"
  df -h / | tail -1
  echo

  echo "== Les 40 plus gros dossiers de ~/Library/Caches =="
  du -sk "$L/Caches"/* 2>/dev/null | sort -rn | head -40 | while read -r k p; do printf "%8s  %s\n" "$((k / 1024)) Mo" "${p/#$HOME/~}"; done
  echo

  echo "== Candidats connus =="
  for d in \
    "$L/Logs" "$HOME/.Trash" \
    "$L/Developer/Xcode/DerivedData" "$L/Developer/Xcode/Archives" "$L/Developer/Xcode/iOS DeviceSupport" \
    "$L/Developer/CoreSimulator/Caches" "$L/Developer/CoreSimulator/Devices" \
    "$L/Caches/Homebrew" "$HOME/.npm/_cacache" "$L/Caches/pip" "$L/Caches/Yarn" "$L/pnpm/store" "$HOME/.cache" \
    "$L/Caches/CocoaPods" "$L/Caches/org.swift.swiftpm" "$L/Caches/org.carthage.CarthageKit" "$HOME/.gradle/caches" \
    "$L/Developer/Xcode/watchOS DeviceSupport" "$L/Caches/JetBrains" "$L/Caches/us.zoom.xos" \
    "$L/Caches/com.operasoftware.Opera" "$L/Caches/com.vivaldi.Vivaldi" \
    "$L/Caches/Google/Chrome" "$L/Application Support/Google/Chrome/Default/Service Worker" \
    "$L/Caches/com.apple.Safari" "$L/Caches/Firefox" "$L/Caches/BraveSoftware" "$L/Caches/com.microsoft.edgemac" \
    "$L/Application Support/discord/Cache" "$L/Application Support/discord/Code Cache" "$L/Application Support/discord/GPUCache" \
    "$L/Caches/com.spotify.client" "$L/Application Support/Spotify/PersistentCache" \
    "$L/Application Support/Slack/Cache" "$L/Application Support/Slack/Service Worker/CacheStorage" \
    "$L/Containers/com.microsoft.teams2/Data/Library/Caches" \
    "$L/Application Support/Code/Cache" "$L/Application Support/Code/CachedData" "$L/Application Support/Code/CachedExtensionVSIXs" \
    "$L/Application Support/Cursor/Cache" "$L/Application Support/Cursor/CachedData" \
    "$L/Caches/Adobe" "$L/Application Support/Adobe/Common/Media Cache Files" "$L/Application Support/Adobe/Common/Media Cache" \
    "$L/Application Support/Steam/appcache" "$L/Application Support/Steam/logs" \
    "$L/Containers/com.apple.mail/Data/Library/Mail Downloads" \
    "$L/Application Support/MobileSync/Backup" \
    "/Library/Caches" "/private/var/log" "/private/var/folders"; do
    taille "$d"
  done
  echo

  echo "== Installeurs dans Téléchargements (.dmg, .pkg, .zip) =="
  find "$HOME/Downloads" -maxdepth 1 \( -iname '*.dmg' -o -iname '*.pkg' -o -iname '*.zip' \) -print0 2>/dev/null \
    | xargs -0 du -sh 2>/dev/null | sort -rh | head -30 | sed "s#$HOME#~#"
  echo

  echo "== Applis lancées à l'ouverture de session =="
  osascript -e 'tell application "System Events" to get the name of every login item' 2>/dev/null
  ls -1 "$L/LaunchAgents" 2>/dev/null | sed 's/^/  ~\/Library\/LaunchAgents\//'
  echo

  echo "== Applis installées (les 25 plus grosses) =="
  du -sk /Applications/*.app 2>/dev/null | sort -rn | head -25 | while read -r k p; do printf "%8s  %s\n" "$((k / 1024)) Mo" "$p"; done
} > "$OUT" 2>/dev/null

echo "Inventaire terminé : $OUT"
echo "Rien n'a été supprimé ni modifié."
