// Couverture du README (FR et EN) et image de partage GitHub, avec le logo balai.
// node design/social/couverture.mjs  →  design/social/couverture-fr.png, couverture-en.png, preview.png
// Les captures viennent de design/screens/ (captures-site.mjs), les polices de src/fonts/.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const require = createRequire('D:/KauryStudio/web/kaury studio/package.json');
const puppeteer = require('puppeteer-core');
const { ICONE, DESSIN_BALAI } = await import(pathToFileURL('D:/KauryStudio/web/clean/src/icones.mjs').href);

const ici = path.dirname(fileURLToPath(import.meta.url));
const racine = path.resolve(ici, '../..');
const url = (p) => pathToFileURL(path.join(racine, p)).href;

const TEXTES = {
  fr: { titre: 'Ton PC,<br>tout propre.', sous: 'Nettoyeur de PC gratuit · Windows et Mac · Open source' },
  en: { titre: 'Your PC,<br>spotless.', sous: 'Free PC cleaner · Windows and Mac · Open source' },
};

const css = `
@font-face{font-family:Unbounded;src:url(${url('src/fonts/Unbounded-latin-5.woff2')})}
@font-face{font-family:Unbounded;src:url(${url('src/fonts/Unbounded-latin-ext-4.woff2')});unicode-range:U+0100-024F}
@font-face{font-family:Manrope;src:url(${url('src/fonts/Manrope-latin-3.woff2')})}
*{margin:0;box-sizing:border-box}
body{width:var(--w);height:var(--h);overflow:hidden;background:#F56E2E;font-family:Manrope,sans-serif;color:#1C1A1A;position:relative}
.bande{position:absolute;inset:0 auto 0 40%;width:22%;background:#F48A50;opacity:.55}
.fond{position:absolute;color:#F48A50;opacity:.75}
.fond svg{width:100%;height:100%;fill:currentColor}
.marque{position:absolute;display:flex;align-items:center;gap:22px;font-family:Unbounded;font-weight:700;letter-spacing:-.02em}
.marque img{filter:drop-shadow(0 10px 18px rgba(120,40,0,.25))}
.marque em{font-style:normal;color:#F1E8CB}
h1{position:absolute;font-family:Unbounded;font-weight:800;letter-spacing:-.045em;line-height:.98}
.sous{position:absolute;font-weight:600}
.fenetre{position:absolute;background:#141010;border-radius:22px;overflow:hidden;box-shadow:0 40px 90px rgba(70,20,0,.45),0 0 0 2px rgba(0,0,0,.4)}
.barre{height:44px;display:flex;align-items:center;gap:12px;padding:0 20px;color:#bdb2a6;font-size:17px;border-bottom:1px solid #2a2220}
.barre img{width:20px}
.barre span{margin-left:auto;letter-spacing:28px;color:#8a7f75}
.fenetre > img{display:block;width:100%}
`;

function page({ w, h, lang, titreTaille, sousTaille, marqueTaille, fen, sousTop }) {
  const t = TEXTES[lang];
  return `<!doctype html><html lang="${lang}"><head><meta charset="utf-8"><style>:root{--w:${w}px;--h:${h}px}${css}</style></head><body>
  <div class="bande"></div>
  <div class="fond" style="left:${w * 0.52}px;top:${-h * 0.12}px;width:${w * 0.75}px;height:${w * 0.75}px"><svg viewBox="0 0 1024 1024">${DESSIN_BALAI}</svg></div>
  <div class="marque" style="left:${w * 0.06}px;top:${h * 0.075}px;font-size:${marqueTaille}px"><img src="data:image/svg+xml;utf8,${encodeURIComponent(ICONE())}" width="${marqueTaille * 2.3}"><span>kaury<em>.</em>clean</span></div>
  <h1 style="left:${w * 0.06}px;top:${h * 0.2}px;font-size:${titreTaille}px">${t.titre}</h1>
  <p class="sous" style="left:${w * 0.062}px;top:${sousTop ?? fen.top - sousTaille * 3.2}px;font-size:${sousTaille}px">${t.sous}</p>
  <div class="fenetre" style="left:${fen.left}px;top:${fen.top}px;width:${fen.width}px">
    <div class="barre"><img src="data:image/svg+xml;utf8,${encodeURIComponent(ICONE('b'))}">Kaury Clean<span>— ▢ ✕</span></div>
    <img src="${url(`design/screens/${lang === 'en' ? 'en/' : ''}done.jpg`)}">
  </div>
</body></html>`;
}

const nav = await puppeteer.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true, args: ['--allow-file-access-from-files'] });
async function rendre(html, w, h, sortie) {
  const f = path.join(ici, '_tmp.html');
  fs.writeFileSync(f, html);
  const p = await nav.newPage();
  await p.setViewport({ width: w, height: h, deviceScaleFactor: 2 });
  await p.goto(pathToFileURL(f).href, { waitUntil: 'networkidle0' });
  await p.evaluate(() => document.fonts.ready);
  await p.screenshot({ path: path.join(ici, sortie) });
  await p.close();
  fs.rmSync(f);
  console.log('  ' + sortie);
}

for (const lang of ['fr', 'en']) {
  const w = 1616, h = 1264;
  await rendre(page({ w, h, lang, titreTaille: 138, sousTaille: 30, marqueTaille: 40, fen: { left: 96, top: 680, width: 1424 } }), w, h, `couverture-${lang}.png`);
}
// Image de partage (GitHub, réseaux) : 1280 × 640, en anglais.
await rendre(page({ w: 1280, h: 640, lang: 'en', titreTaille: 92, sousTaille: 22, marqueTaille: 28, fen: { left: 640, top: 110, width: 600 }, sousTop: 380 }), 1280, 640, 'preview.png');
await nav.close();
