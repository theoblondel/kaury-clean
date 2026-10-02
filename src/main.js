// Kaury Clean : interface. Tout le travail sur le disque se fait côté Rust (src-tauri).

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : demoInvoke;
let elevated = false;
let appVersion = "";
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

// Langue : celle de Windows au premier lancement (français, sinon anglais), puis ton choix dans À propos.
const LANG_KEY = "kaury-clean-lang";
const LANG = (() => {
  try { const l = localStorage.getItem(LANG_KEY); if (l === "fr" || l === "en") return l; } catch { /* stockage indisponible */ }
  return /^fr\b/i.test(navigator.language || "") ? "fr" : "en";
})();
const EN = LANG === "en";
// macOS : le même code, avec les mots du Mac, et sans les sections qui n'existent que sous Windows.
// Détecté dès le chargement (avant les textes fixes) ; app_info le confirme côté moteur.
const MAC = /Mac/i.test(navigator.platform || navigator.userAgent || "");
const MAC_WORDS = [
  [/\bRecycle Bin\b/g, "Trash"], [/\bl'Explorateur\b/g, "le Finder"], [/\bExplorer\b/g, "Finder"],
  [/\bTon PC\b/g, "Ton Mac"], [/\bton PC\b/g, "ton Mac"], [/\bYour PC\b/g, "Your Mac"], [/\byour PC\b/g, "your Mac"],
  [/\bdu PC\b/g, "du Mac"], [/\bthe PC\b/g, "the Mac"], [/\bWindows\b/g, "macOS"],
  [/Performances et démarrage/g, "Performances et mémoire"], [/Performance and startup/g, "Performance and memory"],
];
const macify = (s) => (MAC && typeof s === "string" ? MAC_WORDS.reduce((t, [re, w]) => t.replace(re, w), s) : s);
const tr = (fr, en) => macify(EN ? en : fr);
const LOCALE = EN ? "en-US" : "fr-CH";
const num = (n) => n.toLocaleString(LOCALE);
document.documentElement.lang = LANG;

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

function fmt(bytes) {
  const units = EN ? ["B", "KB", "MB", "GB", "TB"] : ["o", "Ko", "Mo", "Go", "To"];
  let v = bytes, i = 0;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  const digits = i >= 3 ? 1 : 0;
  const n = v.toFixed(digits);
  return (EN ? n : n.replace(".", ",")) + " " + units[i];
}

function ago(unixSecs) {
  if (!unixSecs) return "";
  const days = (Date.now() / 1000 - unixSecs) / 86400;
  if (days < 1) return tr("aujourd'hui", "today");
  if (days < 30) { const n = Math.round(days); return tr(`il y a ${n} j`, `${n} day${n > 1 ? "s" : ""} ago`); }
  if (days < 365) { const n = Math.round(days / 30); return tr(`il y a ${n} mois`, `${n} month${n > 1 ? "s" : ""} ago`); }
  const years = Math.round(days / 365);
  return tr(`il y a ${years} an${years > 1 ? "s" : ""}`, `${years} year${years > 1 ? "s" : ""} ago`);
}

let toastTimer;
function toast(msg) {
  const t = $("#toast");
  t.textContent = msg;
  t.classList.add("on");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove("on"), 3200);
}

// Le moteur Rust envoie ce qu'il est en train d'analyser : on l'affiche dans l'élément « actif ».
let progressEl = null;
window.__TAURI__?.event.listen("progress", (e) => {
  if (progressEl) progressEl.textContent = e.payload;
});

// Compteur local de l'espace libéré depuis l'installation.
const STATS_KEY = "kaury-clean-stats";
function readStats() {
  try { return JSON.parse(localStorage.getItem(STATS_KEY)) || { freed: 0, last: 0 }; } catch { return { freed: 0, last: 0 }; }
}
function addFreed(bytes) {
  const st = readStats();
  st.freed += bytes; st.last = Date.now() / 1000;
  try { localStorage.setItem(STATS_KEY, JSON.stringify(st)); } catch { /* stockage indisponible : tant pis */ }
  renderStats();
}
function renderStats() {
  const st = readStats();
  $("#stats").hidden = !st.freed;
  if (st.freed) $("#stats").textContent = tr(`${fmt(st.freed)} libérés depuis l'installation · dernier nettoyage ${ago(st.last)}`, `${fmt(st.freed)} freed since install · last cleanup ${ago(st.last)}`);
}

// Un élément bloqué tant que l'appli n'a pas les droits administrateur.
const locked = (item) => item.needs_admin && !elevated;

const sumChecked = (root) => $$("input[type=checkbox]:checked", root).reduce((a, i) => a + Number(i.dataset.bytes || 0), 0);

// ---------- Icônes ----------
// Pictos dessinés en SVG, posés sur les tuiles colorées (barre latérale, accueil, cartes).
const ICONS = {
  sparkle: '<svg viewBox="0 0 24 24"><path d="M12 2.5l2.4 7.1 7.1 2.4-7.1 2.4L12 21.5l-2.4-7.1L2.5 12l7.1-2.4z"/></svg>',
  broom: '<svg viewBox="0 0 24 24" fill="none" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 3l-6.5 6.5"/><path d="M10.5 8.5l5 5-3 3.5c-2 2-5.5 3-9 3 0-3.5 1-7 3-9z"/><path d="M7 16l-1.5 1.5"/></svg>',
  bolt: '<svg viewBox="0 0 24 24"><path d="M13.5 2L4 13.5h6.5L9.5 22 20 9.5h-6.5z"/></svg>',
  grid: '<svg viewBox="0 0 24 24"><rect x="3" y="3" width="8" height="8" rx="2.5"/><rect x="13" y="3" width="8" height="8" rx="2.5"/><rect x="3" y="13" width="8" height="8" rx="2.5"/><rect x="13" y="13" width="8" height="8" rx="4"/></svg>',
  folder: '<svg viewBox="0 0 24 24"><path d="M2.5 6.5A2.5 2.5 0 0 1 5 4h4.2l2.3 2.3H19a2.5 2.5 0 0 1 2.5 2.5v8.7A2.5 2.5 0 0 1 19 20H5a2.5 2.5 0 0 1-2.5-2.5z"/></svg>',
  globe: '<svg viewBox="0 0 24 24" fill="none" stroke-width="2.2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3.2 3 14.8 0 18M12 3c-3 3.2-3 14.8 0 18"/></svg>',
  trash: '<svg viewBox="0 0 24 24"><path d="M9 2.5h6l1 2h4.5v2.5h-17V4.5H8z"/><path d="M5 9h14l-1.1 11.2a2 2 0 0 1-2 1.8H8.1a2 2 0 0 1-2-1.8z"/></svg>',
  wrench: '<svg viewBox="0 0 24 24"><path d="M21 7.2a5.5 5.5 0 0 1-7.4 5.2l-7 7a2.1 2.1 0 0 1-3-3l7-7A5.5 5.5 0 0 1 16.8 3l-3.1 3.1.9 2.4 2.4.9z"/></svg>',
  disk: '<svg viewBox="0 0 24 24"><rect x="2.5" y="5" width="19" height="14" rx="3"/><circle cx="17" cy="12" r="1.6" fill="currentColor" style="fill:var(--tile-ink,#1C1A1A)"/></svg>',
  logo: '<svg class="balai" viewBox="0 0 1024 1024"><g transform="translate(-52.97 -52.97) scale(1.1034)"><g transform="rotate(30 512 512) translate(150 70) scale(1.5) translate(-170 -260)"><rect x="486" y="110" width="52" height="420" rx="26"/><rect x="418" y="500" width="188" height="76" rx="30"/><path fill-rule="evenodd" d="M416 594 L608 594 Q642 594 652 626 L718 824 Q726 864 686 870 Q512 896 338 870 Q298 864 306 824 L372 626 Q382 594 416 594 Z M463.9 641.3 L441.9 843.3 L418.1 840.7 L440.1 638.7 Z M500 640 L524 640 L524 852 L500 852 Z M583.9 638.7 L605.9 840.7 L582.1 843.3 L560.1 641.3 Z"/></g><path transform="translate(280 300) scale(0.9)" d="M0 -100 L22 -22 L100 0 L22 22 L0 100 L-22 22 L-100 0 L-22 -22 Z"/></g></svg>',
  "logo-big": '<svg class="balai" viewBox="0 0 1024 1024"><g transform="translate(-52.97 -52.97) scale(1.1034)"><g transform="rotate(30 512 512) translate(150 70) scale(1.5) translate(-170 -260)"><rect x="486" y="110" width="52" height="420" rx="26"/><rect x="418" y="500" width="188" height="76" rx="30"/><path fill-rule="evenodd" d="M416 594 L608 594 Q642 594 652 626 L718 824 Q726 864 686 870 Q512 896 338 870 Q298 864 306 824 L372 626 Q382 594 416 594 Z M463.9 641.3 L441.9 843.3 L418.1 840.7 L440.1 638.7 Z M500 640 L524 640 L524 852 L500 852 Z M583.9 638.7 L605.9 840.7 L582.1 843.3 L560.1 641.3 Z"/></g><path transform="translate(280 300) scale(0.9)" d="M0 -100 L22 -22 L100 0 L22 22 L0 100 L-22 22 L-100 0 L-22 -22 Z"/></g></svg>',
};
function paintIcons(root = document) {
  $$("[data-icon]", root).forEach((el) => { if (!el.firstChild) el.innerHTML = ICONS[el.dataset.icon] || ""; });
}
paintIcons();

