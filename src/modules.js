// Kaury Clean : modules Vieux téléchargements, Ranger, Mémoire, Désinstaller et Maintenance.
// Utilise les outils de main.js ($, invoke, esc, fmt, toast, progressEl, renderFileList…).

const adminLink = (text) =>
  `<div class="admin-note left above">${esc(text)}<button class="link" data-admin>Relancer en administrateur</button></div>`;
const bindAdmin = (root) => $$("[data-admin]", root).forEach((b) => b.addEventListener("click", relaunchAsAdmin));

// ---------- Vieux téléchargements ----------
$("#olddlBtn").addEventListener("click", loadOldDownloads);
// L'entretien intelligent a déjà fait la recherche : on affiche son résultat sans relancer.
function showOldDownloads(files) {
  $("#olddlMin").value = "180";
  renderFileList($("#olddlList"), files.length ? [{ files }] : [], { preselect: false, onDone: loadOldDownloads });
}
async function loadOldDownloads() {
  const box = $("#olddlList");
  startSearch(box, "Recherche dans Téléchargements…");
  $("#olddlBtn").disabled = true;
  try {
    const files = await invoke("find_old_downloads", { minDays: Number($("#olddlMin").value) });
    progressEl = null;
    renderFileList(box, files.length ? [{ files }] : [], { preselect: false, onDone: loadOldDownloads });
  } catch (e) {
    searchFailed(box, e);
  }
  $("#olddlBtn").disabled = false;
}

// ---------- Ranger mes fichiers ----------
let organizeFolder = "downloads";
const FOLDER_NAMES = { downloads: "Téléchargements", desktop: "Bureau" };
const CATEGORY_ICONS = { Images: "▣", Vidéos: "▶", Audio: "♪", Design: "✎", Documents: "≡", Archives: "▤", Installeurs: "↓", Code: "‹›" };

$$("#v-organize [data-folder]").forEach((b) => b.addEventListener("click", () => {
  organizeFolder = b.dataset.folder;
  $$("#v-organize [data-folder]").forEach((x) => x.setAttribute("aria-checked", String(x === b)));
  loadOrganize();
}));
viewLoaders.organize = () => loadOrganize();

async function loadOrganize() {
  const box = $("#organizeBody");
  let plan, canUndo;
  try {
    [plan, canUndo] = await Promise.all([invoke("organize_plan", { folder: organizeFolder }), invoke("organize_can_undo")]);
  } catch (e) {
    box.innerHTML = `<div class="empty">Impossible de lire le dossier : ${esc(e)}</div>`;
    return;
  }
  const undo = canUndo ? `<button class="cta ghost" data-undo>Annuler le dernier rangement</button>` : "";
  if (!plan.length) {
    box.innerHTML = `<div class="empty">${esc(FOLDER_NAMES[organizeFolder])} est déjà bien rangé.${undo}</div>`;
  } else {
    const count = plan.reduce((a, g) => a + g.count, 0);
    box.innerHTML = `<div class="cards">${plan.map((g) => `
      <div class="card"><div class="card-top"><span class="ic">${esc(CATEGORY_ICONS[g.category] || "•")}</span><b>${esc(g.category)}</b><span class="s">${fmt(g.bytes)}</span></div>
        <div class="card-count">${g.count} fichier${g.count > 1 ? "s" : ""}</div>
        <div class="p">${g.examples.map(esc).join(" · ")}</div></div>`).join("")}</div>
      <div class="foot"><span>Dans <b>${esc(FOLDER_NAMES[organizeFolder])}</b>, un sous-dossier par type.</span>
        <span class="actions">${undo}<button class="cta small" data-apply>Ranger ${count} fichier${count > 1 ? "s" : ""}</button></span></div>`;
    $("[data-apply]", box).addEventListener("click", async (e) => {
      e.currentTarget.disabled = true;
      try {
        const r = await invoke("organize_apply", { folder: organizeFolder });
        toast(`${r.moved} fichier${r.moved > 1 ? "s" : ""} rangé${r.moved > 1 ? "s" : ""}` + (r.errors.length ? ` · ${r.errors.length} laissé(s) en place` : ""));
      } catch (err) {
        toast("Le rangement a échoué : " + err);
      }
      loadOrganize();
    });
  }
  $("[data-undo]", box)?.addEventListener("click", async (e) => {
    e.currentTarget.disabled = true;
    try {
      const r = await invoke("organize_undo");
      toast(`${r.moved} fichier${r.moved > 1 ? "s" : ""} remis à leur place` + (r.errors.length ? ` · ${r.errors.length} impossible(s)` : ""));
    } catch (err) {
      toast("Impossible d'annuler : " + err);
    }
    loadOrganize();
  });
}

