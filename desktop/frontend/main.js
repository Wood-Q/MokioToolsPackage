// Mokio desktop front-end. Talks to the Rust back-end via Tauri commands/events.
// UI language defaults to Chinese; the EN/中文 button toggles it.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

/** @type {Array<{info: any, status: any, category_label: string}>} */
let tools = [];
/** @type {Record<string,string>} */
let strings = {};
let lang = "zh";
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

function S(key, fallback) {
  return (strings && strings[key]) || fallback;
}

function statusBadge(status) {
  const tag = statusTag(status);
  if (tag === "installed") {
    const v = statusVersion(status);
    const tmpl = v ? S("st_installed_v", "installed ({v})") : S("st_installed", "installed");
    return { cls: "installed", text: v ? tmpl.replace("{v}", v) : tmpl };
  }
  if (tag === "not_installed") return { cls: "missing", text: S("st_not_installed", "not installed") };
  return { cls: "", text: S("st_unknown", "unknown") };
}

function summaryText() {
  const installed = tools.filter((t) => statusTag(t.status) === "installed").length;
  return S("summary", "{installed}/{total} installed · {selected} selected")
    .replace("{installed}", installed)
    .replace("{total}", tools.length)
    .replace("{selected}", selected.size);
}

function applyStrings() {
  $("tagline").textContent = S("tagline", "一键配置 macOS 开发工具链");
  $("logHead").textContent = S("panel_log", "日志");
  $("footerNote").textContent = S("footer_desktop", "");
  $("selectAll").textContent = S("btn_select_all", "全选");
  $("selectNone").textContent = S("btn_clear", "清空");
  $("refresh").textContent = S("btn_redetect", "重新检测");
  $("clearLog").textContent = S("btn_log_clear", "清空");
  if (!running) $("install").textContent = S("btn_install", "安装所选");
  // lang button always shows the *other* language
  $("langBtn").textContent = lang === "zh" ? "EN" : "中文";
  document.documentElement.lang = lang === "zh" ? "zh" : "en";
}

function render() {
  cardsEl.innerHTML = "";
  for (const t of tools) {
    const { info, status, category_label } = t;
    const badge = statusBadge(status);
    const isSel = selected.has(info.id);
    const card = document.createElement("div");
    card.className = "card" + (isSel ? " selected" : "");
    card.dataset.id = info.id;

    const requires =
      info.requires && info.requires.length
        ? `${S("needs", "needs: {list}").replace("{list}", info.requires.join(", "))}`
        : info.id === "homebrew"
        ? S("foundation_label", info.id === "homebrew" ? "基础" : "")
        : "";

    card.innerHTML = `
      <div class="card-row">
        <div class="check"></div>
        <span class="card-name">${escapeHtml(info.name)}</span>
        <span class="card-cat">${escapeHtml(category_label || info.category)}</span>
      </div>
      <div class="card-desc">${escapeHtml(info.description)}</div>
      <div class="card-foot">
        <span class="badge ${badge.cls}">${badge.text}</span>
        <a class="card-home" data-url="${escapeHtml(info.homepage)}" href="#">${S("homepage", "主页")} ↗</a>
      </div>
      <div class="card-foot"><span>${escapeHtml(requires)}</span><span></span></div>
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
    $("install").textContent = S("btn_installing", "安装中…");
    $("install").disabled = true;
  } else {
    $("install").textContent = S("btn_install", "安装所选");
    $("install").disabled = selected.size === 0;
  }
  for (const b of ["selectAll", "selectNone", "refresh", "langBtn"]) $(b).disabled = isRunning;
}

async function startInstall() {
  if (running || selected.size === 0) return;
  appendLog(
    "phase",
    S("log_install_plan", "Installing {n} tool(s): {list}")
      .replace("{n}", String(selected.size))
      .replace("{list}", Array.from(selected).join(", "))
  );
  setRunning(true);
  try {
    await invoke("install_tools", { ids: Array.from(selected) });
  } catch (e) {
    appendLog("warn", "Failed to start install: " + e);
    setRunning(false);
  }
}

async function loadAll(l) {
  lang = l;
  [strings, tools] = await Promise.all([
    invoke("ui_strings", { lang: l }),
    invoke("list_tools", { lang: l }),
  ]);
  applyStrings();
  render();
}

async function init() {
  try {
    lang = await invoke("current_lang");
  } catch {
    lang = "zh";
  }
  await loadAll(lang);

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
      appendLog("phase", S("log_all_ok", "✅ All selected tools installed successfully."));
    } else {
      appendLog(
        "warn",
        S("log_failed", "Finished with {n} failure(s): {list}")
          .replace("{n}", String(failed.length))
          .replace("{list}", failed.join(", "))
      );
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
  appendLog("phase", S("log_redetect", "Re-running detection..."));
  await loadAll(lang);
});
$("clearLog").addEventListener("click", () => { logEl.innerHTML = ""; });
$("langBtn").addEventListener("click", async () => {
  if (running) return;
  const next = await invoke("cycle_lang");
  await loadAll(next);
});

// Draggable log resizer — drag the handle above the log to resize it.
(function () {
  const resizer = $("resizer");
  const panel = document.querySelector(".log-panel");
  let dragging = false;
  resizer.addEventListener("mousedown", (e) => {
    e.preventDefault();
    dragging = true;
    document.body.classList.add("dragging");
  });
  window.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    const footerH = document.querySelector(".footer").offsetHeight;
    const newH = window.innerHeight - footerH - e.clientY;
    const clamped = Math.max(60, Math.min(newH, window.innerHeight * 0.85));
    panel.style.height = clamped + "px";
  });
  const stop = () => {
    if (!dragging) return;
    dragging = false;
    document.body.classList.remove("dragging");
  };
  window.addEventListener("mouseup", stop);
  window.addEventListener("blur", stop);
})();

init();
