// Kaury Clean : interface. Tout le travail sur le disque se fait côté Rust (src-tauri).

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : demoInvoke;
let elevated = false;
let appVersion = "";
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
  logo: '<svg class="k" viewBox="422 533 1156 934"><g transform="translate(0,2000) scale(0.1,-0.1)"><path d="M4335 14651 c-48 -22 -79 -54 -100 -103 -26 -63 -22 -8996 4 -9053 22 -47 64 -89 104 -104 19 -8 386 -11 1163 -11 1094 0 1136 1 1172 19 70 36 96 83 116 213 18 113 33 306 41 528 4 91 8 181 10 200 3 19 7 346 10 725 7 763 7 758 69 796 35 22 93 25 131 7 17 -8 64 -68 130 -168 1007 -1522 2620 -2370 4511 -2370 1020 0 2481 280 3759 720 268 93 311 126 322 246 6 65 17 41 -237 529 -37 72 -138 267 -225 435 -502 974 -461 903 -537 935 -61 25 -91 19 -279 -60 -771 -321 -1382 -450 -2099 -444 -1164 10 -2114 415 -2423 1035 -248 495 -169 1254 188 1804 472 730 1450 1087 2540 929 390 -56 479 -138 462 -427 -17 -290 -157 -509 -400 -630 -291 -144 -616 -143 -1061 3 -43 14 -80 24 -82 22 -33 -33 -53 -717 -26 -902 105 -723 427 -1091 1032 -1180 1034 -152 1863 261 2374 1183 199 361 284 643 325 1087 15 156 6 667 -13 815 -176 1327 -964 2108 -2300 2280 -928 119 -1966 -41 -2896 -448 -1132 -494 -2218 -1387 -3083 -2535 l-142 -188 -5 1998 -5 1998 -30 45 c-19 28 -47 54 -75 68 l-44 22 -1181 0 c-1126 0 -1182 -1 -1220 -19z"/></g></svg>',
  "logo-big": '<svg class="k" viewBox="422 533 1156 934"><g transform="translate(0,2000) scale(0.1,-0.1)"><path d="M4335 14651 c-48 -22 -79 -54 -100 -103 -26 -63 -22 -8996 4 -9053 22 -47 64 -89 104 -104 19 -8 386 -11 1163 -11 1094 0 1136 1 1172 19 70 36 96 83 116 213 18 113 33 306 41 528 4 91 8 181 10 200 3 19 7 346 10 725 7 763 7 758 69 796 35 22 93 25 131 7 17 -8 64 -68 130 -168 1007 -1522 2620 -2370 4511 -2370 1020 0 2481 280 3759 720 268 93 311 126 322 246 6 65 17 41 -237 529 -37 72 -138 267 -225 435 -502 974 -461 903 -537 935 -61 25 -91 19 -279 -60 -771 -321 -1382 -450 -2099 -444 -1164 10 -2114 415 -2423 1035 -248 495 -169 1254 188 1804 472 730 1450 1087 2540 929 390 -56 479 -138 462 -427 -17 -290 -157 -509 -400 -630 -291 -144 -616 -143 -1061 3 -43 14 -80 24 -82 22 -33 -33 -53 -717 -26 -902 105 -723 427 -1091 1032 -1180 1034 -152 1863 261 2374 1183 199 361 284 643 325 1087 15 156 6 667 -13 815 -176 1327 -964 2108 -2300 2280 -928 119 -1966 -41 -2896 -448 -1132 -494 -2218 -1387 -3083 -2535 l-142 -188 -5 1998 -5 1998 -30 45 c-19 28 -47 54 -75 68 l-44 22 -1181 0 c-1126 0 -1182 -1 -1220 -19z"/></g></svg>',
};
function paintIcons(root = document) {
  $$("[data-icon]", root).forEach((el) => { if (!el.firstChild) el.innerHTML = ICONS[el.dataset.icon] || ""; });
}
paintIcons();