// ---------- Mémoire vive ----------
// La jauge se met à jour toute seule tant que l'écran Mémoire est ouvert.
let memoryTimer = null;
let memoryBusy = false;
viewLoaders.memory = () => {
  loadMemory();
  if (memoryTimer) return;
  memoryTimer = setInterval(() => {
    if ($("#v-memory").hidden) {
      clearInterval(memoryTimer);
      memoryTimer = null;
    } else if (!memoryBusy) {
      loadMemory();
    }
  }, 4000);
};
$("#memoryBtn").addEventListener("click", () => loadMemory());

async function loadMemory() {
  const box = $("#memoryBody");
  if (!box.innerHTML) box.innerHTML = `<div class="empty"><div class="spinner"></div></div>`;
  let m;
  try {
    m = await invoke("memory_status");
  } catch (e) {
    box.innerHTML = `<div class="empty">Impossible de lire la mémoire : ${esc(e)}</div>`;
    return;
  }
  const pct = Math.round((m.used / m.total) * 100);
  const level = pct >= 85 ? "high" : pct >= 70 ? "warn" : "ok";
  const verdict = { high: "Mémoire presque pleine : ferme une ou deux applis.", warn: "Mémoire bien remplie.", ok: "Tout va bien, il reste de la place." }[level];
  box.innerHTML = `
    <div class="meter ${level}">
      <div class="meter-top"><b>${pct} %</b><span>${fmt(m.used)} utilisés sur ${fmt(m.total)}</span></div>
      <div class="bar"><i style="width:${pct}%"></i></div>
      <div class="meter-note">${esc(verdict)}</div>
    </div>
    ${m.apps.length ? `<div class="list">${m.apps.map((a) => `
      <div class="item" data-exe="${esc(a.exe)}"><span class="ic">${esc(a.name[0] || "?")}</span>
        <span class="txt"><div class="n">${esc(a.name)}</div><div class="p">${a.processes > 1 ? `${a.processes} processus · ` : ""}${esc(a.exe)}</div></span>
        <span class="end"><span class="s">${fmt(a.bytes)}</span><button class="cta ghost tiny" data-close>Fermer</button></span></div>`).join("")}</div>`
      : `<div class="empty">Aucune appli ne prend beaucoup de mémoire.</div>`}`;

  $$("[data-close]", box).forEach((b) => b.addEventListener("click", async () => {
    const row = b.closest("[data-exe]");
    const exe = row.dataset.exe;
    const force = b.dataset.force === "1";
    memoryBusy = true;
    b.disabled = true;
    b.textContent = "Fermeture…";
    try {
      const closed = await invoke("close_app", { exe, force });
      if (closed) {
        toast(`${$(".n", row).textContent} fermé`);
        memoryBusy = false;
        return loadMemory();
      }
      // On garde le bouton « Forcer » affiché : pas d'actualisation automatique pendant ce temps.
      // L'appli n'a pas voulu se fermer (fenêtre « Enregistrer ? » ouverte, ou appli bloquée).
      b.dataset.force = "1";
      b.textContent = "Forcer la fermeture";
      b.classList.add("danger");
      b.disabled = false;
      toast("L'appli ne s'est pas fermée. Vérifie qu'elle ne te demande pas d'enregistrer, ou force la fermeture.");
    } catch (e) {
      toast(String(e));
      b.disabled = false;
      b.textContent = "Fermer";
      memoryBusy = false;
    }
  }));
}