// ---------- Navigation ----------
// Six sections dans la barre latérale ; certaines ont des onglets en haut de page.
const SECTIONS = {
  home: [["home", tr("Entretien intelligent", "Smart care")]],
  clean: [["system", tr("Fichiers système", "System files")], ["apps", "Applications"], ["browsers", tr("Navigateurs", "Browsers")], ["trash", tr("Corbeille", "Recycle Bin")]],
  perf: [["memory", tr("Mémoire vive", "Memory")], ["startup", tr("Démarrage", "Startup")], ["tips", tr("Astuces", "Tips")]],
  apps: [["uninstall", tr("Désinstaller", "Uninstall")]],
  files: [["space", tr("Place du disque", "Disk map")], ["large", tr("Gros fichiers", "Large files")], ["dupes", tr("Doublons", "Duplicates")], ["olddl", tr("Vieux téléchargements", "Old downloads")], ["organize", tr("Ranger", "Tidy up")]],
  repair: [["maintenance", "Maintenance"]],
  about: [["about", tr("À propos", "About")]],
};
// Sur Mac : pas encore de démarrage, de désinstallation ni de réparation (prévus pour une version suivante).
if (MAC) {
  SECTIONS.perf = SECTIONS.perf.filter(([v]) => v !== "startup" && v !== "tips");
  delete SECTIONS.apps;
  delete SECTIONS.repair;
}
const lastTab = {};
const sectionOf = (view) => Object.keys(SECTIONS).find((s) => SECTIONS[s].some(([v]) => v === view));

$$("nav [data-section]").forEach((b) => b.addEventListener("click", () => {
  const sec = b.dataset.section;
  show(lastTab[sec] || SECTIONS[sec][0][0]);
}));

function show(view) {
  const sec = sectionOf(view);
  lastTab[sec] = view;
  $$("nav [data-section]").forEach((b) => b.setAttribute("aria-current", String(b.dataset.section === sec)));
  $$(".view").forEach((s) => (s.hidden = s.id !== "v-" + view));
  const tabs = SECTIONS[sec];
  const bar = $("#tabs");
  bar.hidden = tabs.length < 2;
  bar.innerHTML = tabs.map(([v, label]) => {
    const size = junk && GROUPS[v] && groupTotal(v) > 0 ? `<span class="tab-size">${fmt(groupTotal(v))}</span>` : "";
    return `<button role="tab" aria-selected="${v === view}" data-tab="${v}">${esc(label)}${size}</button>`;
  }).join("");
  $$("[data-tab]", bar).forEach((b) => b.addEventListener("click", () => show(b.dataset.tab)));
  $("main").scrollTop = 0;
  if (view === "startup" && !startupLoaded) loadStartup();
  if (view === "tips" && !tipsLoaded) loadTips();
  viewLoaders[view]?.();
}
// Les modules de modules.js s'inscrivent ici pour se charger à la première ouverture.
const viewLoaders = {};

// ---------- Disque ----------
let diskInfo = null;
async function refreshDisk() {
  try {
    const d = await invoke("disk_info");
    if (!d) return;
    diskInfo = d;
    $("#disk").hidden = false;
    $("#diskName").textContent = tr(`Disque ${d.name}`, `Drive ${d.name}`);
    $("#diskFree").textContent = tr(`${fmt(d.free)} libres`, `${fmt(d.free)} free`);
    $("#diskBar").style.width = (((d.total - d.free) / d.total) * 100).toFixed(1) + "%";
  } catch (e) { console.error(e); }
}

// ---------- Anneau animé (pendant l'analyse) ----------
const cv = $("#orb"), ctx = cv.getContext("2d");
let prog = 0, spin = 0, ringMode = "spin";
function drawOrb() {
  if (!$("#careScan").hidden) {
    const W = cv.width, c = W / 2;
    ctx.clearRect(0, 0, W, W);
    const g = ctx.createRadialGradient(c, c, 20, c, c, c);
    g.addColorStop(0, "rgba(245,110,46,.35)"); g.addColorStop(0.6, "rgba(245,110,46,.08)"); g.addColorStop(1, "rgba(245,110,46,0)");
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(c, c, c, 0, 7); ctx.fill();
    ctx.lineWidth = 14; ctx.lineCap = "round";
    ctx.strokeStyle = "rgba(241,232,203,.08)"; ctx.beginPath(); ctx.arc(c, c, 165, 0, Math.PI * 2); ctx.stroke();
    const gr = ctx.createLinearGradient(0, 0, W, W); gr.addColorStop(0, "#FFB27D"); gr.addColorStop(1, "#F56E2E");
    ctx.strokeStyle = gr; ctx.beginPath();
    if (ringMode === "spin") ctx.arc(c, c, 165, spin, spin + 1.1);
    else ctx.arc(c, c, 165, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * prog);
    ctx.stroke();
    for (let i = 0; i < 48; i++) {
      const a = (i / 48) * Math.PI * 2 + spin * 0.3;
      ctx.fillStyle = "rgba(241,232,203,.14)";
      ctx.beginPath(); ctx.arc(c + Math.cos(a) * 192, c + Math.sin(a) * 192, 2.5, 0, 7); ctx.fill();
    }
    if (!reduceMotion) spin += 0.07;
  }
  requestAnimationFrame(drawOrb);
}
drawOrb();
const orbVal = (big, small) => ($("#orbVal").innerHTML = `${esc(big)}<small>${esc(small)}</small>`);

// ---------- Entretien intelligent ----------
// Une analyse regarde tout : fichiers inutiles, mémoire, démarrage, applis et vieux téléchargements.
// « Lancer » nettoie ce qui est sûr et vide le cache DNS ; le reste est proposé à l'examen.
let junk = null; // dernier résultat de scan_junk
const GROUPS = { system: tr("Fichiers système", "System files"), apps: "Applications", browsers: tr("Navigateurs", "Browsers"), trash: tr("Corbeille", "Recycle Bin") };
let care = { state: "idle" };

const groupTotal = (group) => (junk || []).filter((i) => i.group === group && !locked(i)).reduce((a, i) => a + i.bytes, 0);

function careView(state) {
  care.state = state;
  $("#careIntro").hidden = state !== "idle";
  $("#careScan").hidden = !["scanning", "running"].includes(state);
  $("#careScan").classList.toggle("running", state === "running");
  $("#careResults").hidden = !["results", "done"].includes(state);
  $("#careReset").hidden = state !== "results";
  const btn = $("#careBtn");
  btn.textContent = EN
    ? { idle: "Scan", scanning: "Scanning", results: "Run", running: "Running", done: "Done" }[state]
    : { idle: "Analyser", scanning: "Analyse", results: "Lancer", running: "En cours", done: "Terminé" }[state];
  btn.disabled = state === "scanning" || state === "running";
  btn.classList.toggle("busy", btn.disabled);
}

$("#careBtn").addEventListener("click", () => {
  if (care.state === "idle") runCare();
  else if (care.state === "results") runCareActions();
  else if (care.state === "done") { careView("idle"); renderStats(); }
});
$("#careReset").addEventListener("click", () => { careView("idle"); renderStats(); });

async function runCare() {
  careView("scanning");
  ringMode = "spin";
  $("#scanTitle").textContent = tr("Analyse en cours", "Scanning");
  orbVal("…", tr("analyse", "scan"));
  progressEl = $("#careProgress");
  progressEl.textContent = tr("Préparation", "Getting ready");
  const soft = (p) => p.catch(() => null);
  try {
    const [j, mem, startup, installed, oldFiles] = await Promise.all([
      invoke("scan_junk"),
      soft(invoke("memory_status")),
      MAC ? null : soft(invoke("list_startup_apps")),
      MAC ? null : soft(invoke("list_installed_apps")),
      soft(invoke("find_old_downloads", { minDays: 180 })),
    ]);
    junk = j;
    care = { ...care, mem, startup, installed, oldFiles, includeTrash: true };
  } catch (e) {
    progressEl = null;
    careView("idle");
    toast(tr("L'analyse n'a pas pu aboutir : ", "The scan couldn't finish: ") + e);
    return;
  }
  progressEl = null;
  renderJunkEverywhere();
  await refreshDisk();
  careView("results");
  renderBento();
}