// ---------- Navigation ----------
// Six sections dans la barre latérale ; certaines ont des onglets en haut de page.
const SECTIONS = {
  home: [["home", "Entretien intelligent"]],
  clean: [["system", "Fichiers système"], ["apps", "Applications"], ["browsers", "Navigateurs"], ["trash", "Corbeille"]],
  perf: [["memory", "Mémoire vive"], ["startup", "Démarrage"]],
  apps: [["uninstall", "Désinstaller"]],
  files: [["space", "Place du disque"], ["large", "Gros fichiers"], ["dupes", "Doublons"], ["olddl", "Vieux téléchargements"], ["organize", "Ranger"]],
  repair: [["maintenance", "Maintenance"]],
  about: [["about", "À propos"]],
};
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
    $("#diskName").textContent = `Disque ${d.name}`;
    $("#diskFree").textContent = `${fmt(d.free)} libres`;
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
const GROUPS = { system: "Fichiers système", apps: "Applications", browsers: "Navigateurs", trash: "Corbeille" };
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
  btn.textContent = { idle: "Analyser", scanning: "Analyse", results: "Lancer", running: "En cours", done: "Terminé" }[state];
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
  $("#scanTitle").textContent = "Analyse en cours";
  orbVal("…", "analyse");
  progressEl = $("#careProgress");
  progressEl.textContent = "Préparation";
  const soft = (p) => p.catch(() => null);
  try {
    const [j, mem, startup, installed, oldFiles] = await Promise.all([
      invoke("scan_junk"),
      soft(invoke("memory_status")),
      soft(invoke("list_startup_apps")),
      soft(invoke("list_installed_apps")),
      soft(invoke("find_old_downloads", { minDays: 180 })),
    ]);
    junk = j;
    care = { ...care, mem, startup, installed, oldFiles, includeTrash: true };
  } catch (e) {
    progressEl = null;
    careView("idle");
    toast("L'analyse n'a pas pu aboutir : " + e);
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

  $("#careTitle").textContent = done ? "Bravo ! Ton PC est en pleine forme." : "Voilà ce qu'on a trouvé.";
  $("#careLead").textContent = done
    ? (r.skipped ? `${r.skipped.toLocaleString("fr-CH")} fichiers gardés : une appli les utilise ou ils ont moins de 24 h.` : "Tout ce qui pouvait partir est parti.")
    : "Clique sur Lancer pour nettoyer. Le reste t'attend dans chaque section.";

  const cleanExtra = done ? "" : `
    ${trash && trash.bytes > 0 ? `<label class="mini-check"><input type="checkbox" id="careTrash" ${care.includeTrash ? "checked" : ""}> Vider aussi la corbeille (${fmt(trash.bytes)})</label>` : ""}
    ${running.length ? `<div class="mini-note">${esc([...new Set(running)].join(", "))} ${new Set(running).size > 1 ? "ouverts" : "ouvert"} : une partie restera</div>` : ""}
    ${adminBytes > 0 ? `<button class="link mini" data-admin>+ ${fmt(adminBytes)} avec les droits administrateur</button>` : ""}`;

  const cards = [
    card({ cls: "wide", kicker: "Nettoyage", icon: "broom", tile: "t-clean",
      big: done ? fmt(r.freed) : fmt(careBytes()), sub: done ? "libérés" : "de fichiers inutiles",
      status: done ? { kind: "ok", text: "Nettoyé" } : null, action: done ? null : { go: "system", label: "Détails" }, extra: cleanExtra }),
    card({ kicker: "Performances", icon: "bolt", tile: "t-perf",
      big: startupOn != null ? `${startupOn} appli${startupOn > 1 ? "s" : ""}` : "—", sub: `au démarrage${memPct != null ? ` · mémoire ${memPct} %` : ""}`,
      status: done ? (care.dns ? { kind: "ok", text: "Cache DNS vidé" } : null) : null, action: { go: "startup", label: "Voir" } }),
    card({ kicker: "Espace disque", icon: "disk", tile: "t-repair",
      big: diskInfo ? fmt(diskInfo.free) : "—", sub: diskInfo ? `libres sur ${fmt(diskInfo.total)}` : "",
      status: done ? { kind: "ok", text: "Mis à jour" } : null,
      extra: diskInfo ? `<div class="bar"><i style="width:${(((diskInfo.total - diskInfo.free) / diskInfo.total) * 100).toFixed(1)}%"></i></div>` : "" }),
    card({ cls: "half", kicker: "Applications", icon: "grid", tile: "t-apps",
      big: care.installed ? `${care.installed.length} applis` : "—", sub: appsBytes ? `${fmt(appsBytes)} au total` : "installées",
      status: done ? { kind: "ok", text: "Vérifiées" } : null, action: { go: "uninstall", label: "Examiner" } }),
    card({ cls: "half", kicker: "Mes fichiers", icon: "folder", tile: "t-files",
      big: `${old.length} fichier${old.length > 1 ? "s" : ""}`, sub: old.length ? `oubliés dans Téléchargements · ${fmt(oldBytes)}` : "rien d'oublié dans Téléchargements",
      status: done && old.length ? { kind: "todo", text: "À examiner" } : null, action: { go: "olddl", label: "Examiner" } }),
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
  steps.push({ key: "dns", label: "Cache DNS", ids: [] });

  careView("running");
  $("#scanTitle").textContent = "Nettoyage en cours";
  const total = careBytes();
  let freed = 0;
  ringMode = "progress"; prog = 0;
  orbVal(fmt(0), `sur ${fmt(total)}`);
  const list = $("#careSteps");
  list.hidden = false;
  list.innerHTML = steps.map((st) => `<li data-step="${st.key}" class="wait">
      <span class="tile sm ${STEP_LOOK[st.key].tile}" data-icon="${STEP_LOOK[st.key].icon}"></span>
      <span class="step-name">${esc(st.label)}</span><span class="step-state">En attente</span></li>`).join("");
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
        $(".step-state", li).textContent = dns ? "✓ Vidé" : "Pas disponible";
      } else {
        const r = await invoke("clean_junk", { ids: st.ids });
        report.freed += r.freed; report.removed += r.removed; report.skipped += r.skipped;
        freed += r.freed;
        $(".step-state", li).textContent = `✓ ${fmt(r.freed)}`;
      }
      li.className = "done";
    } catch (e) {
      li.className = "fail";
      $(".step-state", li).textContent = "Échec";
      console.error(e);
    }
    prog = (n + 1) / steps.length;
    orbVal(fmt(freed), `sur ${fmt(total)}`);
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
  const head = `<div class="head"><div><h2>${esc(el.dataset.title)}</h2><p>${esc(el.dataset.desc)}</p></div></div>`;
  if (!junk) {
    el.innerHTML = head + `<div class="empty">Lance une analyse pour voir ce qui peut partir.<button class="cta small" data-scan>Analyser</button></div>`;
    $("[data-scan]", el).addEventListener("click", scanJunkOnly);
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
  $("[data-clean]", el).addEventListener("click", (e) => cleanFromSection(e.currentTarget, $$("input:checked", el).map((i) => i.value)));
  $("[data-admin]", el)?.addEventListener("click", relaunchAsAdmin);
  update();
}