// ---------- Désinstaller ----------
let installed = null;
viewLoaders.uninstall = () => { if (!installed) loadInstalled(); };
$("#uninstallSearch").addEventListener("input", renderInstalled);
$("#uninstallSort").addEventListener("change", renderInstalled);

async function loadInstalled() {
  $("#uninstallBody").innerHTML = `<div class="empty"><div class="spinner"></div>Lecture des applis installées…</div>`;
  try {
    installed = await invoke("list_installed_apps");
  } catch (e) {
    $("#uninstallBody").innerHTML = `<div class="empty">Impossible de lire la liste : ${esc(e)}</div>`;
    return;
  }
  renderInstalled();
}

function installedDate(s) {
  if (!/^\d{8}$/.test(s)) return "";
  const d = new Date(Number(s.slice(0, 4)), Number(s.slice(4, 6)) - 1, Number(s.slice(6, 8)));
  return "installée le " + d.toLocaleDateString("fr-CH", { day: "numeric", month: "short", year: "numeric" });
}

function renderInstalled() {
  if (!installed) return;
  const q = $("#uninstallSearch").value.trim().toLowerCase();
  const sort = $("#uninstallSort").value;
  const apps = installed
    .filter((a) => !q || a.name.toLowerCase().includes(q) || a.publisher.toLowerCase().includes(q))
    .sort((a, b) => sort === "bytes" ? b.bytes - a.bytes : sort === "installed" ? b.installed.localeCompare(a.installed) : a.name.localeCompare(b.name, "fr"));
  const total = installed.reduce((s, a) => s + a.bytes, 0);
  $("#uninstallBody").innerHTML = apps.length ? `<div class="list">${apps.map((a) => `
    <div class="item"><span class="ic">${esc(a.name[0] || "?")}</span>
      <span class="txt"><div class="n">${esc(a.name)}</div>
        <div class="p">${[a.publisher, a.version && "v" + a.version, installedDate(a.installed)].filter(Boolean).map(esc).join(" · ")}</div></span>
      <span class="end"><span class="s">${a.bytes ? fmt(a.bytes) : "—"}</span><button class="cta ghost tiny" data-uninstall="${esc(a.id)}">Désinstaller</button></span></div>`).join("")}</div>
    <div class="foot"><span>${installed.length} applis · <b>${fmt(total)}</b> au total</span><button class="cta ghost" data-refresh>Actualiser la liste</button></div>`
    : `<div class="empty">Aucune appli ne correspond.</div>`;
  $$("[data-uninstall]", $("#uninstallBody")).forEach((b) => b.addEventListener("click", async () => {
    const name = $(".n", b.closest(".item")).textContent;
    try {
      await invoke("uninstall_app", { id: b.dataset.uninstall });
      toast(`Le désinstalleur de ${name} est ouvert. Suis ses étapes, puis actualise la liste.`);
    } catch (e) {
      toast(String(e));
    }
  }));
  $("[data-refresh]", $("#uninstallBody"))?.addEventListener("click", loadInstalled);
}

// ---------- Maintenance ----------
let maintenanceRunning = false;
viewLoaders.maintenance = () => { if (!$("#maintenanceBody").innerHTML) loadMaintenance(); };