// Ce que « Lancer » va nettoyer : tout ce qui est sûr et accessible, corbeille selon ton choix.
function careIds() {
  return (junk || []).filter((i) => i.bytes > 0 && !locked(i) && (care.includeTrash || i.group !== "trash")).map((i) => i.id);
}
const careBytes = () => (junk || []).filter((i) => careIds().includes(i.id)).reduce((a, i) => a + i.bytes, 0);

function card({ cls = "", kicker, icon, tile, big, sub, status, action, extra = "" }) {
  return `<div class="bcard ${cls}">
    <span class="bcard-art tile ${tile}" data-icon="${icon}"></span>
    <div class="kicker">${esc(kicker)}</div>
    <div class="bcard-body"><div class="big">${big}</div><div class="sub">${sub}</div>${extra}</div>
    <div class="bcard-foot">
      ${status ? `<span class="status ${status.kind}">${status.kind === "ok" ? "✓" : "•"} ${esc(status.text)}</span>` : "<span></span>"}
      ${action ? `<button class="chip-btn" data-go="${action.go}">${esc(action.label)}</button>` : ""}
    </div>
  </div>`;
}

function renderBento() {
  const done = care.state === "done";
  const r = care.report;
  const startupOn = care.startup ? care.startup.filter((a) => a.enabled).length : null;
  const memPct = care.mem ? Math.round((care.mem.used / care.mem.total) * 100) : null;
  const appsBytes = care.installed ? care.installed.reduce((a, x) => a + x.bytes, 0) : 0;
  const old = care.oldFiles || [];
  const oldBytes = old.reduce((a, f) => a + f.bytes, 0);
  const trash = (junk || []).find((i) => i.group === "trash");
  const adminBytes = (junk || []).filter(locked).reduce((a, i) => a + i.bytes, 0);
  const running = (junk || []).filter((i) => i.running && i.bytes > 0).map((i) => i.running);

  $("#careTitle").textContent = done ? tr("Bravo ! Ton PC est en pleine forme.", "Well done! Your PC is in great shape.") : tr("Voilà ce qu'on a trouvé.", "Here's what we found.");
  $("#careLead").textContent = done
    ? (r.skipped
      ? tr(`${num(r.skipped)} fichiers gardés : une appli les utilise ou ils ont moins de 24 h.`, `${num(r.skipped)} files kept: an app is using them or they're less than 24 h old.`)
      : tr("Tout ce qui pouvait partir est parti.", "Everything that could go is gone."))
    : tr("Clique sur Lancer pour nettoyer. Le reste t'attend dans chaque section.", "Click Run to clean up. Everything else waits in each section.");

  const cleanExtra = done ? "" : `
    ${trash && trash.bytes > 0 ? `<label class="mini-check"><input type="checkbox" id="careTrash" ${care.includeTrash ? "checked" : ""}> ${tr("Vider aussi la corbeille", "Also empty the Recycle Bin")} (${fmt(trash.bytes)})</label>` : ""}
    ${running.length ? `<div class="mini-note">${esc([...new Set(running)].join(", "))} ${MAC
      ? (new Set(running).size > 1 ? tr("ouverts : ils ne seront pas nettoyés", "are open: they won't be cleaned") : tr("ouvert : il ne sera pas nettoyé", "is open: it won't be cleaned"))
      : (new Set(running).size > 1 ? tr("ouverts : une partie restera", "are open: some will stay") : tr("ouvert : une partie restera", "is open: some will stay"))}</div>` : ""}
    ${adminBytes > 0 ? `<button class="link mini" data-admin>+ ${fmt(adminBytes)} ${tr("avec les droits administrateur", "with admin rights")}</button>` : ""}`;

  const cards = [
    card({ cls: "wide", kicker: tr("Nettoyage", "Cleanup"), icon: "broom", tile: "t-clean",
      big: done ? fmt(r.freed) : fmt(careBytes()), sub: done ? tr("libérés", "freed") : tr("de fichiers inutiles", "of junk files"),
      status: done ? { kind: "ok", text: tr("Nettoyé", "Cleaned") } : null, action: done ? null : { go: "system", label: tr("Détails", "Details") }, extra: cleanExtra }),
    MAC ? card({ kicker: tr("Performances", "Performance"), icon: "bolt", tile: "t-perf",
      big: memPct != null ? `${memPct}${EN ? "" : " "}%` : "—", sub: tr("de mémoire utilisée", "of memory in use"),
      action: { go: "memory", label: tr("Voir", "View") } }) :
    card({ kicker: tr("Performances", "Performance"), icon: "bolt", tile: "t-perf",
      big: startupOn != null ? `${startupOn} app${EN ? "" : "li"}${startupOn > 1 ? "s" : ""}` : "—",
      sub: `${tr("au démarrage", "at startup")}${memPct != null ? ` · ${tr("mémoire", "memory")} ${memPct}${EN ? "" : " "}%` : ""}`,
      status: done ? (care.dns ? { kind: "ok", text: tr("Cache DNS vidé", "DNS cache flushed") } : null) : null, action: { go: "startup", label: tr("Voir", "View") } }),
    card({ kicker: tr("Espace disque", "Disk space"), icon: "disk", tile: "t-repair",
      big: diskInfo ? fmt(diskInfo.free) : "—", sub: diskInfo ? tr(`libres sur ${fmt(diskInfo.total)}`, `free of ${fmt(diskInfo.total)}`) : "",
      status: done ? { kind: "ok", text: tr("Mis à jour", "Updated") } : null,
      extra: diskInfo ? `<div class="bar"><i style="width:${(((diskInfo.total - diskInfo.free) / diskInfo.total) * 100).toFixed(1)}%"></i></div>` : "" }),
    MAC ? "" : card({ cls: "half", kicker: "Applications", icon: "grid", tile: "t-apps",
      big: care.installed ? tr(`${care.installed.length} applis`, `${care.installed.length} apps`) : "—",
      sub: appsBytes ? tr(`${fmt(appsBytes)} au total`, `${fmt(appsBytes)} in total`) : tr("installées", "installed"),
      status: done ? { kind: "ok", text: tr("Vérifiées", "Checked") } : null, action: { go: "uninstall", label: tr("Examiner", "Review") } }),
    card({ cls: "half", kicker: tr("Mes fichiers", "My files"), icon: "folder", tile: "t-files",
      big: `${old.length} ${tr("fichier", "file")}${old.length > 1 ? "s" : ""}`,
      sub: old.length ? tr(`oubliés dans Téléchargements · ${fmt(oldBytes)}`, `forgotten in Downloads · ${fmt(oldBytes)}`) : tr("rien d'oublié dans Téléchargements", "nothing forgotten in Downloads"),
      status: done && old.length ? { kind: "todo", text: tr("À examiner", "To review") } : null, action: { go: "olddl", label: tr("Examiner", "Review") } }),
  ];
  const box = $("#bento");
  box.innerHTML = cards.join("");
  paintIcons(box);
  $$("[data-go]", box).forEach((b) => b.addEventListener("click", () => {
    show(b.dataset.go);
    if (b.dataset.go === "olddl" && care.oldFiles) showOldDownloads(care.oldFiles);
  }));
  $("#careTrash")?.addEventListener("change", (e) => { care.includeTrash = e.target.checked; renderBento(); });
  $("[data-admin]", box)?.addEventListener("click", relaunchAsAdmin);
}

// Le ménage se fait section par section : chaque ligne passe de « en attente » à la roue, puis à la coche.
const STEP_LOOK = {
  system: { icon: "disk", tile: "t-repair" },
  apps: { icon: "grid", tile: "t-apps" },
  browsers: { icon: "globe", tile: "t-clean" },
  trash: { icon: "trash", tile: "t-files" },
  dns: { icon: "bolt", tile: "t-perf" },
};

