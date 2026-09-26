// Kaury Clean : interface. Tout le travail sur le disque se fait côté Rust (src-tauri).

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : demoInvoke;
let elevated = false;
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

function fmt(bytes) {
  const units = ["o", "Ko", "Mo", "Go", "To"];
  let v = bytes, i = 0;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  const digits = i >= 3 ? 1 : 0;
  return v.toFixed(digits).replace(".", ",") + " " + units[i];
}

function ago(unixSecs) {
  if (!unixSecs) return "";
  const days = (Date.now() / 1000 - unixSecs) / 86400;
  if (days < 1) return "aujourd'hui";
  if (days < 30) return `il y a ${Math.round(days)} j`;
  if (days < 365) return `il y a ${Math.round(days / 30)} mois`;
  const years = Math.round(days / 365);
  return `il y a ${years} an${years > 1 ? "s" : ""}`;
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
  if (st.freed) $("#stats").textContent = `${fmt(st.freed)} libérés depuis l'installation · dernier nettoyage ${ago(st.last)}`;
}

// Un élément bloqué tant que l'appli n'a pas les droits administrateur.
const locked = (item) => item.needs_admin && !elevated;

const sumChecked = (root) => $$("input[type=checkbox]:checked", root).reduce((a, i) => a + Number(i.dataset.bytes || 0), 0);

// ---------- Navigation ----------
$$("nav button").forEach((b) => b.addEventListener("click", () => show(b.dataset.view)));
function show(view) {
  $$("nav button").forEach((b) => b.setAttribute("aria-current", String(b.dataset.view === view)));
  $$(".view").forEach((s) => (s.hidden = s.id !== "v-" + view));
  if (view === "startup" && !startupLoaded) loadStartup();
  viewLoaders[view]?.();
}
// Les modules de modules.js s'inscrivent ici pour se charger à la première ouverture.
const viewLoaders = {};

// ---------- Disque ----------
async function refreshDisk() {
  try {
    const d = await invoke("disk_info");
    if (!d) return;
    $("#disk").hidden = false;
    $("#diskName").textContent = `Disque local (${d.name})`;
    $("#diskFree").textContent = `${fmt(d.free)} libres`;
    $("#diskTotal").textContent = fmt(d.total);
    $("#diskBar").style.width = (((d.total - d.free) / d.total) * 100).toFixed(1) + "%";
  } catch (e) { console.error(e); }
}

// ---------- Anneau animé ----------
const cv = $("#orb"), ctx = cv.getContext("2d");
let prog = 0, spin = 0, ringMode = "idle";
function drawOrb() {
  const W = cv.width, c = W / 2;
  ctx.clearRect(0, 0, W, W);
  const g = ctx.createRadialGradient(c, c, 20, c, c, c);
  g.addColorStop(0, "rgba(245,110,46,.35)"); g.addColorStop(0.6, "rgba(245,110,46,.08)"); g.addColorStop(1, "rgba(245,110,46,0)");
  ctx.fillStyle = g; ctx.beginPath(); ctx.arc(c, c, c, 0, 7); ctx.fill();
  ctx.lineWidth = 14; ctx.lineCap = "round";
  ctx.strokeStyle = "#2A2523"; ctx.beginPath(); ctx.arc(c, c, 165, 0, Math.PI * 2); ctx.stroke();
  const gr = ctx.createLinearGradient(0, 0, W, W); gr.addColorStop(0, "#FF9A5C"); gr.addColorStop(1, "#F56E2E");
  ctx.strokeStyle = gr; ctx.beginPath();
  if (ringMode === "spin") ctx.arc(c, c, 165, spin, spin + 1.1);
  else if (ringMode === "idle") ctx.arc(c, c, 165, spin, spin + 0.6);
  else ctx.arc(c, c, 165, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * prog);
  ctx.stroke();
  for (let i = 0; i < 48; i++) {
    const a = (i / 48) * Math.PI * 2 + spin * 0.3;
    ctx.fillStyle = ringMode === "progress" && i / 48 < prog ? "rgba(241,232,203,.6)" : "rgba(241,232,203,.12)";
    ctx.beginPath(); ctx.arc(c + Math.cos(a) * 192, c + Math.sin(a) * 192, 2.5, 0, 7); ctx.fill();
  }
  if (!reduceMotion) spin += ringMode === "spin" ? 0.08 : 0.02;
  requestAnimationFrame(drawOrb);
}
drawOrb();
const orbVal = (big, small) => ($("#orbVal").innerHTML = `${esc(big)}<small>${esc(small)}</small>`);