async function loadMaintenance() {
  const box = $("#maintenanceBody");
  let tasks;
  try {
    tasks = await invoke("list_maintenance");
  } catch (e) {
    box.innerHTML = `<div class="empty">${esc(e)}</div>`;
    return;
  }
  const blocked = tasks.some((t) => t.needs_admin) && !elevated;
  box.innerHTML = `${blocked ? adminLink("Les tâches « admin » ont besoin des droits administrateur.") : ""}
    <div class="list">${tasks.map((t) => `
    <div class="item task" data-task="${esc(t.id)}">
      <span class="txt"><div class="n">${esc(t.name)}${t.needs_admin ? `<span class="chip">admin</span>` : ""}<span class="chip">${esc(t.duration)}</span></div>
        <div class="d">${esc(t.detail)}</div>
        <div class="result" hidden></div></span>
      <span class="end"><button class="cta small" data-run ${t.needs_admin && !elevated ? "disabled" : ""}>Lancer</button></span></div>`).join("")}</div>`;
  bindAdmin(box);

  $$("[data-run]", box).forEach((b) => b.addEventListener("click", async () => {
    if (maintenanceRunning) return toast("Une tâche est déjà en cours.");
    maintenanceRunning = true;
    const row = b.closest("[data-task]");
    const result = $(".result", row);
    $$("[data-run]", box).forEach((x) => (x.disabled = true));
    b.textContent = "En cours…";
    result.hidden = false;
    result.className = "result";
    result.innerHTML = `<span class="spinner small"></span><span class="progress"></span><span class="elapsed"></span>`;
    progressEl = $(".progress", result);
    const t0 = Date.now();
    const timer = setInterval(() => {
      const s = Math.round((Date.now() - t0) / 1000);
      $(".elapsed", result).textContent = ` · ${Math.floor(s / 60)} min ${String(s % 60).padStart(2, "0")} s`;
    }, 1000);
    try {
      const r = await invoke("run_maintenance", { id: row.dataset.task });
      result.className = "result " + (r.ok ? "ok" : "bad");
      result.textContent = (r.ok ? "✓ " : "✕ ") + r.message;
    } catch (e) {
      result.className = "result bad";
      result.textContent = "✕ " + e;
    }
    clearInterval(timer);
    progressEl = null;
    maintenanceRunning = false;
    b.textContent = "Relancer";
    $$("[data-run]", box).forEach((x) => (x.disabled = false));
    loadMaintenanceAdminState(box);
  }));
}

// Après une tâche, on remet les boutons « admin » dans leur état bloqué si besoin.
function loadMaintenanceAdminState(box) {
  if (elevated) return;
  $$("[data-task]", box).forEach((row) => {
    if ($(".chip", row)?.textContent === "admin") $("[data-run]", row).disabled = true;
  });
}

// ---------- À propos et mises à jour ----------
let updateInfo = null;
let updating = false;

viewLoaders.about = () => renderAbout();

function renderAbout() {
  const el = $("#v-about");
  el.innerHTML = `
    <div class="about-hero">
      <div class="hero-art small" aria-hidden="true"><div class="float"><div class="tile3d"><span data-icon="logo-big"></span></div></div><div class="floor"></div></div>
      <h1>Kaury Clean</h1>
      <p class="lead">Version ${esc(appVersion || "—")}${elevated ? " · administrateur" : ""}</p>
      <p class="about-text">Un nettoyeur de PC simple et beau, conçu et développé par <b>Kaury Studio</b>, studio créatif à Vevey : identité visuelle, sites web, vidéo, photo et réseaux sociaux.</p>
      <div class="actions">
        <button class="cta" data-link="site">Visiter kaury.studio</button>
        <button class="cta ghost" data-link="behance">Behance</button>
        <button class="cta ghost" data-link="instagram">Instagram</button>
        <button class="cta ghost" data-link="email">hello@kaury.studio</button>
      </div>
    </div>
    <div class="update-card" id="updateCard"></div>`;
  paintIcons(el);
  $$("[data-link]", el).forEach((b) => b.addEventListener("click", () => openLink(b.dataset.link)));
  renderUpdateCard();
}

function openLink(link) {
  invoke("open_link", { link }).catch((e) => toast(String(e)));
}

