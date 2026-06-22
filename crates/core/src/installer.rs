//! The [`Installer`] trait every tool implements, plus the metadata types the
//! front-ends render.

use crate::event::Emitter;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Coarse grouping used to organise the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// Homebrew itself — the foundation everything else leans on.
    Foundational,
    /// Languages / runtimes / package managers (Node, uv).
    Runtime,
    /// Version control (Git).
    Vcs,
    /// Editors (VSCode).
    Editor,
    /// Browsers (Chrome).
    Browser,
    /// Terminal + shell + file manager (Ghostty + Yazi + Oh-My-Zsh).
    Terminal,
    /// AI coding agents and their switchers (Codex, Claude Code, cc-switch).
    AiCli,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Foundational => "Foundational",
            Category::Runtime => "Runtimes & package managers",
            Category::Vcs => "Version control",
            Category::Editor => "Editor",
            Category::Browser => "Browser",
            Category::Terminal => "Terminal & shell",
            Category::AiCli => "AI coding agents",
        }
    }
}

/// Static description of a tool, shown in lists / cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: Category,
    pub homepage: String,
    /// Lower runs first when installing a batch.
    pub order: u32,
    /// Ids that must be installed (or already present) before this tool.
    pub requires: Vec<String>,
    /// If true, the tool is off by default in the selection UI.
    pub default_off: bool,
}

/// Detected installation state of a tool.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Detection has not run yet.
    #[default]
    Unknown,
    NotInstalled,
    Installed {
        version: Option<String>,
    },
}

impl Status {
    pub fn installed(version: impl Into<Option<String>>) -> Self {
        Status::Installed {
            version: version.into(),
        }
    }
    pub fn is_installed(&self) -> bool {
        matches!(self, Status::Installed { .. })
    }
}

/// Outcome of an install attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallOutcome {
    /// Newly installed successfully.
    Installed,
    /// Already present, nothing to do.
    AlreadyInstalled,
}

/// A single tool's installer. Implementations are stateless value types held in
/// the [`crate::catalog::Catalog`].
pub trait Installer: Send + Sync {
    fn info(&self) -> ToolInfo;

    /// Cheap check: is the tool already on the machine?
    fn detect(&self) -> Status;

    /// Perform the installation. Should be idempotent — re-running on an
    /// already-installed machine is a no-op that returns `AlreadyInstalled`.
    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome>;

    /// Optional removal. Default is "not supported".
    fn uninstall(&self, _emit: &dyn Emitter) -> Result<()> {
        Err(crate::error::CoreError::other("uninstall not supported"))
    }
}