async function runCareActions() {
  const ids = careIds();
  const steps = Object.keys(GROUPS)
    .map((g) => ({ key: g, label: GROUPS[g], ids: junk.filter((i) => i.group === g && ids.includes(i.id)).map((i) => i.id) }))
    .filter((st) => st.ids.length);
  if (!MAC) steps.push({ key: "dns", label: tr("Cache DNS", "DNS cache"), ids: [] });

  careView("running");
  $("#scanTitle").textContent = tr("Nettoyage en cours", "Cleaning");
  const total = careBytes();
  let freed = 0;
  ringMode = "progress"; prog = 0;
  orbVal(fmt(0), tr(`sur ${fmt(total)}`, `of ${fmt(total)}`));
  const list = $("#careSteps");
  list.hidden = false;
  list.innerHTML = steps.map((st) => `<li data-step="${st.key}" class="wait">
      <span class="tile sm ${STEP_LOOK[st.key].tile}" data-icon="${STEP_LOOK[st.key].icon}"></span>
      <span class="step-name">${esc(st.label)}</span><span class="step-state">${tr("En attente", "Waiting")}</span></li>`).join("");
  paintIcons(list);
  progressEl = $("#careProgress");

  const report = { freed: 0, removed: 0, skipped: 0 };
  let dns = false;
  for (const [n, st] of steps.entries()) {
    const li = $(`[data-step="${st.key}"]`, list);
    li.className = "doing";
    $("#careProgress").textContent = st.label;
    $(".step-state", li).innerHTML = `<span class="spinner small"></span>`;
    try {
      if (st.key === "dns") {
        dns = await invoke("run_maintenance", { id: "flush_dns" }).then((x) => x.ok);
        $(".step-state", li).textContent = dns ? tr("✓ Vidé", "✓ Flushed") : tr("Pas disponible", "Not available");
      } else {
        const r = await invoke("clean_junk", { ids: st.ids });
        report.freed += r.freed; report.removed += r.removed; report.skipped += r.skipped;
        freed += r.freed;
        $(".step-state", li).textContent = `✓ ${fmt(r.freed)}`;
      }
      li.className = "done";
    } catch (e) {
      li.className = "fail";
      $(".step-state", li).textContent = tr("Échec", "Failed");
      console.error(e);
    }
    prog = (n + 1) / steps.length;
    orbVal(fmt(freed), tr(`sur ${fmt(total)}`, `of ${fmt(total)}`));
  }
  progressEl = null;
  await new Promise((r) => setTimeout(r, 700)); // le temps de voir la dernière coche
  list.hidden = true;
  care = { ...care, report, dns };
  addFreed(report.freed);
  await refreshDisk();
  careView("done");
  renderBento();
  // Les chiffres des sections se remettent à jour en arrière-plan.
  invoke("scan_junk").then((j) => { junk = j; renderJunkEverywhere(); }).catch(() => {});
}

// ---------- Sections Nettoyage (4 onglets) ----------
function renderJunkEverywhere() {
  const total = Object.keys(GROUPS).reduce((a, g) => a + groupTotal(g), 0);
  $('[data-size="clean"]').textContent = junk && total ? fmt(total) : "";
  for (const g of Object.keys(GROUPS)) renderGroupView(g);
  const current = $$(".view").find((v) => !v.hidden);
  if (current && GROUPS[current.id.slice(2)]) show(current.id.slice(2)); // rafraîchit les tailles des onglets
}

function renderGroupView(g) {
  const el = $("#v-" + g);
  const head = `<div class="head"><div><h2>${esc(tr(el.dataset.title, el.dataset.enHeading))}</h2><p>${esc(tr(el.dataset.desc, el.dataset.enDesc))}</p></div></div>`;
  if (!junk) {
    el.innerHTML = head + `<div class="empty">${tr("Lance une analyse pour voir ce qui peut partir.", "Run a scan to see what can go.")}<button class="cta small" data-scan>${tr("Analyser", "Scan")}</button></div>`;
    $("[data-scan]", el).addEventListener("click", scanJunkOnly);
    return;
  }
  const items = junk.filter((i) => i.group === g);
  if (!items.length) {
    el.innerHTML = head + `<div class="empty">${tr("Rien à nettoyer ici.", "Nothing to clean here.")}</div>`;
    return;
  }
  el.innerHTML = head + `<div class="list">${items.map((it) => `
    <label class="item"><input type="checkbox" value="${esc(it.id)}" data-bytes="${it.bytes}" ${it.bytes > 0 && !locked(it) ? "checked" : ""} ${it.bytes > 0 ? "" : "disabled"}>
      <span class="txt"><div class="n">${esc(it.name)}${locked(it) ? `<span class="chip">admin</span>` : ""}${it.running ? `<span class="chip warn">${esc(it.running)} ${tr("ouvert : ferme-le pour tout nettoyer", "is open: close it to clean everything")}</span>` : ""}</div>
      <div class="p">${esc(it.detail)} · ${num(it.files)} ${tr("fichiers", "files")}</div></span>
      <span class="s">${fmt(it.bytes)}</span></label>`).join("")}</div>
    ${items.some(locked) ? `<div class="admin-note left">${tr("Les éléments « admin » sont dans des dossiers protégés : sans les droits administrateur, Windows empêche presque tout de partir.", "\"admin\" items are in protected folders: without admin rights, Windows stops almost everything from going.")}<button class="link" data-admin>${tr("Relancer en administrateur", "Relaunch as administrator")}</button></div>` : ""}
    <div class="foot"><span>${tr("Sélection :", "Selected:")} <b class="sel"></b></span><button class="cta small" data-clean>${g === "trash" ? tr("Vider la corbeille", "Empty the Recycle Bin") : tr("Nettoyer", "Clean")}</button></div>`;
  const update = () => {
    const bytes = sumChecked(el);
    $(".sel", el).textContent = fmt(bytes);
    $("[data-clean]", el).disabled = bytes === 0;
  };
  $$("input", el).forEach((i) => i.addEventListener("change", update));
  $("[data-clean]", el).addEventListener("click", (e) => cleanFromSection(e.currentTarget, $$("input:checked", el).map((i) => i.value)));
  $("[data-admin]", el)?.addEventListener("click", relaunchAsAdmin);
  update();
}

async function scanJunkOnly() {
  $$(".view:not([hidden]) [data-scan]").forEach((b) => { b.disabled = true; b.textContent = tr("Analyse…", "Scanning…"); });
  try {
    junk = await invoke("scan_junk");
  } catch (e) {
    toast(tr("L'analyse a échoué : ", "The scan failed: ") + e);
  }
  renderJunkEverywhere();
}

// Nettoyage depuis un onglet : on reste sur place, puis on réanalyse pour mettre les chiffres à jour.
async function cleanFromSection(btn, ids) {
  if (!ids.length) return;
  btn.disabled = true;
  btn.textContent = tr("Nettoyage…", "Cleaning…");
  try {
    const r = await invoke("clean_junk", { ids });
    addFreed(r.freed);
    toast(tr(`${fmt(r.freed)} libérés`, `${fmt(r.freed)} freed`)
      + (r.skipped ? tr(` · ${num(r.skipped)} fichiers gardés (utilisés ou trop récents)`, ` · ${num(r.skipped)} files kept (in use or too recent)`) : ""));
  } catch (e) {
    toast(tr("Le nettoyage a échoué : ", "The cleanup failed: ") + e);
  }
  refreshDisk();
  await scanJunkOnly();
}

// Affiche « recherche en cours » avec un bouton Arrêter. Renvoie une fonction d'affichage d'erreur.
function startSearch(box, label) {
  box.innerHTML = `<div class="empty"><div class="spinner"></div>${esc(label)}<span class="progress"></span><button class="cta ghost" data-stop>${tr("Arrêter", "Stop")}</button></div>`;
  progressEl = $(".progress", box);
  $("[data-stop]", box).addEventListener("click", (e) => {
    e.currentTarget.disabled = true;
    invoke("cancel_search");
  });
}
function searchFailed(box, e) {
  progressEl = null;
  box.innerHTML = /arrêtée|stopped/.test(String(e))
    ? `<div class="empty">${tr("Recherche arrêtée.", "Search stopped.")}</div>`
    : `<div class="empty">${tr("La recherche a échoué : ", "The search failed: ")}${esc(e)}</div>`;
}

// ---------- Gros fichiers ----------
$("#largeBtn").addEventListener("click", loadLarge);
async function loadLarge() {
  const box = $("#largeList");
  startSearch(box, tr("Recherche dans tes dossiers…", "Searching your folders…"));
  $("#largeBtn").disabled = true;
  try {
    const files = await invoke("find_large_files", { minMb: Number($("#largeMin").value) });
    progressEl = null;
    renderFileList(box, files.length ? [{ files }] : [], { preselect: false, onDone: loadLarge });
  } catch (e) {
    searchFailed(box, e);
  }
  $("#largeBtn").disabled = false;
}

// ---------- Doublons ----------
$("#dupesBtn").addEventListener("click", loadDupes);
async function loadDupes() {
  const box = $("#dupesList");
  startSearch(box, tr("Comparaison des fichiers… Ça peut prendre une minute.", "Comparing files… This can take a minute."));
  $("#dupesBtn").disabled = true;
  try {
    const groups = await invoke("find_duplicates");
    progressEl = null;
    renderFileList(box, groups, { preselect: true, onDone: loadDupes });
  } catch (e) {
    searchFailed(box, e);
  }
  $("#dupesBtn").disabled = false;
}