// ---------- Fichiers inutiles (analyse intelligente + 3 modules) ----------
let junk = null; // dernier résultat de scan_junk
const GROUPS = { system: "Fichiers système", apps: "Applications", browsers: "Navigateurs", trash: "Corbeille" };
let scanState = "idle";

$("#scanBtn").addEventListener("click", () => {
  if (scanState === "idle" || scanState === "done") runScan();
  else if (scanState === "found") runClean($$("#results input:checked").map((i) => i.value), "scan");
});
$("#resetBtn").addEventListener("click", runScan);

async function runScan() {
  scanState = "scanning";
  ringMode = "spin";
  $("#scanBtn").disabled = true; $("#scanBtn").textContent = "Analyse…";
  $("#resetBtn").hidden = true; $("#results").hidden = true; $("#scanText").hidden = false;
  $("#scanTitle").textContent = "Analyse en cours";
  $("#scanText").textContent = "On passe en revue les caches, les fichiers temporaires et la corbeille.";
  $("#stats").hidden = true;
  orbVal("…", "analyse");
  progressEl = $("#scanText");
  try {
    junk = await invoke("scan_junk");
    progressEl = null;
  } catch (e) {
    progressEl = null;
    scanState = "idle"; ringMode = "idle";
    $("#scanBtn").disabled = false; $("#scanBtn").textContent = "Analyser";
    $("#scanTitle").textContent = "L'analyse n'a pas pu aboutir.";
    $("#scanText").textContent = String(e);
    return;
  }
  renderJunkEverywhere();
  showScanResults();
}

function groupTotal(group) {
  return (junk || []).filter((i) => i.group === group && !locked(i)).reduce((a, i) => a + i.bytes, 0);
}

function showScanResults() {
  scanState = "found";
  ringMode = "progress"; prog = 1;
  $("#scanTitle").textContent = "Voilà ce qu'on peut libérer.";
  $("#scanText").hidden = true;
  const r = $("#results");
  r.hidden = false;
  r.innerHTML = Object.entries(GROUPS).map(([g, title]) => {
    const items = junk.filter((i) => i.group === g && i.bytes > 0 && !locked(i));
    if (!items.length) return "";
    const warn = items.find((i) => i.running);
    return `<label class="res"><input type="checkbox" checked value="${esc(g)}" data-bytes="${groupTotal(g)}">
      <span class="txt"><div class="t">${esc(title)}${warn ? `<span class="chip warn">${esc(warn.running)} ouvert</span>` : ""}</div>
      <div class="d">${items.length} élément${items.length > 1 ? "s" : ""} · ${items.reduce((a, i) => a + i.files, 0).toLocaleString("fr-CH")} fichiers</div></span>
      <span class="s">${fmt(groupTotal(g))}</span></label>`;
  }).join("");
  const update = () => {
    const bytes = sumChecked(r);
    orbVal(fmt(bytes), "à libérer");
    $("#scanBtn").disabled = bytes === 0;
  };
  if (!r.innerHTML.trim()) {
    $("#scanTitle").textContent = "Ton PC est déjà tout propre.";
    orbVal("0 o", "à libérer");
    scanState = "done";
    $("#scanBtn").disabled = false; $("#scanBtn").textContent = "Nouvelle analyse";
    return;
  }
  const adminBytes = junk.filter(locked).reduce((a, i) => a + i.bytes, 0);
  if (adminBytes > 0) {
    r.insertAdjacentHTML("beforeend", `<div class="admin-note">+ ${fmt(adminBytes)} dans les dossiers protégés de Windows.<button class="link" data-admin>Relancer en administrateur</button></div>`);
    $("[data-admin]", r).addEventListener("click", relaunchAsAdmin);
  }
  $$("input", r).forEach((i) => i.addEventListener("change", update));
  update();
  $("#scanBtn").textContent = "Nettoyer";
  $("#resetBtn").hidden = false;
}

