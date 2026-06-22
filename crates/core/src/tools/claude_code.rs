//! Claude Code — Anthropic's agentic CLI, via npm.

use crate::detect;
use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct ClaudeCode;

const ID: &str = "claude-code";

impl Installer for ClaudeCode {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Claude Code".to_string(),
            description: "Anthropic's agentic command-line coding tool (`claude`).".to_string(),
            category: Category::AiCli,
            homepage: "https://docs.claude.com/en/docs/claude-code/overview".to_string(),
            order: 75,
            requires: vec!["node".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        match detect::command_version("claude", &["--version"]) {
            Some(v) => Status::installed(Some(v)),
            None => Status::NotInstalled,
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("Claude Code already installed — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }
        shell::npm_global_install(emit, "@anthropic-ai/claude-code")?;
        Ok(InstallOutcome::Installed)
    }
}