// Liste de fichiers avec cases à cocher et bouton « corbeille ». Pour les doublons, chaque groupe
// garde sa première copie (la plus récente) décochée.
function renderFileList(box, groups, { preselect, onDone }) {
  if (!groups.length) {
    box.innerHTML = `<div class="empty">${tr("Rien trouvé. Bonne nouvelle.", "Nothing found. Good news.")}</div>`;
    return;
  }
  const isDupes = preselect;
  box.innerHTML = `<div class="list">${groups.map((grp, g) => `
    ${isDupes ? `<div class="group-title"><b>${esc(grp.files[0].name)}</b><span>${grp.files.length} copies · ${fmt(grp.bytes)} ${tr("chacune", "each")}</span></div>` : ""}
    ${grp.files.map((f, i) => `
      <label class="item"><input type="checkbox" value="${esc(f.path)}" data-bytes="${f.bytes}" data-group="${g}" ${isDupes && i > 0 ? "checked" : ""}>
        <span class="txt"><div class="n">${esc(isDupes ? f.folder : f.name)}${isDupes && i === 0 ? `<span class="chip ok">${tr("la plus récente", "most recent")}</span>` : ""}<span class="chip ${!isDupes && f.bytes > 4 * 1024 ** 3 ? "high" : ""}">${esc(ago(f.modified))}</span></div>
        <div class="p">${esc(isDupes ? f.name : f.folder)}</div></span>
        <span class="end"><button class="reveal" data-reveal="${esc(f.path)}" title="${tr("Afficher dans l'Explorateur", "Show in Explorer")}">${tr("Afficher", "Show")}</button><span class="s">${fmt(f.bytes)}</span></span></label>`).join("")}`).join("")}</div>
    <div class="foot"><span>${tr("Sélection :", "Selected:")} <b class="sel"></b><span class="warn-text" hidden></span></span><button class="cta small" data-trash>${tr("Mettre à la corbeille", "Move to Recycle Bin")}</button></div>`;
  const update = () => {
    const bytes = sumChecked(box);
    $(".sel", box).textContent = fmt(bytes);
    // Doublons : on refuse de supprimer toutes les copies d'un même fichier.
    const allGone = isDupes && groups.some((grp, g) => $$(`input[data-group="${g}"]:not(:checked)`, box).length === 0);
    const warn = $(".warn-text", box);
    warn.hidden = !allGone;
    warn.textContent = tr(" · garde au moins une copie de chaque fichier", " · keep at least one copy of each file");
    $("[data-trash]", box).disabled = bytes === 0 || allGone;
  };
  $$("input", box).forEach((i) => i.addEventListener("change", update));
  $$("[data-reveal]", box).forEach((b) => b.addEventListener("click", (e) => {
    e.preventDefault(); // le bouton est dans un <label> : sans ça, il cocherait la case
    invoke("reveal_file", { path: b.dataset.reveal }).catch((err) => toast(String(err)));
  }));
  update();
  $("[data-trash]", box).addEventListener("click", async () => {
    const paths = $$("input:checked", box).map((i) => i.value);
    $("[data-trash]", box).disabled = true;
    try {
      const r = await invoke("move_to_trash", { paths });
      toast(tr(`${r.moved} fichier${r.moved > 1 ? "s" : ""} à la corbeille (${fmt(r.bytes)}). Vide la corbeille pour récupérer la place.`,
        `${r.moved} file${r.moved > 1 ? "s" : ""} moved to the Recycle Bin (${fmt(r.bytes)}). Empty the Recycle Bin to get the space back.`)
        + (r.errors.length ? tr(` ${r.errors.length} impossible(s).`, ` ${r.errors.length} couldn't be moved.`) : ""));
      if (r.errors.length) console.warn(r.errors);
    } catch (e) {
      toast(tr("Impossible de déplacer les fichiers : ", "Couldn't move the files: ") + e);
    }
    junk = null; renderJunkEverywhere(); refreshDisk();
    onDone();
  });
}

// ---------- Démarrage ----------
let startupLoaded = false;
async function loadStartup() {
  startupLoaded = true;
  const box = $("#startupList");
  box.innerHTML = `<div class="empty"><div class="spinner"></div></div>`;
  let apps;
  try {
    apps = await invoke("list_startup_apps");
  } catch (e) {
    box.innerHTML = `<div class="empty">${tr("Impossible de lire la liste : ", "Couldn't read the list: ")}${esc(e)}</div>`;
    return;
  }
  if (!apps.length) {
    box.innerHTML = `<div class="empty">${tr("Aucune appli ne se lance au démarrage.", "No app launches at startup.")}</div>`;
    return;
  }
  const lockedApp = (a) => a.scope === "machine" && !elevated;
  box.innerHTML = `${apps.some(lockedApp) ? `<div class="admin-note left above">${tr("Les applis « tous les comptes » se modifient avec les droits administrateur.", "\"All accounts\" apps can only be changed with admin rights.")}<button class="link" data-admin>${tr("Relancer en administrateur", "Relaunch as administrator")}</button></div>` : ""}
    <div class="list">${apps.map((a) => `
    <div class="item"><span class="ic">${esc(a.name.trim()[0] || "?")}</span>
      <span class="txt"><div class="n">${esc(a.name)}${a.scope === "machine" ? `<span class="chip">${tr("tous les comptes · admin", "all accounts · admin")}</span>` : ""}</div>
      <div class="p" title="${esc(a.command)}">${esc(a.command)}</div></span>
      <button class="toggle" role="switch" data-id="${esc(a.id)}" aria-label="${esc(a.name)} ${tr("au démarrage", "at startup")}" aria-checked="${a.enabled}" ${lockedApp(a) ? "disabled" : ""}></button></div>`).join("")}</div>
    <div class="foot"><span>${tr("Activées :", "Enabled:")} <b class="sel"></b></span></div>`;
  const update = () => ($(".sel", box).textContent = `${$$('[aria-checked="true"]', box).length} ${tr("sur", "of")} ${apps.length}`);
  update();
  $("[data-admin]", box)?.addEventListener("click", relaunchAsAdmin);
  $$(".toggle", box).forEach((t) => t.addEventListener("click", async () => {
    const enabled = t.getAttribute("aria-checked") !== "true";
    t.disabled = true;
    try {
      await invoke("set_startup_app", { id: t.dataset.id, enabled });
      t.setAttribute("aria-checked", String(enabled));
      update();
    } catch (e) {
      toast(String(e));
    }
    t.disabled = false;
  }));
}

// ---------- Astuces ----------
// De petits réglages de Windows, chacun réversible : le moteur note la valeur d'origine avant d'écrire.
const TIP_GROUPS = [
  ["speed", tr("Plus rapide", "Faster")],
  ["games", tr("Jeux", "Games")],
  ["calm", tr("Plus calme", "Quieter")],
  ["handy", tr("Pratique", "Handy")],
  ["privacy", tr("Vie privée", "Privacy")],
];
let tipsLoaded = false;
const tipsPending = { explorer: false, signout: false };
async function loadTips() {
  tipsLoaded = true;
  const box = $("#tipsBody");
  box.innerHTML = `<div class="empty"><div class="spinner"></div></div>`;
  let tips;
  try {
    tips = await invoke("list_tweaks");
  } catch (e) {
    box.innerHTML = `<div class="empty">${tr("Impossible de lire les réglages : ", "Couldn't read the settings: ")}${esc(e)}</div>`;
    return;
  }
  const locked = (t) => t.needs_admin && !elevated;
  const when = (t) => t.after === "signout" ? tr("à la prochaine session", "next sign-in") : t.after === "explorer" ? tr("après redémarrage de l'Explorateur", "after restarting Explorer") : "";
  box.innerHTML = `${tips.some(locked) ? `<div class="admin-note left above">${tr("Certains réglages se modifient avec les droits administrateur.", "Some settings can only be changed with admin rights.")}<button class="link" data-admin>${tr("Relancer en administrateur", "Relaunch as administrator")}</button></div>` : ""}
    <div class="tips-pending" id="tipsPending" hidden></div>
    ${TIP_GROUPS.map(([g, label]) => {
      const rows = tips.filter((t) => t.group === g);
      if (!rows.length) return "";
      return `<div class="list tips"><div class="group-title"><b>${esc(label)}</b><span>${rows.filter((t) => t.enabled).length} / ${rows.length}</span></div>${rows.map((t) => `
        <div class="item"><span class="ic" data-icon="${g === "games" ? "bolt" : g === "privacy" ? "logo" : "sparkle"}"></span>
          <span class="txt"><div class="n">${esc(t.name)}${t.needs_admin ? `<span class="chip">admin</span>` : ""}${when(t) ? `<span class="chip">${esc(when(t))}</span>` : ""}</div>
          <div class="d">${esc(t.detail)}</div></span>
          <button class="toggle" role="switch" data-id="${esc(t.id)}" data-after="${esc(t.after)}" aria-label="${esc(t.name)}" aria-checked="${t.enabled}" ${locked(t) ? "disabled" : ""}></button></div>`).join("")}</div>`;
    }).join("")}
    <p class="tips-foot">${tr("Chaque réglage se désactive d'un clic : Kaury Clean remet exactement la valeur qu'il y avait avant. Aucun service de Windows n'est coupé, rien n'est supprimé.", "Every setting turns off in one click: Kaury Clean puts back exactly the value that was there before. No Windows service is turned off, nothing is deleted.")}</p>`;
  paintIcons(box);
  $("[data-admin]", box)?.addEventListener("click", relaunchAsAdmin);
  $$(".toggle", box).forEach((t) => t.addEventListener("click", async () => {
    const enabled = t.getAttribute("aria-checked") !== "true";
    t.disabled = true;
    try {
      await invoke("set_tweak", { id: t.dataset.id, enabled });
      t.setAttribute("aria-checked", String(enabled));
      const list = t.closest(".list");
      $(".group-title span", list).textContent = `${$$('[aria-checked="true"]', list).length} / ${$$(".toggle", list).length}`;
      if (t.dataset.after) tipsPending[t.dataset.after] = true;
      renderTipsPending();
    } catch (e) {
      toast(String(e));
    }
    t.disabled = false;
  }));
}
function renderTipsPending() {
  const bar = $("#tipsPending");
  if (!bar) return;
  const { explorer, signout } = tipsPending;
  bar.hidden = !explorer && !signout;
  bar.innerHTML = `${explorer ? `<span>${tr("Certains changements s'appliquent après le redémarrage de l'Explorateur.", "Some changes apply after restarting Explorer.")}</span><button class="cta small" data-restart>${tr("Redémarrer l'Explorateur", "Restart Explorer")}</button>` : ""}
    ${signout ? `<span>${tr("D'autres s'appliquent à ta prochaine session (déconnexion ou redémarrage du PC).", "Others apply at your next sign-in (sign out or restart the PC).")}</span>` : ""}`;
  $("[data-restart]", bar)?.addEventListener("click", async (e) => {
    e.currentTarget.disabled = true;
    try {
      await invoke("run_maintenance", { id: "restart_explorer" });
      tipsPending.explorer = false;
      renderTipsPending();
      toast(tr("Explorateur redémarré.", "Explorer restarted."));
    } catch (err) {
      toast(String(err));
      e.currentTarget.disabled = false;
    }
  });
}