async function scanJunkOnly() {
  $$(".view:not([hidden]) [data-scan]").forEach((b) => { b.disabled = true; b.textContent = "Analyse…"; });
  try {
    junk = await invoke("scan_junk");
  } catch (e) {
    toast("L'analyse a échoué : " + e);
  }
  renderJunkEverywhere();
}

// Nettoyage depuis un onglet : on reste sur place, puis on réanalyse pour mettre les chiffres à jour.
async function cleanFromSection(btn, ids) {
  if (!ids.length) return;
  btn.disabled = true;
  btn.textContent = "Nettoyage…";
  try {
    const r = await invoke("clean_junk", { ids });
    addFreed(r.freed);
    toast(`${fmt(r.freed)} libérés` + (r.skipped ? ` · ${r.skipped.toLocaleString("fr-CH")} fichiers gardés (utilisés ou trop récents)` : ""));
  } catch (e) {
    toast("Le nettoyage a échoué : " + e);
  }
  refreshDisk();
  await scanJunkOnly();
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
  box.innerHTML = `<div class="list">${groups.map((grp, g) => `
    ${isDupes ? `<div class="group-title"><b>${esc(grp.files[0].name)}</b><span>${grp.files.length} copies · ${fmt(grp.bytes)} chacune</span></div>` : ""}
    ${grp.files.map((f, i) => `
      <label class="item"><input type="checkbox" value="${esc(f.path)}" data-bytes="${f.bytes}" data-group="${g}" ${isDupes && i > 0 ? "checked" : ""}>
        <span class="txt"><div class="n">${esc(isDupes ? f.folder : f.name)}${isDupes && i === 0 ? `<span class="chip ok">la plus récente</span>` : ""}<span class="chip ${!isDupes && f.bytes > 4 * 1024 ** 3 ? "high" : ""}">${esc(ago(f.modified))}</span></div>
        <div class="p">${esc(isDupes ? f.name : f.folder)}</div></span>
        <span class="end"><button class="reveal" data-reveal="${esc(f.path)}" title="Afficher dans l'Explorateur">Afficher</button><span class="s">${fmt(f.bytes)}</span></span></label>`).join("")}`).join("")}</div>
    <div class="foot"><span>Sélection : <b class="sel"></b><span class="warn-text" hidden></span></span><button class="cta small" data-trash>Mettre à la corbeille</button></div>`;
  const update = () => {
    const bytes = sumChecked(box);
    $(".sel", box).textContent = fmt(bytes);
    // Doublons : on refuse de supprimer toutes les copies d'un même fichier.
    const allGone = isDupes && groups.some((grp, g) => $$(`input[data-group="${g}"]:not(:checked)`, box).length === 0);
    const warn = $(".warn-text", box);
    warn.hidden = !allGone;
    warn.textContent = " · garde au moins une copie de chaque fichier";
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
      toast(`${r.moved} fichier${r.moved > 1 ? "s" : ""} à la corbeille (${fmt(r.bytes)}). Vide la corbeille pour récupérer la place.` + (r.errors.length ? ` ${r.errors.length} impossible(s).` : ""));
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
  const lockedApp = (a) => a.scope === "machine" && !elevated;
  box.innerHTML = `${apps.some(lockedApp) ? `<div class="admin-note left above">Les applis « tous les comptes » se modifient avec les droits administrateur.<button class="link" data-admin>Relancer en administrateur</button></div>` : ""}
    <div class="list">${apps.map((a) => `
    <div class="item"><span class="ic">${esc(a.name.trim()[0] || "?")}</span>
      <span class="txt"><div class="n">${esc(a.name)}${a.scope === "machine" ? `<span class="chip">tous les comptes · admin</span>` : ""}</div>
      <div class="p" title="${esc(a.command)}">${esc(a.command)}</div></span>
      <button class="toggle" role="switch" data-id="${esc(a.id)}" aria-label="${esc(a.name)} au démarrage" aria-checked="${a.enabled}" ${lockedApp(a) ? "disabled" : ""}></button></div>`).join("")}</div>
    <div class="foot"><span>Activées : <b class="sel"></b></span></div>`;
  const update = () => ($(".sel", box).textContent = `${$$('[aria-checked="true"]', box).length} sur ${apps.length}`);
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

// ---------- Droits administrateur ----------
async function relaunchAsAdmin() {
  try {
    await invoke("relaunch_as_admin");
  } catch (e) {
    toast("Relance annulée. Tu peux continuer sans les droits administrateur.");
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

// ---------- Mode démo (ouverture dans un navigateur, sans Tauri) ----------
const demoState = { undo: false };
const DEMO_JUNK = (() => { const GB = 1024 ** 3, MB = 1024 ** 2; return [
      { id: "user_temp", group: "system", name: "Fichiers temporaires", detail: "Dossier Temp de ton compte", bytes: 3.3 * GB, files: 18422, running: null },
      { id: "windows_update", group: "system", name: "Téléchargements Windows Update", detail: "Mises à jour déjà installées", bytes: 2.1 * GB, files: 311, running: null, needs_admin: true },
      { id: "crash_reports", group: "system", name: "Rapports d'erreur", detail: "Rapports de plantage et fichiers dump", bytes: 268 * MB, files: 47, running: null },
      { id: "adobe_media_cache", group: "apps", name: "Cache média Adobe", detail: "Premiere Pro et After Effects", bytes: 1.8 * GB, files: 902, running: null },
      { id: "spotify", group: "apps", name: "Spotify", detail: "Musique mise en cache : elle se retélécharge quand tu l'écoutes", bytes: 3.4 * GB, files: 812, running: "Spotify" },
      { id: "discord", group: "apps", name: "Discord", detail: "Images et vidéos déjà vues", bytes: 640 * MB, files: 4211, running: null },
      { id: "chrome", group: "browsers", name: "Google Chrome", detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas", bytes: 1.2 * GB, files: 6230, running: "Chrome" },
      { id: "edge", group: "browsers", name: "Microsoft Edge", detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas", bytes: 486 * MB, files: 2104, running: null },
      { id: "recycle_bin", group: "trash", name: "Corbeille", detail: "Tous les disques", bytes: 1.25 * GB, files: 251, running: null },
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
    case "app_info": return wait(20, { version: "0.7.0", elevated: false });
    case "check_update": return wait(600, { available: true, current: "0.7.0", latest: "0.8.0", notes: "", size: 6.4 * MB, page: "" });
    case "install_update": return wait(1500, null).then(() => Promise.reject("Mode démo : la mise à jour se fait seulement dans l'appli"));
    case "open_link": return Promise.reject("Mode démo : les liens s'ouvrent seulement dans l'appli");
    case "relaunch_as_admin": return Promise.reject("Mode démo");
    case "reveal_file": return Promise.reject("Mode démo : l'Explorateur s'ouvre seulement dans l'appli");
    case "open_folder": return Promise.reject("Mode démo : l'Explorateur s'ouvre seulement dans l'appli");
    case "disk_usage": {
      const e = (name, kind, bytes, note) => ({ name, kind, bytes, note, path: "C:\\Users\\toi\\" + name });
      return wait(2400, {
        total: 476 * GB, used: 414.8 * GB,
        groups: [
          { kind: "files", label: "Tes fichiers", bytes: 168 * GB },
          { kind: "apps", label: "Données d'applis", bytes: 71 * GB },
          { kind: "programs", label: "Programmes", bytes: 118 * GB },
          { kind: "system", label: "Windows et le reste", bytes: 57.8 * GB },
        ],
        top: [
          e("Videos", "files", 96 * GB, "Tes fichiers : « Gros fichiers » et « Doublons » t'aident à trier"),
          e("Steam", "programs", 74 * GB, "Tes jeux Steam : à désinstaller depuis Steam"),
          e("Downloads", "files", 38 * GB, "Tes fichiers : « Gros fichiers » et « Doublons » t'aident à trier"),
          e("Packages", "apps", 20 * GB, "Applis du Microsoft Store et leurs données (WhatsApp, Spotify, Claude…)"),
          e("Adobe", "programs", 14 * GB, "Applis Adobe et leurs caches"),
          e("uv", "apps", 9.2 * GB, "Python (uv). Son cache se vide dans Nettoyage › Applications"),
          e("Google", "apps", 8.5 * GB, "Chrome : profils, extensions et données des sites. Le cache se vide dans Nettoyage"),
          e(".minecraft", "apps", 2.5 * GB, "Mondes, modpacks et versions de Minecraft"),
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
      { id: "restore_point", name: "Créer un point de restauration", detail: "Une sauvegarde de l'état de Windows, pour revenir en arrière si une réparation ou une désinstallation se passe mal. Active la protection du système si elle est coupée. À faire avant les autres tâches.", duration: "1 à 2 min", needs_admin: true },
      { id: "repair_windows", name: "Réparer Windows", detail: "Vérifie et répare les fichiers système abîmés (DISM puis SFC). À faire si Windows plante, affiche des erreurs bizarres ou si des applis ne s'ouvrent plus. Il faut Internet.", duration: "15 à 30 min", needs_admin: true },
      { id: "component_cleanup", name: "Supprimer les anciennes versions de Windows", detail: "Retire les composants remplacés par les mises à jour. Libère souvent plusieurs Go.", duration: "5 à 15 min", needs_admin: true },
      { id: "optimize_drive", name: "Optimiser le disque", detail: "Envoie TRIM à un SSD ou défragmente un disque dur, selon ton matériel. Garde le disque rapide.", duration: "1 à 10 min", needs_admin: true },
      { id: "check_disk", name: "Vérifier le disque", detail: "Cherche les erreurs du système de fichiers sans redémarrer.", duration: "2 à 10 min", needs_admin: true },
      { id: "hibernate_off", name: "Désactiver la veille prolongée", detail: "Supprime le fichier hiberfil.sys, souvent aussi gros que la moitié de ta RAM. Utile sur un PC fixe ; sur un portable, garde-la si tu utilises la veille prolongée. Le démarrage rapide de Windows est aussi désactivé.", duration: "quelques secondes", needs_admin: true },
      { id: "flush_dns", name: "Vider le cache DNS", detail: "Règle les sites qui ne chargent plus ou qui affichent une ancienne version.", duration: "quelques secondes", needs_admin: false },
      { id: "restart_explorer", name: "Redémarrer l'Explorateur", detail: "Débloque la barre des tâches, le menu Démarrer ou le Bureau quand ils sont figés.", duration: "quelques secondes", needs_admin: false },
      { id: "refresh_icons", name: "Rafraîchir les icônes", detail: "Corrige les icônes blanches ou mauvaises sur le Bureau et dans l'Explorateur.", duration: "quelques secondes", needs_admin: false },
      { id: "reset_store", name: "Réparer le Microsoft Store", detail: "Vide le cache du Store quand les téléchargements ou les mises à jour d'applis bloquent.", duration: "quelques secondes", needs_admin: false },
    ]);
    case "run_maintenance": return wait(1500, { ok: true, message: "Cache DNS vidé." });
    default: return Promise.reject("Commande inconnue : " + cmd);
  }
}
