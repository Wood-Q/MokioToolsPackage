use crate::detect;
use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct Git;

const ID: &str = "git";

impl Installer for Git {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Git".to_string(),
            description: "Distributed version control. Installs the Homebrew formula and ensures the Xcode Command Line Tools are present.".to_string(),
            category: Category::Vcs,
            homepage: "https://git-scm.com".to_string(),
            order: 10,
            requires: vec!["homebrew".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        match detect::command_version("git", &["--version"]) {
            Some(v) => Status::installed(Some(v)),
            None => Status::NotInstalled,
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("Git already present — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }
        // Command Line Tools provide the system git and the SDK brew needs.
        emit.phase("Ensuring Xcode Command Line Tools");
        let _ = shell::run_sh(emit, "xcode-select --install 2>/dev/null || true");
        shell::brew_formula(emit, "git")?;
        Ok(InstallOutcome::Installed)
    }
}