// ---------- Droits administrateur ----------
async function relaunchAsAdmin() {
  try {
    await invoke("relaunch_as_admin");
  } catch (e) {
    toast(tr("Relance annulée. Tu peux continuer sans les droits administrateur.", "Relaunch cancelled. You can keep going without admin rights."));
  }
}

// F5, Ctrl+R, Ctrl+Maj+I… : recharger la page remettrait l'appli à zéro en plein nettoyage.
document.addEventListener("keydown", (e) => {
  const k = e.key.toLowerCase();
  if (k === "f5" || (e.ctrlKey && (k === "r" || k === "p" || (e.shiftKey && (k === "i" || k === "j" || k === "c"))))) e.preventDefault();
});

// Menu clic droit du navigateur (Recharger, Inspecter…) : inutile dans une appli.
document.addEventListener("contextmenu", (e) => {
  if (!e.target.closest(".p")) e.preventDefault();
});

// ---------- Démarrage de l'interface ----------
(async () => {
  translateStatic();
  if (MAC) {
    macifyStatic();
    $$('[data-section="apps"], [data-section="repair"]').forEach((b) => (b.hidden = true));
  }
  // Le moteur doit connaître la langue avant ses premiers textes (analyse, erreurs, tâches).
  try { await invoke("set_language", { lang: LANG }); } catch (e) { console.error(e); }
  try {
    const info = await invoke("app_info");
    elevated = info.elevated;
    appVersion = info.version;
    $("#version").textContent = `v${info.version}${elevated ? " · admin" : ""}`;
  } catch (e) { console.error(e); }
  renderJunkEverywhere();
  renderStats();
  refreshDisk();
  careView("idle");
  checkUpdateQuietly();
})();

function macifyStatic() {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) n.textContent = macify(n.textContent);
  for (const attr of ["title", "placeholder", "aria-label"]) $$(`[${attr}]`).forEach((el) => el.setAttribute(attr, macify(el.getAttribute(attr))));
}

// Textes fixes de index.html : chaque élément porte sa version anglaise dans data-en
// (et data-en-title, data-en-placeholder… pour les attributs).
function translateStatic() {
  if (!EN) return;
  $$("[data-en]").forEach((el) => {
    const text = [...el.childNodes].find((n) => n.nodeType === Node.TEXT_NODE && n.textContent.trim());
    if (text) text.textContent = text.textContent.replace(text.textContent.trim(), el.dataset.en);
    else el.textContent = el.dataset.en;
  });
  for (const attr of ["title", "placeholder", "aria-label"]) {
    const key = "en" + attr.replace(/(^|-)(\w)/g, (_, _d, c) => c.toUpperCase());
    $$(`[data-en-${attr}]`).forEach((el) => el.setAttribute(attr, el.dataset[key]));
  }
}

// ---------- Mode démo (ouverture dans un navigateur, sans Tauri) ----------
const demoState = { undo: false };
const DEMO_JUNK = (() => { const GB = 1024 ** 3, MB = 1024 ** 2; return [
      { id: "user_temp", group: "system", name: tr("Fichiers temporaires", "Temporary files"), detail: tr("Dossier Temp de ton compte", "Your account's Temp folder"), bytes: 3.3 * GB, files: 18422, running: null },
      { id: "windows_update", group: "system", name: tr("Téléchargements Windows Update", "Windows Update downloads"), detail: tr("Mises à jour déjà installées", "Updates already installed"), bytes: 2.1 * GB, files: 311, running: null, needs_admin: true },
      { id: "crash_reports", group: "system", name: tr("Rapports d'erreur", "Error reports"), detail: tr("Rapports de plantage et fichiers dump", "Crash reports and dump files"), bytes: 268 * MB, files: 47, running: null },
      { id: "adobe_media_cache", group: "apps", name: tr("Cache média Adobe", "Adobe media cache"), detail: tr("Premiere Pro et After Effects", "Premiere Pro and After Effects"), bytes: 1.8 * GB, files: 902, running: null },
      { id: "spotify", group: "apps", name: "Spotify", detail: tr("Musique mise en cache : elle se retélécharge quand tu l'écoutes", "Cached music: it downloads again when you play it"), bytes: 3.4 * GB, files: 812, running: "Spotify" },
      { id: "discord", group: "apps", name: "Discord", detail: tr("Images et vidéos déjà vues", "Images and videos already seen"), bytes: 640 * MB, files: 4211, running: null },
      { id: "other_apps", group: "apps", name: tr("Autres applis", "Other apps"), detail: tr("Caches des applis web installées (Claude, Outlook, Riot, launchers…) : recréés à l'ouverture, tes données ne bougent pas", "Caches of installed web apps (Claude, Outlook, Riot, launchers…): rebuilt on launch, your data stays put"), bytes: 2.4 * GB, files: 9120, running: null },
      { id: "chrome", group: "browsers", name: "Google Chrome", detail: tr("Cache uniquement : mots de passe, favoris et sessions ne bougent pas", "Cache only: passwords, bookmarks and sessions stay put"), bytes: 1.2 * GB, files: 6230, running: "Chrome" },
      { id: "edge", group: "browsers", name: "Microsoft Edge", detail: tr("Cache uniquement : mots de passe, favoris et sessions ne bougent pas", "Cache only: passwords, bookmarks and sessions stay put"), bytes: 486 * MB, files: 2104, running: null },
      { id: "recycle_bin", group: "trash", name: tr("Corbeille", "Recycle Bin"), detail: tr("Tous les disques", "All drives"), bytes: 1.25 * GB, files: 251, running: null },
    ]; })();
