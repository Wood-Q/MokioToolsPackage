use crate::detect;
use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct VsCode;

const ID: &str = "vscode";

impl Installer for VsCode {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Visual Studio Code".to_string(),
            description: "The editor. Installs the cask and wires up the `code` CLI.".to_string(),
            category: Category::Editor,
            homepage: "https://code.visualstudio.com".to_string(),
            order: 30,
            requires: vec!["homebrew".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        if detect::app_bundle("Visual Studio Code").is_some() {
            return Status::installed(detect::command_version("code", &["--version"]));
        }
        if shell::which("code").is_some() {
            return Status::installed(detect::command_version("code", &["--version"]));
        }
        Status::NotInstalled
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("VS Code already installed — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }
        shell::brew_cask(emit, "visual-studio-code")?;
        Ok(InstallOutcome::Installed)
    }
}
