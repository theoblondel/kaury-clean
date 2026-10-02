// Captures de l'appli pour clean.kaury.studio, prises sur le mode démo (php -S localhost:8799).
// node design/screens/captures-site.mjs [en]
// → design/screens/[en/]<nom>.jpg (2360 × 1560) et clean/src/assets/img/[en/]<nom>-1600.webp / -900.webp
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const require = createRequire('D:/KauryStudio/web/kaury studio/package.json');
const puppeteer = require('puppeteer-core');
const sharp = require('sharp');

const ici = path.dirname(fileURLToPath(import.meta.url));
const EN = process.argv[2] === 'en';
const jpgDir = path.join(ici, EN ? 'en' : '');
const webDir = path.join('D:/KauryStudio/web/clean/src/assets/img', EN ? 'en' : '');
fs.mkdirSync(jpgDir, { recursive: true });
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

const nav = await puppeteer.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
const p = await nav.newPage();
await p.evaluateOnNewDocument((en) => { try { localStorage.setItem('kaury-clean-lang', en ? 'en' : 'fr'); } catch {} }, EN);
// Le mode démo annonce une vieille version et une mise à jour : on montre la version publiée, à jour.
const VERSION = process.env.VERSION || '0.9.4';
await p.setRequestInterception(true);
p.on('request', async (r) => {
  if (!r.url().endsWith('/main.js')) return r.continue();
  const res = await fetch(r.url());
  const js = (await res.text())
    .replace('{ version: "0.8.0"', `{ version: "${VERSION}"`)
    .replace('{ available: true, current: "0.8.0", latest: "0.9.0"', `{ available: false, current: "${VERSION}", latest: "${VERSION}"`);
  r.respond({ status: 200, contentType: 'application/javascript', body: js });
});
await p.setViewport({ width: 1180, height: 780, deviceScaleFactor: 2 });
await p.goto('http://localhost:8799/', { waitUntil: 'networkidle0' });
await pause(1000);

// Pas de fenêtre ni de bandeau de mise à jour sur les captures.
const sansMaj = () => p.evaluate(() => {
  const m = document.querySelector('#updateModal'); if (m) m.hidden = true;
  const b = document.querySelector('#updateBanner'); if (b) b.hidden = true;
  const d = document.querySelector('#updateDot'); if (d) d.hidden = true;
});
async function capture(nom) {
  await sansMaj();
  const png = await p.screenshot();
  await sharp(png).jpeg({ quality: 88, mozjpeg: true }).toFile(path.join(jpgDir, nom + '.jpg'));
  for (const w of [1600, 900]) {
    await sharp(png).resize(w).webp({ quality: 82 }).toFile(path.join(webDir, `${nom}-${w}.webp`));
  }
  console.log('  ' + nom);
}
const vue = async (v, ms = 1500) => { await p.evaluate((v) => show(v), v); await pause(ms); };

await capture('home');
await p.click('#careBtn');
await pause(1700);
await capture('running');
await pause(6000);
await capture('results');
await p.click('#careBtn');
await pause(9500);
await capture('done');

await vue('system', 3000);
await capture('clean');
await vue('memory');
await capture('memory');
await vue('organize');
await capture('organize');
await vue('uninstall', 2500);
await capture('uninstall');
await vue('maintenance');
await capture('maintenance');
await vue('about');
await capture('about');

await nav.close();
console.log('captures prêtes' + (EN ? ' (en)' : ''));