async function runClean(selection, from) {
  // selection : des groupes (depuis l'analyse) ou des identifiants (depuis un module)
  const ids = from === "scan"
    ? junk.filter((i) => selection.includes(i.group) && !locked(i)).map((i) => i.id)
    : selection;
  if (!ids.length) return;
  const expected = junk.filter((i) => ids.includes(i.id)).reduce((a, i) => a + i.bytes, 0);

  scanState = "cleaning";
  show("scan");
  $("#results").hidden = true; $("#resetBtn").hidden = true;
  $("#scanBtn").disabled = true; $("#scanBtn").textContent = "Nettoyage…";
  $("#scanTitle").textContent = "Nettoyage en cours";
  ringMode = "spin";
  orbVal(fmt(expected), "à libérer");
  $("#scanText").hidden = false;
  $("#scanText").textContent = "";
  progressEl = $("#scanText");

  let report;
  try {
    report = await invoke("clean_junk", { ids });
    progressEl = null;
  } catch (e) {
    progressEl = null;
    toast("Le nettoyage a échoué : " + e);
    scanState = "done"; ringMode = "idle";
    $("#scanBtn").disabled = false; $("#scanBtn").textContent = "Nouvelle analyse";
    return;
  }
  ringMode = "progress"; prog = 1;
  orbVal(fmt(report.freed), "libérés");
  $("#scanTitle").textContent = "C'est tout propre.";
  $("#scanText").hidden = false;
  $("#scanText").textContent = report.skipped
    ? `${report.removed.toLocaleString("fr-CH")} fichiers supprimés. ${report.skipped.toLocaleString("fr-CH")} fichiers gardés, parce qu'une appli les utilise ou qu'ils ont moins de 24 h. Ferme tes navigateurs et relance pour aller plus loin.`
    : `${report.removed.toLocaleString("fr-CH")} fichiers supprimés.`;
  scanState = "done";
  $("#scanBtn").disabled = false; $("#scanBtn").textContent = "Nouvelle analyse";
  addFreed(report.freed);
  junk = null;
  renderJunkEverywhere();
  refreshDisk();
}

function renderJunkEverywhere() {
  for (const g of Object.keys(GROUPS)) {
    $(`[data-size="${g}"]`).textContent = junk ? fmt(groupTotal(g)) : "";
    renderGroupView(g);
  }
}

function renderGroupView(g) {
  const el = $("#v-" + g);
  const head = `<div class="head"><div><h2>${esc(el.dataset.title)}</h2><p>${esc(el.dataset.desc)}</p></div></div>`;
  if (!junk) {
    el.innerHTML = head + `<div class="empty">Lance une analyse pour voir ce qui peut partir.<button class="cta small" data-scan>Analyser</button></div>`;
    $("[data-scan]", el).addEventListener("click", () => { show("scan"); runScan(); });
    return;
  }
  const items = junk.filter((i) => i.group === g);
  if (!items.length) {
    el.innerHTML = head + `<div class="empty">Rien à nettoyer ici.</div>`;
    return;
  }
  el.innerHTML = head + `<div class="list">${items.map((it) => `
    <label class="item"><input type="checkbox" value="${esc(it.id)}" data-bytes="${it.bytes}" ${it.bytes > 0 && !locked(it) ? "checked" : ""} ${it.bytes > 0 ? "" : "disabled"}>
      <span class="txt"><div class="n">${esc(it.name)}${locked(it) ? `<span class="chip">admin</span>` : ""}${it.running ? `<span class="chip warn">${esc(it.running)} ouvert : ferme-le pour tout nettoyer</span>` : ""}</div>
      <div class="p">${esc(it.detail)} · ${it.files.toLocaleString("fr-CH")} fichiers</div></span>
      <span class="s">${fmt(it.bytes)}</span></label>`).join("")}</div>
    ${items.some(locked) ? `<div class="admin-note left">Les éléments « admin » sont dans des dossiers protégés : sans les droits administrateur, Windows empêche presque tout de partir.<button class="link" data-admin>Relancer en administrateur</button></div>` : ""}
    <div class="foot"><span>Sélection : <b class="sel"></b></span><button class="cta small" data-clean>${g === "trash" ? "Vider la corbeille" : "Nettoyer"}</button></div>`;
  const update = () => {
    const bytes = sumChecked(el);
    $(".sel", el).textContent = fmt(bytes);
    $("[data-clean]", el).disabled = bytes === 0;
  };
  $$("input", el).forEach((i) => i.addEventListener("change", update));
  $("[data-clean]", el).addEventListener("click", () => runClean($$("input:checked", el).map((i) => i.value), g));
  $("[data-admin]", el)?.addEventListener("click", relaunchAsAdmin);
  update();
}

