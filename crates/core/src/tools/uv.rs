use crate::detect;
use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct Uv;

const ID: &str = "uv";

impl Installer for Uv {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "uv".to_string(),
            description: "Extremely fast Python package installer / resolver (Astral). Replaces pip + virtualenv workflows.".to_string(),
            category: Category::Runtime,
            homepage: "https://docs.astral.sh/uv/".to_string(),
            order: 25,
            requires: vec!["homebrew".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        match detect::command_version("uv", &["--version"]) {
            Some(v) => Status::installed(Some(v)),
            None => Status::NotInstalled,
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("uv already installed — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }
        shell::brew_formula(emit, "uv")?;
        Ok(InstallOutcome::Installed)
    }
}
