// Kaury Clean : modules Vieux téléchargements, Ranger, Mémoire, Désinstaller et Maintenance.
// Utilise les outils de main.js ($, invoke, esc, fmt, toast, progressEl, renderFileList…).

const adminLink = (text) =>
  `<div class="admin-note left above">${esc(text)}<button class="link" data-admin>${tr("Relancer en administrateur", "Relaunch as administrator")}</button></div>`;
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
  startSearch(box, tr("Recherche dans Téléchargements…", "Searching Downloads…"));
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
const FOLDER_NAMES = { downloads: tr("Téléchargements", "Downloads"), desktop: tr("Bureau", "Desktop") };
const CATEGORY_ICONS = { Images: "▣", Vidéos: "▶", Videos: "▶", Audio: "♪", Design: "✎", Documents: "≡", Archives: "▤", Installeurs: "↓", Installers: "↓", Code: "‹›" };

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
    box.innerHTML = `<div class="empty">${tr("Impossible de lire le dossier : ", "Couldn't read the folder: ")}${esc(e)}</div>`;
    return;
  }
  const undo = canUndo ? `<button class="cta ghost" data-undo>${tr("Annuler le dernier rangement", "Undo the last tidy-up")}</button>` : "";
  if (!plan.length) {
    box.innerHTML = `<div class="empty">${esc(FOLDER_NAMES[organizeFolder])} ${tr("est déjà bien rangé.", "is already tidy.")}${undo}</div>`;
  } else {
    const count = plan.reduce((a, g) => a + g.count, 0);
    box.innerHTML = `<div class="cards">${plan.map((g) => `
      <div class="card"><div class="card-top"><span class="ic">${esc(CATEGORY_ICONS[g.category] || "•")}</span><b>${esc(g.category)}</b><span class="s">${fmt(g.bytes)}</span></div>
        <div class="card-count">${g.count} ${tr("fichier", "file")}${g.count > 1 ? "s" : ""}</div>
        <div class="p">${g.examples.map(esc).join(" · ")}</div></div>`).join("")}</div>
      <div class="foot"><span>${tr("Dans", "In")} <b>${esc(FOLDER_NAMES[organizeFolder])}</b>${tr(", un sous-dossier par type.", ", one subfolder per type.")}</span>
        <span class="actions">${undo}<button class="cta small" data-apply>${tr("Ranger", "Tidy")} ${count} ${tr("fichier", "file")}${count > 1 ? "s" : ""}</button></span></div>`;
    $("[data-apply]", box).addEventListener("click", async (e) => {
      e.currentTarget.disabled = true;
      try {
        const r = await invoke("organize_apply", { folder: organizeFolder });
        toast(tr(`${r.moved} fichier${r.moved > 1 ? "s" : ""} rangé${r.moved > 1 ? "s" : ""}`, `${r.moved} file${r.moved > 1 ? "s" : ""} tidied`)
          + (r.errors.length ? tr(` · ${r.errors.length} laissé(s) en place`, ` · ${r.errors.length} left in place`) : ""));
      } catch (err) {
        toast(tr("Le rangement a échoué : ", "The tidy-up failed: ") + err);
      }
      loadOrganize();
    });
  }
  $("[data-undo]", box)?.addEventListener("click", async (e) => {
    e.currentTarget.disabled = true;
    try {
      const r = await invoke("organize_undo");
      toast(tr(`${r.moved} fichier${r.moved > 1 ? "s" : ""} remis à leur place`, `${r.moved} file${r.moved > 1 ? "s" : ""} put back`)
        + (r.errors.length ? tr(` · ${r.errors.length} impossible(s)`, ` · ${r.errors.length} couldn't be moved`) : ""));
    } catch (err) {
      toast(tr("Impossible d'annuler : ", "Couldn't undo: ") + err);
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
    box.innerHTML = `<div class="empty">${tr("Impossible de lire la mémoire : ", "Couldn't read memory: ")}${esc(e)}</div>`;
    return;
  }
  const pct = Math.round((m.used / m.total) * 100);
  const level = pct >= 85 ? "high" : pct >= 70 ? "warn" : "ok";
  const verdict = EN
    ? { high: "Memory almost full: close one or two apps.", warn: "Memory is quite full.", ok: "All good, there's room left." }[level]
    : { high: "Mémoire presque pleine : ferme une ou deux applis.", warn: "Mémoire bien remplie.", ok: "Tout va bien, il reste de la place." }[level];
  box.innerHTML = `
    <div class="meter ${level}">
      <div class="meter-top"><b>${pct}${EN ? "" : " "}%</b><span>${tr(`${fmt(m.used)} utilisés sur ${fmt(m.total)}`, `${fmt(m.used)} used of ${fmt(m.total)}`)}</span></div>
      <div class="bar"><i style="width:${pct}%"></i></div>
      <div class="meter-note">${esc(verdict)}</div>
    </div>
    ${m.apps.length ? `<div class="list">${m.apps.map((a) => `
      <div class="item" data-exe="${esc(a.exe)}"><span class="ic">${esc(a.name[0] || "?")}</span>
        <span class="txt"><div class="n">${esc(a.name)}</div><div class="p">${a.processes > 1 ? `${a.processes} ${tr("processus", "processes")} · ` : ""}${esc(a.exe)}</div></span>
        <span class="end"><span class="s">${fmt(a.bytes)}</span><button class="cta ghost tiny" data-close>${tr("Fermer", "Close")}</button></span></div>`).join("")}</div>`
      : `<div class="empty">${tr("Aucune appli ne prend beaucoup de mémoire.", "No app is using much memory.")}</div>`}`;

  $$("[data-close]", box).forEach((b) => b.addEventListener("click", async () => {
    const row = b.closest("[data-exe]");
    const exe = row.dataset.exe;
    const force = b.dataset.force === "1";
    memoryBusy = true;
    b.disabled = true;
    b.textContent = tr("Fermeture…", "Closing…");
    try {
      const closed = await invoke("close_app", { exe, force });
      if (closed) {
        toast(`${$(".n", row).textContent} ${tr("fermé", "closed")}`);
        memoryBusy = false;
        return loadMemory();
      }
      // On garde le bouton « Forcer » affiché : pas d'actualisation automatique pendant ce temps.
      // L'appli n'a pas voulu se fermer (fenêtre « Enregistrer ? » ouverte, ou appli bloquée).
      b.dataset.force = "1";
      b.textContent = tr("Forcer la fermeture", "Force close");
      b.classList.add("danger");
      b.disabled = false;
      toast(tr("L'appli ne s'est pas fermée. Vérifie qu'elle ne te demande pas d'enregistrer, ou force la fermeture.", "The app didn't close. Check it isn't asking you to save, or force it to close."));
    } catch (e) {
      toast(String(e));
      b.disabled = false;
      b.textContent = tr("Fermer", "Close");
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
  $("#uninstallBody").innerHTML = `<div class="empty"><div class="spinner"></div>${tr("Lecture des applis installées…", "Reading installed apps…")}</div>`;
  try {
    installed = await invoke("list_installed_apps");
  } catch (e) {
    $("#uninstallBody").innerHTML = `<div class="empty">${tr("Impossible de lire la liste : ", "Couldn't read the list: ")}${esc(e)}</div>`;
    return;
  }
  renderInstalled();
}

function installedDate(s) {
  if (!/^\d{8}$/.test(s)) return "";
  const d = new Date(Number(s.slice(0, 4)), Number(s.slice(4, 6)) - 1, Number(s.slice(6, 8)));
  return tr("installée le ", "installed ") + d.toLocaleDateString(LOCALE, { day: "numeric", month: "short", year: "numeric" });
}

function renderInstalled() {
  if (!installed) return;
  const q = $("#uninstallSearch").value.trim().toLowerCase();
  const sort = $("#uninstallSort").value;
  const apps = installed
    .filter((a) => !q || a.name.toLowerCase().includes(q) || a.publisher.toLowerCase().includes(q))
    .sort((a, b) => sort === "bytes" ? b.bytes - a.bytes : sort === "installed" ? b.installed.localeCompare(a.installed) : a.name.localeCompare(b.name, LANG));
  const total = installed.reduce((s, a) => s + a.bytes, 0);
  $("#uninstallBody").innerHTML = apps.length ? `<div class="list">${apps.map((a) => `
    <div class="item"><span class="ic">${esc(a.name[0] || "?")}</span>
      <span class="txt"><div class="n">${esc(a.name)}</div>
        <div class="p">${[a.publisher, a.version && "v" + a.version, installedDate(a.installed)].filter(Boolean).map(esc).join(" · ")}</div></span>
      <span class="end"><span class="s">${a.bytes ? fmt(a.bytes) : "—"}</span><button class="cta ghost tiny" data-uninstall="${esc(a.id)}">${tr("Désinstaller", "Uninstall")}</button></span></div>`).join("")}</div>
    <div class="foot"><span>${installed.length} ${tr("applis", "apps")} · <b>${fmt(total)}</b> ${tr("au total", "in total")}</span><button class="cta ghost" data-refresh>${tr("Actualiser la liste", "Refresh the list")}</button></div>`
    : `<div class="empty">${tr("Aucune appli ne correspond.", "No matching app.")}</div>`;
  $$("[data-uninstall]", $("#uninstallBody")).forEach((b) => b.addEventListener("click", async () => {
    const name = $(".n", b.closest(".item")).textContent;
    try {
      await invoke("uninstall_app", { id: b.dataset.uninstall });
      toast(tr(`Le désinstalleur de ${name} est ouvert. Suis ses étapes, puis actualise la liste.`, `The ${name} uninstaller is open. Follow its steps, then refresh the list.`));
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
  box.innerHTML = `${blocked ? adminLink(tr("Les tâches « admin » ont besoin des droits administrateur.", "\"admin\" tasks need admin rights.")) : ""}
    <div class="list">${tasks.map((t) => `
    <div class="item task" data-task="${esc(t.id)}">
      <span class="txt"><div class="n">${esc(t.name)}${t.needs_admin ? `<span class="chip">admin</span>` : ""}<span class="chip">${esc(t.duration)}</span></div>
        <div class="d">${esc(t.detail)}</div>
        <div class="result" hidden></div></span>
      <span class="end"><button class="cta small" data-run ${t.needs_admin && !elevated ? "disabled" : ""}>${tr("Lancer", "Run")}</button></span></div>`).join("")}</div>`;
  bindAdmin(box);

  $$("[data-run]", box).forEach((b) => b.addEventListener("click", async () => {
    if (maintenanceRunning) return toast(tr("Une tâche est déjà en cours.", "A task is already running."));
    maintenanceRunning = true;
    const row = b.closest("[data-task]");
    const result = $(".result", row);
    $$("[data-run]", box).forEach((x) => (x.disabled = true));
    b.textContent = tr("En cours…", "Running…");
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
    b.textContent = tr("Relancer", "Run again");
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
      <p class="lead">Version ${esc(appVersion || "—")}${elevated ? tr(" · administrateur", " · administrator") : ""}</p>
      <p class="about-text">${EN
        ? "A simple, beautiful PC cleaner, designed and built by <b>Kaury Studio</b>, a creative studio in Vevey, Switzerland: visual identity, websites, video, photo and social media."
        : "Un nettoyeur de PC simple et beau, conçu et développé par <b>Kaury Studio</b>, studio créatif à Vevey : identité visuelle, sites web, vidéo, photo et réseaux sociaux."}</p>
      <div class="actions">
        <button class="cta" data-link="site">${tr("Visiter kaury.studio", "Visit kaury.studio")}</button>
        <button class="cta ghost" data-link="behance">Behance</button>
        <button class="cta ghost" data-link="instagram">Instagram</button>
        <button class="cta ghost" data-link="email">hello@kaury.studio</button>
      </div>
      <div class="seg lang-switch" role="radiogroup" aria-label="${tr("Langue", "Language")}">
        <button role="radio" aria-checked="${!EN}" data-lang="fr">Français</button>
        <button role="radio" aria-checked="${EN}" data-lang="en">English</button>
      </div>
    </div>
    <div class="update-card" id="updateCard"></div>`;
  paintIcons(el);
  $$("[data-link]", el).forEach((b) => b.addEventListener("click", () => openLink(b.dataset.link)));
  // Changer de langue recharge l'interface : tout se réécrit d'un coup, sans état à moitié traduit.
  $$("[data-lang]", el).forEach((b) => b.addEventListener("click", () => {
    if (b.dataset.lang === LANG) return;
    try { localStorage.setItem(LANG_KEY, b.dataset.lang); } catch { /* stockage indisponible */ }
    location.reload();
  }));
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
    body = `<div class="uc-text"><b>${tr("Mise à jour en cours", "Updating")}</b><span class="progress" id="updateProgress">${tr("Téléchargement…", "Downloading…")}</span></div><span class="spinner"></span>`;
  } else if (u && u.available) {
    body = `<div class="uc-text"><b>${tr(`Kaury Clean ${esc(u.latest)} est disponible`, `Kaury Clean ${esc(u.latest)} is available`)}</b><span>${EN
      ? `You have version ${esc(u.current)}${u.size ? ` · ${fmt(u.size)} to download` : ""}. The installer replaces the old version, your settings are kept.`
      : `Tu as la version ${esc(u.current)}${u.size ? ` · ${fmt(u.size)} à télécharger` : ""}. L'installeur remplace l'ancienne version, tes réglages sont gardés.`}</span></div>
      <div class="uc-actions"><button class="cta ghost small" data-link="releases">${tr("Nouveautés", "What's new")}</button><button class="cta small" data-install>${tr("Mettre à jour", "Update")}</button></div>`;
  } else {
    body = `<div class="uc-text"><b>${esc(message || (u ? tr("Tu as la dernière version", "You have the latest version") : tr("Mises à jour", "Updates")))}</b><span>${u ? `Version ${esc(u.current)}.` : tr("Kaury Clean vérifie les nouvelles versions à chaque ouverture.", "Kaury Clean checks for new versions each time it opens.")}</span></div>
      <div class="uc-actions"><button class="cta ghost small" data-check>${tr("Vérifier maintenant", "Check now")}</button></div>`;
  }
  box.innerHTML = body;
  $("[data-install]", box)?.addEventListener("click", installUpdate);
  $("[data-link]", box)?.addEventListener("click", () => openLink("releases"));
  $("[data-check]", box)?.addEventListener("click", async (e) => {
    e.currentTarget.disabled = true;
    e.currentTarget.textContent = tr("Vérification…", "Checking…");
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
  $("#updateModalTitle").textContent = tr(`Kaury Clean ${updateInfo.latest} est disponible`, `Kaury Clean ${updateInfo.latest} is available`);
  const size = updateInfo.size ? ` (${fmt(updateInfo.size)})` : "";
  $("#updateModalText").textContent = tr(
    `Tu as la version ${updateInfo.current}. La mise à jour se télécharge${size} et s'installe par-dessus : tes réglages sont gardés.`,
    `You have version ${updateInfo.current}. The update downloads${size} and installs on top: your settings are kept.`);
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
  banner.innerHTML = `<span class="tile sm t-home" data-icon="sparkle"></span><span><b>${tr(`Kaury Clean ${esc(updateInfo.latest)} est disponible.`, `Kaury Clean ${esc(updateInfo.latest)} is available.`)}</b> ${tr("Une nouvelle version avec des améliorations.", "A new version with improvements.")}</span><button class="cta small" data-install>${tr("Mettre à jour", "Update")}</button><button class="link" data-later>${tr("Plus tard", "Later")}</button>`;
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
    renderUpdateCard(tr("La mise à jour n'a pas pu se faire : ", "The update couldn't be done: ") + e);
  }
}

// ---------- Place du disque ----------
// Lecture seule : une carte de ce qui prend de la place. Aucun bouton n'efface quoi que ce soit.
const SPACE_KINDS = EN
  ? { files: "Your files", apps: "App data", programs: "Programs", system: "Windows and the rest" }
  : { files: "Tes fichiers", apps: "Données d'applis", programs: "Programmes", system: "Windows et le reste" };
$("#spaceBtn").addEventListener("click", loadSpace);

async function loadSpace() {
  const box = $("#spaceBody");
  startSearch(box, tr("Mesure de ton disque… Ça peut prendre une ou deux minutes.", "Measuring your drive… This can take a minute or two."));
  $("#spaceBtn").disabled = true;
  try {
    const r = await invoke("disk_usage");
    progressEl = null;
    renderSpace(box, r);
  } catch (e) {
    searchFailed(box, e);
  }
  $("#spaceBtn").disabled = false;
}

function renderSpace(box, r) {
  const used = r.used || r.groups.reduce((a, g) => a + g.bytes, 0);
  const pct = (b) => (used ? (b / used) * 100 : 0);
  const max = r.top.length ? r.top[0].bytes : 1;
  const initial = (name) => (name.replace(/^\./, "")[0] || "?").toUpperCase();
  box.innerHTML = `
    <div class="space-map">
      <div class="space-top"><b>${fmt(used)}</b><span>${tr("utilisés", "used")}${r.total ? ` ${tr("sur", "of")} ${fmt(r.total)}` : ""}</span></div>
      <div class="space-bar">${r.groups.filter((g) => g.bytes > 0).map((g) =>
        `<i class="k-${esc(g.kind)}" style="width:${pct(g.bytes).toFixed(2)}%" title="${esc(g.label)} · ${fmt(g.bytes)}"></i>`).join("")}</div>
      <div class="space-legend">${r.groups.map((g) => `<span><i class="k-${esc(g.kind)}"></i>${esc(g.label)}<b>${fmt(g.bytes)}</b></span>`).join("")}</div>
    </div>
    ${r.top.length ? `<div class="list">${r.top.map((e) => `
      <div class="item"><span class="ic k-${esc(e.kind)}">${esc(initial(e.name))}</span>
        <span class="txt"><div class="n">${esc(e.name)}<span class="chip">${esc(SPACE_KINDS[e.kind] || "")}</span></div>
          <div class="p">${esc(e.note)}</div>
          <div class="bar thin"><i style="width:${((e.bytes / max) * 100).toFixed(1)}%"></i></div></span>
        <span class="end"><button class="reveal" data-open="${esc(e.path)}" title="${tr("Ouvrir dans l'Explorateur", "Open in Explorer")}">${tr("Ouvrir", "Open")}</button><span class="s">${fmt(e.bytes)}</span></span></div>`).join("")}</div>
    <div class="foot"><span>${tr(`Les ${r.top.length} plus gros dossiers de ton compte, des données d'applis et des programmes. Rien n'est effacé d'ici.`, `The ${r.top.length} largest folders in your account, app data and programs. Nothing is deleted from here.`)}</span></div>`
    : `<div class="empty">${tr("Aucun dossier lisible.", "No readable folder.")}</div>`}`;
  $$("[data-open]", box).forEach((b) => b.addEventListener("click", () => {
    invoke("open_folder", { path: b.dataset.open }).catch((err) => toast(String(err)));
  }));
}