function renderUpdateCard(message) {
  const box = $("#updateCard");
  if (!box) return;
  const u = updateInfo;
  let body;
  if (updating) {
    body = `<div class="uc-text"><b>Mise à jour en cours</b><span class="progress" id="updateProgress">Téléchargement…</span></div><span class="spinner"></span>`;
  } else if (u && u.available) {
    body = `<div class="uc-text"><b>Kaury Clean ${esc(u.latest)} est disponible</b><span>Tu as la version ${esc(u.current)}${u.size ? ` · ${fmt(u.size)} à télécharger` : ""}. L'installeur remplace l'ancienne version, tes réglages sont gardés.</span></div>
      <div class="uc-actions"><button class="cta ghost small" data-link="releases">Nouveautés</button><button class="cta small" data-install>Mettre à jour</button></div>`;
  } else {
    body = `<div class="uc-text"><b>${esc(message || (u ? "Tu as la dernière version" : "Mises à jour"))}</b><span>${u ? `Version ${esc(u.current)}.` : "Kaury Clean vérifie les nouvelles versions à chaque ouverture."}</span></div>
      <div class="uc-actions"><button class="cta ghost small" data-check>Vérifier maintenant</button></div>`;
  }
  box.innerHTML = body;
  $("[data-install]", box)?.addEventListener("click", installUpdate);
  $("[data-link]", box)?.addEventListener("click", () => openLink("releases"));
  $("[data-check]", box)?.addEventListener("click", async (e) => {
    e.currentTarget.disabled = true;
    e.currentTarget.textContent = "Vérification…";
    try {
      updateInfo = await invoke("check_update");
      renderUpdateCard();
      renderUpdateBanner();
    } catch (err) {
      renderUpdateCard(String(err));
    }
  });
}

// Au démarrage : on regarde discrètement s'il y a une nouvelle version. Pas de connexion, pas de message.
async function checkUpdateQuietly() {
  try {
    updateInfo = await invoke("check_update");
  } catch {
    return;
  }
  renderUpdateBanner();
  if ($("#updateCard")) renderUpdateCard();
  proposeUpdate();
}

// Une fenêtre propose la nouvelle version à l'ouverture. « Plus tard » ne la remontre pas pour cette
// version-là (le bandeau et la pastille restent).
const SKIP_KEY = "kaury-clean-update-later";
function proposeUpdate() {
  if (!updateInfo || !updateInfo.available) return;
  let skipped = null;
  try { skipped = localStorage.getItem(SKIP_KEY); } catch { /* stockage indisponible */ }
  if (skipped === updateInfo.latest) return;
  const modal = $("#updateModal");
  $("#updateModalTitle").textContent = `Kaury Clean ${updateInfo.latest} est disponible`;
  $("#updateModalText").textContent = `Tu as la version ${updateInfo.current}. La mise à jour se télécharge${updateInfo.size ? ` (${fmt(updateInfo.size)})` : ""} et s'installe par-dessus : tes réglages sont gardés.`;
  paintIcons(modal);
  modal.hidden = false;
  $("#updateNow").focus();
}
$("#updateLater").addEventListener("click", () => {
  $("#updateModal").hidden = true;
  try { localStorage.setItem(SKIP_KEY, updateInfo.latest); } catch { /* stockage indisponible */ }
});
$("#updateNow").addEventListener("click", () => {
  $("#updateModal").hidden = true;
  show("about");
  installUpdate();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$("#updateModal").hidden) $("#updateLater").click();
});

function renderUpdateBanner() {
  const available = !!(updateInfo && updateInfo.available);
  $("#updateDot").hidden = !available;
  const banner = $("#updateBanner");
  banner.hidden = !available;
  if (!available) return;
  banner.innerHTML = `<span class="tile sm t-home" data-icon="sparkle"></span><span><b>Kaury Clean ${esc(updateInfo.latest)} est disponible.</b> Une nouvelle version avec des améliorations.</span><button class="cta small" data-install>Mettre à jour</button><button class="link" data-later>Plus tard</button>`;
  paintIcons(banner);
  $("[data-install]", banner).addEventListener("click", () => { show("about"); installUpdate(); });
  $("[data-later]", banner).addEventListener("click", () => (banner.hidden = true));
}

async function installUpdate() {
  if (updating) return;
  updating = true;
  if (!$("#updateCard")) show("about");
  renderUpdateCard();
  progressEl = $("#updateProgress");
  try {
    await invoke("install_update");
    // L'installeur est lancé : l'appli se ferme toute seule.
  } catch (e) {
    updating = false;
    progressEl = null;
    renderUpdateCard("La mise à jour n'a pas pu se faire : " + e);
  }
}
