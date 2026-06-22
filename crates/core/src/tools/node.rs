//! Node.js via nvm — matches the developer's existing setup (the same `nvm`
//! loader block is written to ~/.zshrc by the terminal installer).

use crate::detect;
use crate::event::Emitter;
use crate::error::{CoreError, Result};
use crate::github;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;
use std::path::PathBuf;

pub struct Node;

const ID: &str = "node";
const NVM_DIR_NAME: &str = ".nvm";

fn nvm_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(NVM_DIR_NAME))
}

impl Installer for Node {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Node.js (nvm)".to_string(),
            description: "Node.js LTS managed by nvm, the Node Version Manager. Replicates your current ~/.nvm setup.".to_string(),
            category: Category::Runtime,
            homepage: "https://nodejs.org".to_string(),
            order: 20,
            requires: vec!["homebrew".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        let nvm = nvm_dir().map(|d| d.is_dir()).unwrap_or(false);
        match detect::command_version("node", &["--version"]) {
            Some(v) if nvm => Status::installed(Some(v)),
            Some(v) => Status::installed(Some(v)),
            None if nvm => Status::installed(None),
            None => Status::NotInstalled,
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("Node.js already available — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }

        let nvm = nvm_dir().ok_or_else(|| CoreError::other("no home directory"))?;
        if !nvm.is_dir() {
            emit.phase("Fetching latest nvm release tag");
            let tag = github::latest_tag("nvm-sh", "nvm").unwrap_or_else(|| "v0.40.1".to_string());
            emit.info(format!("Installing nvm {tag}"));
            let url = format!("https://raw.githubusercontent.com/nvm-sh/nvm/{tag}/install.sh");
            // The installer fetches the repo into ~/.nvm and appends loaders to
            // shell rc files. It is safe to re-run.
            shell::run_sh(emit, &format!("PROFILE=/dev/null bash -c \"$(curl -fsSL {url})\""))?;
        } else {
            emit.info("nvm already cloned — skipping download.");
        }

        emit.phase("Installing Node.js LTS via nvm");
        // nvm is a shell function; it must be sourced before use.
        let script = r#". "$HOME/.nvm/nvm.sh" && nvm install --lts && nvm alias default 'lts/*' && nvm use default"#;
        shell::run_sh(emit, script)?;
        Ok(InstallOutcome::Installed)
    }
}
