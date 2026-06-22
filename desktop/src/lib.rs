//! Tauri 2 back-end. Thin shell over `mokio-core`: it exposes two commands
//! (`list_tools`, `install_tools`) and bridges the core `Emitter` stream to the
//! webview via Tauri events.

use std::sync::Mutex;

use mokio_core::event::{Emitter as CoreEmitter, Event};
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
}

/// One entry per tool, with live detection.
#[tauri::command]
fn list_tools() -> Vec<ToolEntry> {
    let catalog = Catalog::new();
    catalog
        .infos()
        .into_iter()
        .map(|info| {
            let status = catalog
                .get(&info.id)
                .map(|i| i.detect())
                .unwrap_or_default();
            ToolEntry { info, status }
        })
        .collect()
}

/// Re-run detection for a single tool (used by the refresh button).
#[tauri::command]
fn detect_tool(id: String) -> Status {
    let catalog = Catalog::new();
    catalog
        .get(&id)
        .map(|i| i.detect())
        .unwrap_or_default()
}

struct InstallState {
    busy: Mutex<bool>,
}

/// Kick off installs on a background thread; events stream back over the bus.
/// Returns immediately so the UI can render progress.
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

pub fn run() {
    tauri::Builder::default()
        .manage(InstallState { busy: Mutex::new(false) })
        .invoke_handler(tauri::generate_handler![list_tools, detect_tool, install_tools, open_url])
        .run(tauri::generate_context!())
        .expect("error while running mokio-desktop");
}