function demoInvoke(cmd, args) {
  if (cmd === "organize_apply") demoState.undo = true;
  if (cmd === "organize_undo") demoState.undo = false;
  const wait = (ms, v) => new Promise((r) => setTimeout(() => r(v), ms));
  const GB = 1024 ** 3, MB = 1024 ** 2, now = Date.now() / 1000, day = 86400;
  const f = (path, bytes, daysOld) => {
    const i = path.lastIndexOf("\\");
    return { path, name: path.slice(i + 1), folder: path.slice(0, i), bytes, modified: now - daysOld * day };
  };
  switch (cmd) {
    case "set_language": return wait(5, null);
    case "app_info": return wait(20, { version: "0.8.0", elevated: false });
    case "check_update": return wait(600, { available: true, current: "0.8.0", latest: "0.9.0", notes: "", size: 6.4 * MB, page: "" });
    case "install_update": return wait(1500, null).then(() => Promise.reject(tr("Mode démo : la mise à jour se fait seulement dans l'appli", "Demo mode: updates only happen in the app")));
    case "open_link": return Promise.reject(tr("Mode démo : les liens s'ouvrent seulement dans l'appli", "Demo mode: links only open in the app"));
    case "relaunch_as_admin": return Promise.reject(tr("Mode démo", "Demo mode"));
    case "reveal_file": return Promise.reject(tr("Mode démo : l'Explorateur s'ouvre seulement dans l'appli", "Demo mode: Explorer only opens in the app"));
    case "open_folder": return Promise.reject(tr("Mode démo : l'Explorateur s'ouvre seulement dans l'appli", "Demo mode: Explorer only opens in the app"));
    case "disk_usage": {
      const e = (name, kind, bytes, note) => ({ name, kind, bytes, note, path: "C:\\Users\\toi\\" + name });
      return wait(2400, {
        total: 476 * GB, used: 414.8 * GB,
        groups: [
          { kind: "files", label: tr("Tes fichiers", "Your files"), bytes: 168 * GB },
          { kind: "apps", label: tr("Données d'applis", "App data"), bytes: 71 * GB },
          { kind: "programs", label: tr("Programmes", "Programs"), bytes: 118 * GB },
          { kind: "system", label: tr("Windows et le reste", "Windows and the rest"), bytes: 57.8 * GB },
        ],
        top: [
          e("Videos", "files", 96 * GB, tr("Tes fichiers : « Gros fichiers » et « Doublons » t'aident à trier", "Your files: \"Large files\" and \"Duplicates\" help you sort them")),
          e("Steam", "programs", 74 * GB, tr("Tes jeux Steam : à désinstaller depuis Steam", "Your Steam games: uninstall them from Steam")),
          e("Downloads", "files", 38 * GB, tr("Tes fichiers : « Gros fichiers » et « Doublons » t'aident à trier", "Your files: \"Large files\" and \"Duplicates\" help you sort them")),
          e("Packages", "apps", 20 * GB, tr("Applis du Microsoft Store et leurs données (WhatsApp, Spotify, Claude…)", "Microsoft Store apps and their data (WhatsApp, Spotify, Claude…)")),
          e("Adobe", "programs", 14 * GB, tr("Applis Adobe et leurs caches", "Adobe apps and their caches")),
          e("uv", "apps", 9.2 * GB, tr("Python (uv). Son cache se vide dans Nettoyage › Applications", "Python (uv). Its cache is cleared in Cleanup › Applications")),
          e("Google", "apps", 8.5 * GB, tr("Chrome : profils, extensions et données des sites. Le cache se vide dans Nettoyage", "Chrome: profiles, extensions and site data. The cache is cleared in Cleanup")),
          e(".minecraft", "apps", 2.5 * GB, tr("Mondes, modpacks et versions de Minecraft", "Minecraft worlds, modpacks and versions")),
        ],
      });
    }
    case "disk_info": return wait(50, { name: "C:", total: 476 * GB, free: 61.2 * GB });
    case "scan_junk": return wait(2200, DEMO_JUNK);
    case "clean_junk": {
      const items = DEMO_JUNK.filter((i) => args.ids.includes(i.id));
      const bytes = items.reduce((a, i) => a + i.bytes, 0);
      const files = items.reduce((a, i) => a + i.files, 0);
      return wait(1100, { freed: Math.round(bytes * 0.94), removed: Math.round(files * 0.95), skipped: Math.round(files * 0.05) });
    }
    case "find_large_files": return wait(900, [
      f("D:\\Projets\\Kaury\\Exports\\Rendu_Kaury_Reel_4K_v3.mov", 18 * GB, 240),
      f("C:\\Users\\Theo\\Downloads\\Windows11_23H2.iso", 6.2 * GB, 400),
      f("C:\\Users\\Theo\\Downloads\\Shooting_Vevey_RAW.zip", 4.8 * GB, 150),
      f("C:\\Users\\Theo\\Downloads\\Setup_DaVinci_Resolve.exe", 2.9 * GB, 380),
      f("C:\\Users\\Theo\\Desktop\\Blondel_Display_sources.psd", 1.2 * GB, 700),
    ].filter((x) => x.bytes >= args.minMb * MB));
    case "find_duplicates": return wait(1400, [
      { bytes: 38 * MB, files: [f("C:\\Users\\Theo\\Documents\\Projets\\Logo_Kaury_final_FINAL.ai", 38 * MB, 3), f("C:\\Users\\Theo\\Desktop\\Logo_Kaury_final_FINAL.ai", 38 * MB, 40), f("C:\\Users\\Theo\\Downloads\\Logo_Kaury_final_FINAL.ai", 38 * MB, 41)] },
      { bytes: 22 * MB, files: [f("C:\\Users\\Theo\\Documents\\Projets\\Moodboard_Lac.png", 22 * MB, 12), f("C:\\Users\\Theo\\Desktop\\Moodboard_Lac.png", 22 * MB, 60)] },
    ]);
    case "move_to_trash": return wait(600, { moved: args.paths.length, bytes: args.paths.length * 30 * MB, errors: [] });
    case "list_startup_apps": return wait(200, [
      { id: "hkcu|Adobe Creative Cloud", name: "Adobe Creative Cloud", command: '"C:\\Program Files\\Adobe\\Adobe Creative Cloud\\ACC\\Creative Cloud.exe" --showwindow=false', scope: "user", enabled: true },
      { id: "hkcu|Discord", name: "Discord", command: "C:\\Users\\Theo\\AppData\\Local\\Discord\\Update.exe --processStart Discord.exe", scope: "user", enabled: true },
      { id: "folder|Logitech G HUB.lnk", name: "Logitech G HUB", command: "C:\\Users\\Theo\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\Logitech G HUB.lnk", scope: "user", enabled: true },
      { id: "hkcu|Spotify", name: "Spotify", command: "C:\\Users\\Theo\\AppData\\Roaming\\Spotify\\Spotify.exe /minimized", scope: "user", enabled: false },
      { id: "hklm|SecurityHealth", name: "SecurityHealth", command: "%windir%\\system32\\SecurityHealthSystray.exe", scope: "machine", enabled: true },
    ]);
    case "set_startup_app": return wait(150, null);
    case "list_tweaks": return wait(200, [
      { id: "power_high", group: "speed", name: tr("Mode d’alimentation « Performances élevées »", "\"High performance\" power plan"), detail: tr("Le processeur ne ralentit plus pour économiser l’énergie : tout répond plus vite. Idéal sur un PC fixe ; sur un portable, la batterie dure moins longtemps.", "The processor no longer slows down to save power: everything responds faster. Ideal on a desktop PC; on a laptop, the battery runs out sooner."), needs_admin: false, after: "", enabled: false },
      { id: "menu_delay", group: "speed", name: tr("Menus sans délai", "Menus without delay"), detail: tr("Les sous-menus (Ouvrir avec, Envoyer vers…) s’ouvrent tout de suite au lieu d’attendre 0,4 seconde.", "Submenus (Open with, Send to…) open right away instead of waiting 0.4 seconds."), needs_admin: false, after: "signout", enabled: false },
      { id: "animations", group: "speed", name: tr("Couper les animations des fenêtres", "Turn off window animations"), detail: tr("Les fenêtres s’ouvrent et se réduisent d’un coup, sans effet de zoom. Le PC paraît nettement plus vif, surtout s’il est ancien.", "Windows open and minimize instantly, with no zoom effect. The PC feels much snappier, especially an older one."), needs_admin: false, after: "signout", enabled: false },
      { id: "this_pc", group: "speed", name: tr("L’Explorateur s’ouvre sur « Ce PC »", "Explorer opens on \"This PC\""), detail: tr("Au lieu de l’Accueil, qui charge tes fichiers récents et ceux du cloud : la fenêtre s’affiche plus vite.", "Instead of Home, which loads your recent and cloud files: the window shows up faster."), needs_admin: false, after: "", enabled: true },
      { id: "start_web", group: "speed", name: tr("Recherche Windows sans Bing", "Windows search without Bing"), detail: tr("La recherche du menu Démarrer ne cherche plus sur internet : elle trouve tes applis et fichiers plus vite, sans résultats web.", "Start menu search no longer looks on the internet: it finds your apps and files faster, without web results."), needs_admin: true, after: "explorer", enabled: false },
      { id: "game_dvr", group: "games", name: tr("Couper l’enregistrement en arrière-plan", "Turn off background recording"), detail: tr("La Xbox Game Bar n’enregistre plus tes parties en continu : quelques images par seconde en plus dans les jeux.", "The Xbox Game Bar no longer records your games continuously: a few more frames per second in games."), needs_admin: false, after: "", enabled: false },
      { id: "suggestions", group: "calm", name: tr("Plus de suggestions ni de pubs de Windows", "No more Windows suggestions or ads"), detail: tr("Coupe les « astuces », les applis suggérées, les écrans de bienvenue après les mises à jour et les rappels pour « terminer la configuration ».", "Turns off \"tips\", suggested apps, welcome screens after updates and reminders to \"finish setting up\"."), needs_admin: false, after: "", enabled: false },
      { id: "classic_menu", group: "handy", name: tr("Menu clic droit complet", "Full right-click menu"), detail: tr("Le clic droit affiche directement toutes les options, sans passer par « Afficher plus d’options ».", "Right-click shows every option right away, without going through \"Show more options\"."), needs_admin: false, after: "explorer", enabled: false },
      { id: "file_ext", group: "handy", name: tr("Afficher les extensions des fichiers", "Show file extensions"), detail: tr("Tu vois « facture.pdf.exe » au lieu de « facture.pdf » : le piège classique des virus ne marche plus.", "You see \"invoice.pdf.exe\" instead of \"invoice.pdf\": the classic virus trick no longer works."), needs_admin: false, after: "", enabled: true },
      { id: "ad_id", group: "privacy", name: tr("Pas d’identifiant publicitaire", "No advertising ID"), detail: tr("Les applis ne peuvent plus te suivre d’une appli à l’autre pour te montrer des pubs ciblées.", "Apps can no longer track you from one app to another to show targeted ads."), needs_admin: false, after: "", enabled: false },
    ]);
    case "set_tweak": return wait(150, null);
    case "cancel_search": return wait(10, null);
    case "find_old_downloads": return wait(900, [
      f("C:\\Users\\Theo\\Downloads\\Windows11_23H2.iso", 6.2 * GB, 400),
      f("C:\\Users\\Theo\\Downloads\\Setup_DaVinci_Resolve.exe", 2.9 * GB, 380),
      f("C:\\Users\\Theo\\Downloads\\Mockups_packaging.zip", 820 * MB, 210),
      f(tr("C:\\Users\\Theo\\Downloads\\Facture_imprimeur_mars.pdf", "C:\\Users\\Theo\\Downloads\\Printer_invoice_March.pdf"), 2 * MB, 190),
    ].filter((x) => (now - x.modified) / day >= args.minDays));
    case "organize_plan": return wait(150, args.folder === "desktop" ? [
      { category: "Images", count: 23, bytes: 180 * MB, examples: ["capture-2026-09-12.png", "moodboard.jpg", "ref_lac.png"] },
      { category: "Design", count: 6, bytes: 1.4 * GB, examples: ["Blondel_Display_sources.psd", "logo_v4.ai"] },
    ] : [
      { category: "Images", count: 48, bytes: 620 * MB, examples: ["IMG_4521.jpg", "moodboard.png", "photo_client.heic"] },
      { category: "Documents", count: 31, bytes: 84 * MB, examples: EN ? ["Client_brief_Mercier.pdf", "Quote_2026.xlsx", "contract.docx"] : ["Brief_client_Mercier.pdf", "Devis_2026.xlsx", "contrat.docx"] },
      { category: "Design", count: 12, bytes: 2.1 * GB, examples: ["Logo_Kaury_final_FINAL.ai", "mockup.psd"] },
      { category: tr("Installeurs", "Installers"), count: 9, bytes: 4.3 * GB, examples: ["Setup_DaVinci_Resolve.exe", "Figma-Setup.exe"] },
      { category: "Archives", count: 7, bytes: 5.1 * GB, examples: ["Shooting_Vevey_RAW.zip"] },
    ]);
    case "organize_apply": return wait(700, { moved: 107, errors: [] });
    case "organize_can_undo": return wait(20, demoState.undo);
    case "organize_undo": return wait(500, { moved: 107, errors: [] });
    case "list_installed_apps": return wait(300, [
      { id: "hklm|Adobe", name: "Adobe Creative Cloud", publisher: "Adobe Inc.", version: "6.4.0", bytes: 1.1 * GB, installed: "20250314" },
      { id: "hklm|Blender", name: "Blender", publisher: "Blender Foundation", version: "4.2.1", bytes: 780 * MB, installed: "20240802" },
      { id: "hkcu|Discord", name: "Discord", publisher: "Discord Inc.", version: "1.0.9160", bytes: 410 * MB, installed: "20260101" },
      { id: "hklm|Figma", name: "Figma", publisher: "Figma, Inc.", version: "125.4", bytes: 290 * MB, installed: "20260812" },
      { id: "hklm|McAfee", name: "McAfee WebAdvisor", publisher: "McAfee, LLC", version: "4.1.1", bytes: 48 * MB, installed: "20230519" },
      { id: "hklm|Steam", name: "Steam", publisher: "Valve Corporation", version: "2.10.91", bytes: 0, installed: "" },
    ]);
    case "uninstall_app": return Promise.reject(tr("Mode démo : le désinstalleur s'ouvre seulement dans l'appli", "Demo mode: the uninstaller only opens in the app"));
    case "memory_status": return wait(250, { total: 16 * GB, used: 13.1 * GB, apps: [
      { exe: "chrome.exe", name: "Chrome", bytes: 3.9 * GB, processes: 38 },
      { exe: "Adobe Premiere Pro.exe", name: "Adobe Premiere Pro", bytes: 2.6 * GB, processes: 1 },
      { exe: "Discord.exe", name: "Discord", bytes: 780 * MB, processes: 6 },
      { exe: "Spotify.exe", name: "Spotify", bytes: 420 * MB, processes: 5 },
      { exe: "Figma.exe", name: "Figma", bytes: 390 * MB, processes: 4 },
    ] });
    case "close_app": return wait(800, args.force || args.exe !== "Adobe Premiere Pro.exe");
    case "list_maintenance": return wait(50, [
      { id: "restore_point", name: tr("Créer un point de restauration", "Create a restore point"), detail: tr("Une sauvegarde de l'état de Windows, pour revenir en arrière si une réparation ou une désinstallation se passe mal. Active la protection du système si elle est coupée. À faire avant les autres tâches.", "A snapshot of Windows' state, to go back if a repair or an uninstall goes wrong. Turns on system protection if it's off. Do this before the other tasks."), duration: tr("1 à 2 min", "1 to 2 min"), needs_admin: true },
      { id: "repair_windows", name: tr("Réparer Windows", "Repair Windows"), detail: tr("Vérifie et répare les fichiers système abîmés (DISM puis SFC). À faire si Windows plante, affiche des erreurs bizarres ou si des applis ne s'ouvrent plus. Il faut Internet.", "Checks and repairs damaged system files (DISM then SFC). Do this if Windows crashes, shows strange errors or apps no longer open. Requires Internet."), duration: tr("15 à 30 min", "15 to 30 min"), needs_admin: true },
      { id: "component_cleanup", name: tr("Supprimer les anciennes versions de Windows", "Remove old Windows versions"), detail: tr("Retire les composants remplacés par les mises à jour. Libère souvent plusieurs Go.", "Removes components replaced by updates. Often frees several GB."), duration: tr("5 à 15 min", "5 to 15 min"), needs_admin: true },
      { id: "optimize_drive", name: tr("Optimiser le disque", "Optimize the drive"), detail: tr("Envoie TRIM à un SSD ou défragmente un disque dur, selon ton matériel. Garde le disque rapide.", "Sends TRIM to an SSD or defragments a hard drive, depending on your hardware. Keeps the drive fast."), duration: tr("1 à 10 min", "1 to 10 min"), needs_admin: true },
      { id: "check_disk", name: tr("Vérifier le disque", "Check the drive"), detail: tr("Cherche les erreurs du système de fichiers sans redémarrer.", "Looks for file system errors without restarting."), duration: tr("2 à 10 min", "2 to 10 min"), needs_admin: true },
      { id: "hibernate_off", name: tr("Désactiver la veille prolongée", "Turn off hibernation"), detail: tr("Supprime le fichier hiberfil.sys, souvent aussi gros que la moitié de ta RAM. Utile sur un PC fixe ; sur un portable, garde-la si tu utilises la veille prolongée. Le démarrage rapide de Windows est aussi désactivé.", "Deletes hiberfil.sys, often as large as half your RAM. Useful on a desktop PC; on a laptop, keep it if you use hibernation. Windows Fast Startup is also turned off."), duration: tr("quelques secondes", "a few seconds"), needs_admin: true },
      { id: "flush_dns", name: tr("Vider le cache DNS", "Flush the DNS cache"), detail: tr("Règle les sites qui ne chargent plus ou qui affichent une ancienne version.", "Fixes sites that no longer load or show an old version."), duration: tr("quelques secondes", "a few seconds"), needs_admin: false },
      { id: "restart_explorer", name: tr("Redémarrer l'Explorateur", "Restart Explorer"), detail: tr("Débloque la barre des tâches, le menu Démarrer ou le Bureau quand ils sont figés.", "Unfreezes the taskbar, Start menu or Desktop when they're stuck."), duration: tr("quelques secondes", "a few seconds"), needs_admin: false },
      { id: "refresh_icons", name: tr("Rafraîchir les icônes", "Refresh icons"), detail: tr("Corrige les icônes blanches ou mauvaises sur le Bureau et dans l'Explorateur.", "Fixes blank or wrong icons on the Desktop and in Explorer."), duration: tr("quelques secondes", "a few seconds"), needs_admin: false },
      { id: "reset_store", name: tr("Réparer le Microsoft Store", "Repair the Microsoft Store"), detail: tr("Vide le cache du Store quand les téléchargements ou les mises à jour d'applis bloquent.", "Clears the Store cache when app downloads or updates get stuck."), duration: tr("quelques secondes", "a few seconds"), needs_admin: false },
    ]);
    case "run_maintenance": return wait(1500, { ok: true, message: tr("Cache DNS vidé.", "DNS cache flushed.") });
    default: return Promise.reject("Commande inconnue : " + cmd);
  }
}
