//! Progress / log events streamed out of an installer run.
//!
//! Installers are synchronous and blocking (they spawn subprocesses). Front-ends
//! run them on a worker thread and receive [`Event`]s through a channel that
//! implements [`Emitter`].

use crate::installer::Status;

/// A single line of progress emitted by an installer.
#[derive(Debug, Clone)]
pub enum Event {
    /// A high-level phase transition, e.g. "Installing Homebrew cask ghostty".
    Phase(String),
    /// A raw stdout/stderr line from a subprocess.
    Log(String),
    /// A normal human-readable informational line (not from a subprocess).
    Info(String),
    /// A warning that does not abort the run.
    Warn(String),
    /// Determinate progress, when known.
    Progress { done: usize, total: usize },
    /// The tool's status changed (e.g. detected installed, or finished).
    Status { id: String, status: Status },
}

/// Sink for installer events. Implementations forward to a channel
/// (TUI) or a Tauri event bus (desktop).
///
/// The trait stays dyn-compatible (single non-generic method); the typed
/// convenience helpers live as inherent methods on `dyn Emitter` below.
pub trait Emitter: Send + Sync {
    fn emit(&self, event: Event);
}

impl Emitter for std::sync::mpsc::Sender<Event> {
    fn emit(&self, event: Event) {
        // A full channel is treated as best-effort; drop rather than block the worker.
        let _ = self.send(event);
    }
}

/// Wrapper that tags every [`Event::Status`] with the owning tool id, so a
/// tool module never has to pass its own id around.
pub struct ScopedEmitter<'a> {
    pub id: &'a str,
    pub inner: &'a dyn Emitter,
}

impl Emitter for ScopedEmitter<'_> {
    fn emit(&self, event: Event) {
        if let Event::Status { status, .. } = &event {
            self.inner.emit(Event::Status {
                id: self.id.to_string(),
                status: status.clone(),
            });
        } else {
            self.inner.emit(event);
        }
    }
}

impl dyn Emitter + '_ {
    pub fn phase(&self, msg: impl Into<String>) {
        self.emit(Event::Phase(msg.into()));
    }
    pub fn log(&self, msg: impl Into<String>) {
        self.emit(Event::Log(msg.into()));
    }
    pub fn info(&self, msg: impl Into<String>) {
        self.emit(Event::Info(msg.into()));
    }
    pub fn warn(&self, msg: impl Into<String>) {
        self.emit(Event::Warn(msg.into()));
    }
    pub fn progress(&self, done: usize, total: usize) {
        self.emit(Event::Progress { done, total });
    }
    pub fn status(&self, id: &str, status: Status) {
        self.emit(Event::Status {
            id: id.to_string(),
            status,
        });
    }
}
