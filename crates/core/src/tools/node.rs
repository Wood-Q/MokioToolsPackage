//! Node.js + npm via nvm — installs nvm, Node.js LTS (which bundles npm),
//! and verifies that `npm` is available on PATH.

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
            name: "Node.js + npm (nvm)".to_string(),
            description: "Node.js LTS (with npm) managed by nvm, the Node Version Manager. Replicates your current ~/.nvm setup.".to_string(),
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
        // `nvm install --lts` installs the latest LTS and its bundled npm.
        let script = r#". "$HOME/.nvm/nvm.sh" && nvm install --lts && nvm alias default 'lts/*' && nvm use default"#;
        shell::run_sh(emit, script)?;

        // Verify npm is available alongside node.
        emit.phase("Verifying npm");
        let verify = r#". "$HOME/.nvm/nvm.sh" && npm --version"#;
        match shell::run_sh(emit, verify) {
            Ok(()) => emit.info("npm is available."),
            Err(e) => emit.warn(format!("npm verification failed ({e}). Node.js LTS bundles npm — try opening a new terminal and running `npm --version`.")),
        }

        Ok(InstallOutcome::Installed)
    }
}
