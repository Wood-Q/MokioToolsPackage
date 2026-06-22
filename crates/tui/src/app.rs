//! TUI application state and the background worker that drives installs.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Sender;
use std::thread;

use mokio_core::catalog::Catalog;
use mokio_core::event::{Emitter, Event};
use mokio_core::i18n::{self, Lang};
use mokio_core::installer::{Status, ToolInfo};

const LOG_CAP: usize = 2000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Phase,
    Info,
    Log,
    Warn,
}

#[derive(Clone)]
pub struct LogLine {
    pub level: Level,
    pub text: String,
}

/// Messages the worker thread sends to the UI thread.
pub enum TuiMsg {
    Start(String),
    Event(Event),
    DoneOne { id: String, ok: bool },
    AllFinished { failed: Vec<String> },
}

/// Emitter backed by the UI channel — what installers write into.
struct ChannelEmitter {
    tx: Sender<TuiMsg>,
}

impl Emitter for ChannelEmitter {
    fn emit(&self, event: Event) {
        let _ = self.tx.send(TuiMsg::Event(event));
    }
}

pub struct App {
    pub catalog: Catalog,
    pub infos: Vec<ToolInfo>,
    pub selected: HashSet<String>,
    pub statuses: HashMap<String, Status>,
    pub log: VecDeque<LogLine>,
    pub cursor: usize,
    pub running: bool,
    pub finished: bool,
    pub current: Option<String>,
    pub done_count: usize,
    pub total_count: usize,
    pub last_error: Option<String>,
    pub lang: Lang,
    pub log_height: u16,
}

impl App {
    pub fn new() -> Self {
        let catalog = Catalog::new();
        let lang = Lang::default();
        let infos = catalog.localized_infos(lang);

        let mut statuses = HashMap::new();
        for info in &infos {
            if let Some(inst) = catalog.get(&info.id) {
                statuses.insert(info.id.clone(), inst.detect());
            }
        }

        let selected: HashSet<String> = infos
            .iter()
            .filter(|i| !i.default_off)
            .map(|i| i.id.clone())
            .collect();

        Self {
            catalog,
            infos,
            selected,
            statuses,
            log: VecDeque::new(),
            cursor: 0,
            running: false,
            finished: false,
            current: None,
            done_count: 0,
            total_count: 0,
            last_error: None,
            lang,
            log_height: 14,
        }
    }

    pub fn toggle_lang(&mut self) {
        self.lang = self.lang.toggle();
        self.infos = self.catalog.localized_infos(self.lang);
    }

    pub fn set_lang(&mut self, lang: Lang) {
        if lang != self.lang {
            self.lang = lang;
            self.infos = self.catalog.localized_infos(self.lang);
        }
    }

    /// Grow / shrink the log panel (keyboard `]` / `[`).
    pub fn grow_log(&mut self) {
        self.log_height = self.log_height.saturating_add(3);
    }
    pub fn shrink_log(&mut self) {
        self.log_height = self.log_height.saturating_sub(3).max(6);
    }

    /// Resize the log panel so its top border sits at `row` (mouse drag).
    /// `term_height` is the full terminal height in rows. Final clamping to fit
    /// the screen happens at draw time.
    pub fn drag_log_to(&mut self, row: u16, term_height: u16) {
        if term_height <= row + 1 {
            return;
        }
        let h = term_height.saturating_sub(row).saturating_sub(1);
        self.log_height = h.max(6);
    }

    pub fn cursor_info(&self) -> Option<&ToolInfo> {
        self.infos.get(self.cursor)
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let n = self.infos.len();
        if n == 0 {
            return;
        }
        let mut i = self.cursor as i32 + delta;
        if i < 0 {
            i = 0;
        }
        if i as usize >= n {
            i = (n - 1) as i32;
        }
        self.cursor = i as usize;
    }

    pub fn toggle_selected(&mut self) {
        if let Some(info) = self.cursor_info() {
            let id = info.id.clone();
            if info.id == "homebrew" {
                return; // foundation is always required
            }
            if self.selected.contains(&id) {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        }
    }

    pub fn select_all(&mut self, on: bool) {
        if on {
            for info in &self.infos {
                self.selected.insert(info.id.clone());
            }
        } else {
            self.selected.clear();
            self.selected.insert("homebrew".to_string()); // keep foundation
        }
    }

    pub fn redetect(&mut self) {
        for info in &self.infos {
            if let Some(inst) = self.catalog.get(&info.id) {
                self.statuses.insert(info.id.clone(), inst.detect());
            }
        }
        self.push_log(Level::Info, i18n::ui(self.lang, "log_redetect"));
    }

    pub fn push_log(&mut self, level: Level, text: impl Into<String>) {
        while self.log.len() >= LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(LogLine {
            level,
            text: text.into(),
        });
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Phase(s) => {
                self.push_log(Level::Phase, s.clone());
            }
            Event::Info(s) => self.push_log(Level::Info, s.clone()),
            Event::Log(s) => self.push_log(Level::Log, s.clone()),
            Event::Warn(s) => {
                self.last_error = Some(s.clone());
                self.push_log(Level::Warn, s.clone());
            }
            Event::Progress { .. } => {}
            Event::Status { id, status } => {
                self.statuses.insert(id.clone(), status.clone());
            }
        }
    }

    /// Start the install worker for the current selection (with prerequisites).
    pub fn start_install(&mut self, tx: Sender<TuiMsg>) {
        let ids: Vec<String> = self.selected.iter().cloned().collect();
        let ordered = self.catalog.expand_with_deps(&ids);
        self.total_count = ordered.len();
        self.done_count = 0;
        self.running = true;
        self.finished = false;
        self.last_error = None;
        self.push_log(
            Level::Phase,
            i18n::ui(self.lang, "log_install_plan")
                .replace("{n}", &ordered.len().to_string())
                .replace("{list}", &ordered.join(", ")),
        );

        let catalog = Catalog::new();
        thread::spawn(move || {
            let emitter = ChannelEmitter { tx: tx.clone() };
            let emit: &dyn Emitter = &emitter;
            let mut failed = Vec::new();
            for id in &ordered {
                let _ = tx.send(TuiMsg::Start(id.clone()));
                let result = match catalog.get(id) {
                    Some(inst) => inst.install(emit).map(|_| true),
                    None => {
                        emit.warn(format!("unknown tool {id}"));
                        Ok(false)
                    }
                };
                let ok = result.unwrap_or_else(|e| {
                    emit.warn(format!("✗ {id}: {e}"));
                    false
                });
                let _ = tx.send(TuiMsg::DoneOne {
                    id: id.clone(),
                    ok,
                });
                if !ok {
                    failed.push(id.clone());
                }
            }
            let _ = tx.send(TuiMsg::AllFinished { failed });
        });
    }
}
