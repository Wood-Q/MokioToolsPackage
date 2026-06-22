// Mokio desktop front-end. Talks to the Rust back-end via Tauri commands/events.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

/** @type {Array<{info: any, status: any}>} */
let tools = [];
const selected = new Set();
let running = false;
let total = 0;

const $ = (id) => document.getElementById(id);
const cardsEl = $("cards");
const logEl = $("log");
const summaryEl = $("summary");

// The core `Status` enum serializes (externally-tagged, snake_case) as:
//   "unknown" | "not_installed" | {"installed": {version} | null}
function statusTag(status) {
  if (typeof status === "string") return status;
  if (status && typeof status === "object") return Object.keys(status)[0];
  return "unknown";
}
function statusVersion(status) {
  if (status && typeof status === "object" && status.installed) {
    return status.installed.version || null;
  }
  return null;
}

function statusBadge(status) {
  const tag = statusTag(status);
  if (tag === "installed") {
    const v = statusVersion(status);
    return { cls: "installed", text: "installed" + (v ? ` · ${v}` : "") };
  }
  if (tag === "not_installed") return { cls: "missing", text: "not installed" };
  return { cls: "", text: "unknown" };
}

function summaryText() {
  const installed = tools.filter((t) => statusTag(t.status) === "installed").length;
  return `${installed}/${tools.length} installed · ${selected.size} selected`;
}

function render() {
  cardsEl.innerHTML = "";
  for (const t of tools) {
    const { info, status } = t;
    const badge = statusBadge(status);
    const isSel = selected.has(info.id);
    const card = document.createElement("div");
    card.className = "card" + (isSel ? " selected" : "");
    card.dataset.id = info.id;

    const requires = info.requires && info.requires.length
      ? `needs: ${info.requires.join(", ")}`
      : (info.id === "homebrew" ? "foundation" : "");

    card.innerHTML = `
      <div class="card-row">
        <div class="check"></div>
        <span class="card-name">${escapeHtml(info.name)}</span>
        <span class="card-cat">${info.category}</span>
      </div>
      <div class="card-desc">${escapeHtml(info.description)}</div>
      <div class="card-foot">
        <span class="badge ${badge.cls}">${badge.text}</span>
        <a class="card-home" data-url="${escapeHtml(info.homepage)}" href="#">homepage ↗</a>
      </div>
      <div class="card-foot"><span>${requires}</span><span></span></div>
    `;

    card.addEventListener("click", (e) => {
      if (e.target.classList.contains("card-home")) {
        e.preventDefault();
        invoke("open_url", { url: e.target.dataset.url });
        return;
      }
      if (running) return;
      if (info.id === "homebrew") return; // foundation is locked on
      if (selected.has(info.id)) selected.delete(info.id);
      else selected.add(info.id);
      render();
    });
    cardsEl.appendChild(card);
  }
  summaryEl.textContent = summaryText();
  $("install").disabled = running || selected.size === 0;
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  }[c]));
}

function setCardStatus(id, status) {
  const t = tools.find((x) => x.info.id === id);
  if (t) t.status = status;
  // update just that card's badge without full re-render
  const card = cardsEl.querySelector(`.card[data-id="${id}"]`);
  if (card) {
    const badge = statusBadge(status);
    const b = card.querySelector(".badge");
    b.className = "badge " + badge.cls;
    b.textContent = badge.text;
  }
  summaryEl.textContent = summaryText();
}

function setCurrent(id) {
  for (const c of cardsEl.querySelectorAll(".card")) c.classList.remove("running");
  if (id) {
    const card = cardsEl.querySelector(`.card[data-id="${id}"]`);
    if (card) card.classList.add("running");
  }
}

function appendLog(level, text) {
  const line = document.createElement("div");
  line.className = "line " + (level || "info");
  line.textContent = prefixFor(level) + text;
  logEl.appendChild(line);
  logEl.scrollTop = logEl.scrollHeight;
  // cap log length
  while (logEl.childNodes.length > 2000) logEl.removeChild(logEl.firstChild);
}
function prefixFor(level) {
  switch (level) {
    case "phase": return "▶ ";
    case "warn": return "! ";
    default: return "  ";
  }
}

function setRunning(isRunning, t) {
  running = isRunning;
  if (isRunning) {
    total = t || 0;
    $("progressWrap").hidden = false;
    $("progressFill").style.width = "0%";
    $("progressLabel").textContent = total ? `0 / ${total}` : "";
    $("install").textContent = "Installing…";
    $("install").disabled = true;
  } else {
    $("install").textContent = "Install selected";
    $("install").disabled = selected.size === 0;
  }
  for (const b of ["selectAll", "selectNone", "refresh"]) $(b).disabled = isRunning;
}

async function startInstall() {
  if (running || selected.size === 0) return;
  appendLog("phase", `Installing ${selected.size} tool(s) (with prerequisites)...`);
  setRunning(true);
  try {
    await invoke("install_tools", { ids: Array.from(selected) });
  } catch (e) {
    appendLog("warn", "Failed to start install: " + e);
    setRunning(false);
  }
}

async function init() {
  try {
    tools = await invoke("list_tools");
  } catch (e) {
    appendLog("warn", "Could not load tools: " + e);
    return;
  }
  // default-select everything (foundation stays locked on regardless)
  for (const t of tools) if (!t.info.default_off) selected.add(t.info.id);
  // ensure homebrew selected
  selected.add("homebrew");
  render();

  await listen("mokio://started", (e) => setRunning(true, e.payload.total));
  await listen("mokio://log", (e) => appendLog(e.payload.level, e.payload.text));
  await listen("mokio://status", (e) => setCardStatus(e.payload.id, e.payload.status));
  await listen("mokio://progress", (e) => {
    const { done, total: t, id } = e.payload;
    const pct = t ? Math.round((done / t) * 100) : 0;
    $("progressFill").style.width = pct + "%";
    $("progressLabel").textContent = `${done} / ${t}` + (id ? ` · ${id}` : "");
    setCurrent(id);
  });
  await listen("mokio://finished", (e) => {
    setCurrent(null);
    const failed = (e.payload && e.payload.failed) || [];
    $("progressFill").style.width = "100%";
    if (failed.length === 0) {
      appendLog("phase", "✅ All selected tools installed successfully.");
    } else {
      appendLog("warn", `Finished with ${failed.length} failure(s): ${failed.join(", ")}`);
    }
    setRunning(false);
  });
}

// wire buttons
$("install").addEventListener("click", startInstall);
$("selectAll").addEventListener("click", () => {
  if (running) return;
  for (const t of tools) selected.add(t.info.id);
  render();
});
$("selectNone").addEventListener("click", () => {
  if (running) return;
  selected.clear();
  selected.add("homebrew");
  render();
});
$("refresh").addEventListener("click", async () => {
  if (running) return;
  appendLog("phase", "Re-running detection...");
  tools = await invoke("list_tools");
  render();
});
$("clearLog").addEventListener("click", () => { logEl.innerHTML = ""; });

init();
