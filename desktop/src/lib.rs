//! Tauri 2 back-end. Thin shell over `mokio-core`: it exposes a handful of
//! commands and bridges the core `Emitter` stream to the webview via Tauri
//! events. UI language (Chinese by default) is toggled from the front-end and
//! drives localized tool names / category labels / chrome strings.

use std::collections::HashMap;
use std::sync::Mutex;

use mokio_core::event::{Emitter as CoreEmitter, Event};
use mokio_core::i18n::{self, Lang};
use mokio_core::installer::Status;
use mokio_core::{Catalog, ToolInfo};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize, Clone)]
struct LogPayload {
    level: &'static str,
    text: String,
}

#[derive(Serialize, Clone)]
struct StatusPayload {
    id: String,
    status: Status,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    done: usize,
    total: usize,
    id: Option<String>,
}

#[derive(Serialize, Clone)]
struct FinishedPayload {
    failed: Vec<String>,
}

/// Bridges core events to the webview's Tauri event bus.
struct TauriEmitter {
    app: AppHandle,
}

impl CoreEmitter for TauriEmitter {
    fn emit(&self, event: Event) {
        match event {
            Event::Phase(s) => {
                let _ = self.app.emit("mokio://log", LogPayload { level: "phase", text: s });
            }
            Event::Info(s) => {
                let _ = self.app.emit("mokio://log", LogPayload { level: "info", text: s });
            }
            Event::Log(s) => {
                let _ = self.app.emit("mokio://log", LogPayload { level: "log", text: s });
            }
            Event::Warn(s) => {
                let _ = self.app.emit("mokio://log", LogPayload { level: "warn", text: s });
            }
            Event::Status { id, status } => {
                let _ = self.app.emit("mokio://status", StatusPayload { id, status });
            }
            Event::Progress { done, total } => {
                let _ = self.app.emit(
                    "mokio://progress",
                    ProgressPayload { done, total, id: None },
                );
            }
        }
    }
}

#[derive(Serialize)]
struct ToolEntry {
    info: ToolInfo,
    status: Status,
    category_label: String,
}

/// One entry per tool, localised to `lang`, with live detection.
#[tauri::command]
fn list_tools(lang: Lang) -> Vec<ToolEntry> {
    let catalog = Catalog::new();
    catalog
        .localized_infos(lang)
        .into_iter()
        .map(|info| {
            let category_label = i18n::category_label(lang, info.category).to_string();
            let status = catalog
                .get(&info.id)
                .map(|i| i.detect())
                .unwrap_or_default();
            ToolEntry { info, status, category_label }
        })
        .collect()
}

/// All UI chrome strings for `lang`, as a key→string map the front-end looks up.
#[tauri::command]
fn ui_strings(lang: Lang) -> HashMap<String, String> {
    const KEYS: &[&str] = &[
        "tagline",
        "panel_log",
        "btn_install",
        "btn_installing",
        "btn_select_all",
        "btn_clear",
        "btn_redetect",
        "btn_log_clear",
        "summary",
        "footer_desktop",
        "log_all_ok",
        "log_failed",
        "log_install_plan",
        "log_redetect",
        "needs",
        "homepage",
        "foundation_label",
        "st_installed_v",
        "st_installed",
        "st_not_installed",
        "st_unknown",
    ];
    KEYS.iter()
        .map(|k| ((*k).to_string(), i18n::ui(lang, k).to_string()))
        .collect()
}

struct LangState {
    lang: Mutex<Lang>,
}

#[tauri::command]
fn current_lang(state: State<'_, LangState>) -> Lang {
    *state.lang.lock().expect("lang lock")
}

#[tauri::command]
fn cycle_lang(state: State<'_, LangState>) -> Lang {
    let mut lang = state.lang.lock().expect("lang lock");
    *lang = lang.toggle();
    *lang
}

#[tauri::command]
fn set_lang(lang: Lang, state: State<'_, LangState>) -> Lang {
    let mut current = state.lang.lock().expect("lang lock");
    *current = lang;
    *current
}

/// Kick off installs on a background thread; events stream back over the bus.
#[tauri::command]
fn install_tools(
    ids: Vec<String>,
    app: AppHandle,
    state: State<'_, InstallState>,
) -> Result<(), String> {
    {
        let mut busy = state.busy.lock().map_err(|e| e.to_string())?;
        if *busy {
            return Err("an install is already running".into());
        }
        *busy = true;
    }

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let catalog = Catalog::new();
        let ordered = catalog.expand_with_deps(&ids);
        let total = ordered.len();
        let _ = app.emit("mokio://started", serde_json::json!({ "total": total }));

        let emitter = TauriEmitter { app: app.clone() };
        let emit: &dyn CoreEmitter = &emitter;

        let mut failed: Vec<String> = Vec::new();
        for (idx, id) in ordered.iter().enumerate() {
            let _ = app.emit(
                "mokio://progress",
                ProgressPayload { done: idx, total, id: Some(id.clone()) },
            );
            match catalog.get(id) {
                Some(installer) => {
                    if let Err(e) = installer.install(emit) {
                        emit.warn(format!("✗ {id}: {e}"));
                        failed.push(id.clone());
                    }
                }
                None => {
                    emit.warn(format!("unknown tool: {id}"));
                    failed.push(id.clone());
                }
            }
        }

        // Final detection sweep so the UI reflects reality.
        for info in catalog.infos() {
            if let Some(installer) = catalog.get(&info.id) {
                let _ = app.emit(
                    "mokio://status",
                    StatusPayload { id: info.id.clone(), status: installer.detect() },
                );
            }
        }

        let _ = app_handle.emit("mokio://finished", FinishedPayload { failed });
        if let Some(s) = app_handle.try_state::<InstallState>() {
            if let Ok(mut busy) = s.busy.lock() {
                *busy = false;
            }
        }
    });

    Ok(())
}

/// Open a URL in the system browser (used for the homepage links).
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

struct InstallState {
    busy: Mutex<bool>,
}

pub fn run() {
    tauri::Builder::default()
        .manage(LangState { lang: Mutex::new(Lang::default()) })
        .manage(InstallState { busy: Mutex::new(false) })
        .invoke_handler(tauri::generate_handler![
            list_tools,
            ui_strings,
            current_lang,
            cycle_lang,
            set_lang,
            install_tools,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running mokio-desktop");
}
