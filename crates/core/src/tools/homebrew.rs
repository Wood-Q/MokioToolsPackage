//! Homebrew — the foundation. Most other installers shell out to `brew`, so
//! this one runs first whenever anything that needs it is selected.

use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct Homebrew;

const ID: &str = "homebrew";

impl Installer for Homebrew {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Homebrew".to_string(),
            description: "The missing package manager for macOS — required by nearly everything else.".to_string(),
            category: Category::Foundational,
            homepage: "https://brew.sh".to_string(),
            order: 0,
            requires: vec![],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        match shell::which("brew") {
            Some(_) => Status::installed(shell::capture("brew", &["--version"])),
            None => Status::NotInstalled,
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("Homebrew already installed — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }
        emit.phase("Installing Homebrew (NONINTERACTIVE)");
        emit.warn("This may take several minutes and will prompt for your password if sudo is needed.");
        // The official installer is non-interactive with this env var.
        run_sh_with_retry(
            emit,
            r#"NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#,
        )?;

        // Help future shells find brew on Apple Silicon.
        if cfg!(target_arch = "aarch64") {
            let hint = r#"eval "$(/opt/homebrew/bin/brew shellenv)""#;
            emit.info("Homebrew installed at /opt/homebrew. Add this to your shell profile (already in ~/.zprofile by the installer):");
            emit.info(hint);
        }
        Ok(InstallOutcome::Installed)
    }
}

fn run_sh_with_retry(emit: &dyn Emitter, script: &str) -> Result<()> {
    match shell::run_sh(emit, script) {
        Ok(()) => Ok(()),
        Err(e) => {
            emit.warn(format!("First attempt failed ({e}); retrying once..."));
            shell::run_sh(emit, script)
        }
    }
}