// Affiche « recherche en cours » avec un bouton Arrêter. Renvoie une fonction d'affichage d'erreur.
function startSearch(box, label) {
  box.innerHTML = `<div class="empty"><div class="spinner"></div>${esc(label)}<span class="progress"></span><button class="cta ghost" data-stop>Arrêter</button></div>`;
  progressEl = $(".progress", box);
  $("[data-stop]", box).addEventListener("click", (e) => {
    e.currentTarget.disabled = true;
    invoke("cancel_search");
  });
}
function searchFailed(box, e) {
  progressEl = null;
  box.innerHTML = String(e).includes("arrêtée")
    ? `<div class="empty">Recherche arrêtée.</div>`
    : `<div class="empty">La recherche a échoué : ${esc(e)}</div>`;
}

// ---------- Gros fichiers ----------
$("#largeBtn").addEventListener("click", loadLarge);
async function loadLarge() {
  const box = $("#largeList");
  startSearch(box, "Recherche dans tes dossiers…");
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
  startSearch(box, "Comparaison des fichiers… Ça peut prendre une minute.");
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
    box.innerHTML = `<div class="empty">Rien trouvé. Bonne nouvelle.</div>`;
    return;
  }
  const isDupes = preselect;
  box.innerHTML = `<div class="list">${groups.map((grp) => `
    ${isDupes ? `<div class="group-title"><b>${esc(grp.files[0].name)}</b><span>${grp.files.length} copies · ${fmt(grp.bytes)} chacune</span></div>` : ""}
    ${grp.files.map((f, i) => `
      <label class="item"><input type="checkbox" value="${esc(f.path)}" data-bytes="${f.bytes}" ${isDupes && i > 0 ? "checked" : ""}>
        <span class="txt"><div class="n">${esc(isDupes ? f.folder : f.name)}${isDupes && i === 0 ? `<span class="chip ok">la plus récente</span>` : ""}<span class="chip ${!isDupes && f.bytes > 4 * 1024 ** 3 ? "high" : ""}">${esc(ago(f.modified))}</span></div>
        <div class="p">${esc(isDupes ? f.name : f.folder)}</div></span>
        <span class="end"><button class="reveal" data-reveal="${esc(f.path)}" title="Afficher dans l'Explorateur">Afficher</button><span class="s">${fmt(f.bytes)}</span></span></label>`).join("")}`).join("")}</div>
    <div class="foot"><span>Sélection : <b class="sel"></b></span><button class="cta small" data-trash>Mettre à la corbeille</button></div>`;
  const update = () => {
    const bytes = sumChecked(box);
    $(".sel", box).textContent = fmt(bytes);
    $("[data-trash]", box).disabled = bytes === 0;
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
      toast(`${r.moved} fichier${r.moved > 1 ? "s" : ""} à la corbeille · ${fmt(r.bytes)}` + (r.errors.length ? ` · ${r.errors.length} impossible(s)` : ""));
      if (r.errors.length) console.warn(r.errors);
    } catch (e) {
      toast("Impossible de déplacer les fichiers : " + e);
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
    box.innerHTML = `<div class="empty">Impossible de lire la liste : ${esc(e)}</div>`;
    return;
  }
  if (!apps.length) {
    box.innerHTML = `<div class="empty">Aucune appli ne se lance au démarrage.</div>`;
    return;
  }
  box.innerHTML = `<div class="list">${apps.map((a) => `
    <div class="item"><span class="ic">${esc(a.name.trim()[0] || "?")}</span>
      <span class="txt"><div class="n">${esc(a.name)}${a.scope === "machine" ? `<span class="chip">tous les comptes · admin</span>` : ""}</div>
      <div class="p" title="${esc(a.command)}">${esc(a.command)}</div></span>
      <button class="toggle" role="switch" data-id="${esc(a.id)}" aria-label="${esc(a.name)} au démarrage" aria-checked="${a.enabled}"></button></div>`).join("")}</div>
    <div class="foot"><span>Activées : <b class="sel"></b></span></div>`;
  const update = () => ($(".sel", box).textContent = `${$$('[aria-checked="true"]', box).length} sur ${apps.length}`);
  update();
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

