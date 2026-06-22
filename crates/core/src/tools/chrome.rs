use crate::detect;
use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct Chrome;

const ID: &str = "chrome";

impl Installer for Chrome {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Google Chrome".to_string(),
            description: "The browser most web tooling expects.".to_string(),
            category: Category::Browser,
            homepage: "https://www.google.com/chrome/".to_string(),
            order: 35,
            requires: vec!["homebrew".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        match detect::app_bundle("Google Chrome") {
            Some(_) => Status::installed(None),
            None => Status::NotInstalled,
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("Google Chrome already installed — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }
        shell::brew_cask(emit, "google-chrome")?;
        Ok(InstallOutcome::Installed)
    }
}
