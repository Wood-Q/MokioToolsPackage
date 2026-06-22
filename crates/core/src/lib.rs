//! mokio-core — installation logic shared by the TUI and the desktop app.
//!
//! Everything that actually installs / configures a tool lives here as a
//! stateless [`Installer`] implementation. The front-ends only render state and
//! forward events.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod detect;
pub mod error;
pub mod event;
pub mod github;
pub mod i18n;
pub mod installer;
pub mod shell;
pub mod tools;

pub use catalog::Catalog;
pub use error::{CoreError, Result};
pub use event::{Emitter, Event, ScopedEmitter};
pub use i18n::Lang;
pub use installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