// ---------- Droits administrateur ----------
async function relaunchAsAdmin() {
  try {
    await invoke("relaunch_as_admin");
  } catch (e) {
    toast("Relance annulée. Tu peux continuer sans les droits administrateur.");
  }
}

// Menu clic droit du navigateur (Recharger, Inspecter…) : inutile dans une appli.
document.addEventListener("contextmenu", (e) => {
  if (!e.target.closest(".p")) e.preventDefault();
});

// ---------- Démarrage de l'interface ----------
(async () => {
  try {
    const info = await invoke("app_info");
    elevated = info.elevated;
    $("#version").textContent = `v${info.version}${elevated ? " · administrateur" : ""}`;
  } catch (e) { console.error(e); }
  renderJunkEverywhere();
  renderStats();
  refreshDisk();
})();

// ---------- Mode démo (ouverture dans un navigateur, sans Tauri) ----------
const demoState = { undo: false };
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
    case "app_info": return wait(20, { version: "0.2.0", elevated: false });
    case "relaunch_as_admin": return Promise.reject("Mode démo");
    case "reveal_file": return Promise.reject("Mode démo : l'Explorateur s'ouvre seulement dans l'appli");
    case "disk_info": return wait(50, { name: "C:", total: 476 * GB, free: 61.2 * GB });
    case "scan_junk": return wait(2200, [
      { id: "user_temp", group: "system", name: "Fichiers temporaires", detail: "Dossier Temp de ton compte", bytes: 3.3 * GB, files: 18422, running: null },
      { id: "windows_update", group: "system", name: "Téléchargements Windows Update", detail: "Mises à jour déjà installées", bytes: 2.1 * GB, files: 311, running: null, needs_admin: true },
      { id: "crash_reports", group: "system", name: "Rapports d'erreur", detail: "Rapports de plantage et fichiers dump", bytes: 268 * MB, files: 47, running: null },
      { id: "adobe_media_cache", group: "apps", name: "Cache média Adobe", detail: "Premiere Pro et After Effects", bytes: 1.8 * GB, files: 902, running: null },
      { id: "spotify", group: "apps", name: "Spotify", detail: "Musique mise en cache : elle se retélécharge quand tu l'écoutes", bytes: 3.4 * GB, files: 812, running: "Spotify" },
      { id: "discord", group: "apps", name: "Discord", detail: "Images et vidéos déjà vues", bytes: 640 * MB, files: 4211, running: null },
      { id: "chrome", group: "browsers", name: "Google Chrome", detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas", bytes: 1.2 * GB, files: 6230, running: "Chrome" },
      { id: "edge", group: "browsers", name: "Microsoft Edge", detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas", bytes: 486 * MB, files: 2104, running: null },
      { id: "recycle_bin", group: "trash", name: "Corbeille", detail: "Tous les disques", bytes: 1.25 * GB, files: 251, running: null },
    ]);
    case "clean_junk": return wait(1500, { freed: 8.9 * GB, removed: 26012, skipped: 1240 });
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
    case "cancel_search": return wait(10, null);
    case "find_old_downloads": return wait(900, [
      f("C:\\Users\\Theo\\Downloads\\Windows11_23H2.iso", 6.2 * GB, 400),
      f("C:\\Users\\Theo\\Downloads\\Setup_DaVinci_Resolve.exe", 2.9 * GB, 380),
      f("C:\\Users\\Theo\\Downloads\\Mockups_packaging.zip", 820 * MB, 210),
      f("C:\\Users\\Theo\\Downloads\\Facture_imprimeur_mars.pdf", 2 * MB, 190),
    ].filter((x) => (now - x.modified) / day >= args.minDays));
    case "organize_plan": return wait(150, args.folder === "desktop" ? [
      { category: "Images", count: 23, bytes: 180 * MB, examples: ["capture-2026-09-12.png", "moodboard.jpg", "ref_lac.png"] },
      { category: "Design", count: 6, bytes: 1.4 * GB, examples: ["Blondel_Display_sources.psd", "logo_v4.ai"] },
    ] : [
      { category: "Images", count: 48, bytes: 620 * MB, examples: ["IMG_4521.jpg", "moodboard.png", "photo_client.heic"] },
      { category: "Documents", count: 31, bytes: 84 * MB, examples: ["Brief_client_Mercier.pdf", "Devis_2026.xlsx", "contrat.docx"] },
      { category: "Design", count: 12, bytes: 2.1 * GB, examples: ["Logo_Kaury_final_FINAL.ai", "mockup.psd"] },
      { category: "Installeurs", count: 9, bytes: 4.3 * GB, examples: ["Setup_DaVinci_Resolve.exe", "Figma-Setup.exe"] },
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
    case "uninstall_app": return Promise.reject("Mode démo : le désinstalleur s'ouvre seulement dans l'appli");
    case "memory_status": return wait(250, { total: 16 * GB, used: 13.1 * GB, apps: [
      { exe: "chrome.exe", name: "Chrome", bytes: 3.9 * GB, processes: 38 },
      { exe: "Adobe Premiere Pro.exe", name: "Adobe Premiere Pro", bytes: 2.6 * GB, processes: 1 },
      { exe: "Discord.exe", name: "Discord", bytes: 780 * MB, processes: 6 },
      { exe: "Spotify.exe", name: "Spotify", bytes: 420 * MB, processes: 5 },
      { exe: "Figma.exe", name: "Figma", bytes: 390 * MB, processes: 4 },
    ] });
    case "close_app": return wait(800, args.force || args.exe !== "Adobe Premiere Pro.exe");
    case "list_maintenance": return wait(50, [
      { id: "repair_windows", name: "Réparer Windows", detail: "Vérifie et répare les fichiers système abîmés (DISM puis SFC). À faire si Windows plante, affiche des erreurs bizarres ou si des applis ne s'ouvrent plus. Il faut Internet.", duration: "15 à 30 min", needs_admin: true },
      { id: "component_cleanup", name: "Supprimer les anciennes versions de Windows", detail: "Retire les composants remplacés par les mises à jour. Libère souvent plusieurs Go.", duration: "5 à 15 min", needs_admin: true },
      { id: "optimize_drive", name: "Optimiser le disque", detail: "Envoie TRIM à un SSD ou défragmente un disque dur, selon ton matériel. Garde le disque rapide.", duration: "1 à 10 min", needs_admin: true },
      { id: "check_disk", name: "Vérifier le disque", detail: "Cherche les erreurs du système de fichiers sans redémarrer.", duration: "2 à 10 min", needs_admin: true },
      { id: "flush_dns", name: "Vider le cache DNS", detail: "Règle les sites qui ne chargent plus ou qui affichent une ancienne version.", duration: "quelques secondes", needs_admin: false },
      { id: "restart_explorer", name: "Redémarrer l'Explorateur", detail: "Débloque la barre des tâches, le menu Démarrer ou le Bureau quand ils sont figés.", duration: "quelques secondes", needs_admin: false },
      { id: "refresh_icons", name: "Rafraîchir les icônes", detail: "Corrige les icônes blanches ou mauvaises sur le Bureau et dans l'Explorateur.", duration: "quelques secondes", needs_admin: false },
      { id: "reset_store", name: "Réparer le Microsoft Store", detail: "Vide le cache du Store quand les téléchargements ou les mises à jour d'applis bloquent.", duration: "quelques secondes", needs_admin: false },
    ]);
    case "run_maintenance": return wait(1500, { ok: true, message: "Cache DNS vidé." });
    default: return Promise.reject("Commande inconnue : " + cmd);
  }
}
