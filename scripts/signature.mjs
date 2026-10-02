// Signature des mises à jour de Kaury Clean.
//
// L'appli n'installe une mise à jour que si son installeur porte une signature faite avec la clé
// privée ci-dessous. Cette clé reste sur ton PC, jamais sur GitHub : même avec ton compte GitHub
// piraté, personne ne peut pousser un faux Kaury Clean chez les utilisateurs.
//
//   npm run cle              crée la clé (une seule fois) et affiche la clé publique à mettre dans update.rs
//   npm run signer           signe l'installeur de la dernière release, une fois le build GitHub terminé
//   npm run signer v0.8.0    signe une release précise
//
// SAUVEGARDE la clé privée (clé USB, gestionnaire de mots de passe). Perdue, plus aucune mise à jour
// ne sera acceptée par les versions déjà installées : il faudrait les réinstaller à la main.

import { createPrivateKey, createPublicKey, generateKeyPairSync, sign, verify } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";

const REPO = "theoblondel/kaury-clean";
const KEY_DIR = join(homedir(), ".kaury-clean");
const KEY_FILE = join(KEY_DIR, "signature-mises-a-jour.pem");
// Doit rester identique à SIGNATURE_CONTEXT dans src-tauri/src/update.rs.
const CONTEXT = "kaury-clean/installeur/v1\n";

/** Message signé : contexte + nom du fichier (qui porte la version) + contenu de l'installeur. */
export function message(fileName, installer) {
  return Buffer.concat([Buffer.from(CONTEXT + fileName + "\n", "utf8"), installer]);
}

function publicKeyBytes(key) {
  // Les 32 derniers octets de la clé publique au format SPKI sont la clé Ed25519 brute.
  return createPublicKey(key).export({ type: "spki", format: "der" }).subarray(-32);
}

function gh(args, options = {}) {
  return execFileSync("gh", args, { encoding: "utf8", ...options });
}

function creerCle() {
  if (existsSync(KEY_FILE)) {
    console.error(`La clé existe déjà : ${KEY_FILE}\nOn ne l'écrase jamais (les versions installées ne reconnaîtraient plus tes mises à jour).`);
    process.exit(1);
  }
  mkdirSync(KEY_DIR, { recursive: true });
  const { privateKey } = generateKeyPairSync("ed25519");
  writeFileSync(KEY_FILE, privateKey.export({ type: "pkcs8", format: "pem" }), { mode: 0o600 });
  const bytes = [...publicKeyBytes(privateKey)].map((b) => `0x${b.toString(16).padStart(2, "0")}`);
  console.log(`Clé privée créée : ${KEY_FILE}\n→ SAUVEGARDE-LA maintenant (clé USB, gestionnaire de mots de passe).\n`);
  console.log("Clé publique, pour PUBLIC_KEY dans src-tauri/src/update.rs :\n");
  console.log(`[${bytes.join(", ")}]`);
}

function signer(tag) {
  if (!existsSync(KEY_FILE)) {
    console.error(`Clé introuvable : ${KEY_FILE}\nRemets ta sauvegarde à cet endroit (ou lance « npm run cle » s'il n'y en a jamais eu).`);
    process.exit(1);
  }
  const privateKey = createPrivateKey(readFileSync(KEY_FILE));
  const release = JSON.parse(gh(["release", "view", ...(tag ? [tag] : []), "-R", REPO, "--json", "tagName,assets"]));
  // Un installeur par système : l'installeur NSIS de Windows, le .dmg de macOS.
  const installers = release.assets.filter((a) => a.name.endsWith("-setup.exe") || a.name.endsWith(".dmg"));
  if (!installers.length) {
    console.error(`La release ${release.tagName} n'a pas encore d'installeur : attends la fin du build GitHub Actions, puis relance.`);
    process.exit(1);
  }
  const dir = mkdtempSync(join(tmpdir(), "kaury-signature-"));
  try {
    for (const installer of installers) {
      gh(["release", "download", release.tagName, "-R", REPO, "-p", installer.name, "-D", dir]);
      const data = readFileSync(join(dir, installer.name));
      const signature = sign(null, message(installer.name, data), privateKey);
      if (!verify(null, message(installer.name, data), createPublicKey(privateKey), signature)) {
        throw new Error(`La signature de ${installer.name} ne se vérifie pas`);
      }
      const sigPath = join(dir, `${installer.name}.sig`);
      writeFileSync(sigPath, signature);
      gh(["release", "upload", release.tagName, "-R", REPO, sigPath, "--clobber"], { stdio: "inherit" });
      console.log(`${installer.name}.sig ajouté à la release.`);
    }
    console.log(`\n${release.tagName} signée. Les Kaury Clean installés proposeront la mise à jour dès maintenant.`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const [command, arg] = process.argv.slice(2);
if (command === "cle") creerCle();
else if (command === "signer") signer(arg);
else {
  console.log("Usage : npm run cle | npm run signer [tag]");
  process.exit(1);
}
