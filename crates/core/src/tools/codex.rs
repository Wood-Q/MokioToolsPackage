//! OpenAI Codex — both the CLI (npm) and the desktop app (cask `codex-app`).

use crate::detect;
use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct Codex;

const ID: &str = "codex";

impl Installer for Codex {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Codex (CLI + Desktop)".to_string(),
            description: "OpenAI Codex coding agent — the `codex` CLI via npm, plus the Codex desktop app.".to_string(),
            category: Category::AiCli,
            homepage: "https://github.com/openai/codex".to_string(),
            order: 70,
            requires: vec!["node".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        let cli = shell::which("codex").is_some();
        let app = detect::app_bundle("Codex").is_some();
        if cli || app {
            Status::installed(detect::command_version("codex", &["--version"]))
        } else {
            Status::NotInstalled
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        let was_installed = self.detect().is_installed();

        // CLI (requires Node, which is a declared prerequisite).
        if shell::which("codex").is_none() {
            shell::npm_global_install(emit, "@openai/codex")?;
        } else {
            emit.info("codex CLI already present.");
        }

        // Desktop app.
        if detect::app_bundle("Codex").is_none() {
            // codex-app is best-effort: if the cask is unavailable on this
            // platform, warn and move on rather than failing the whole run.
            if let Err(e) = shell::brew_cask(emit, "codex-app") {
                emit.warn(format!(
                    "Codex desktop cask could not be installed ({e}). \
                     Download it manually from https://chatgpt.com/codex if needed."
                ));
            }
        } else {
            emit.info("Codex desktop app already installed.");
        }

        if was_installed {
            Ok(InstallOutcome::AlreadyInstalled)
        } else {
            Ok(InstallOutcome::Installed)
        }
    }
}
