//! cc-switch — `farion1231/cc-switch`, a Tauri desktop app for switching AI CLI
//! provider configs. No Homebrew cask, so we pull the latest signed `.zip`
//! release from GitHub and drop `CC Switch.app` into /Applications.

use crate::detect;
use crate::event::Emitter;
use crate::error::{CoreError, Result};
use crate::github;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

pub struct CcSwitch;

const ID: &str = "cc-switch";
const APP_NAME: &str = "CC Switch"; // bundle: "CC Switch.app"

impl Installer for CcSwitch {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "CC Switch".to_string(),
            description: "Desktop app for switching Claude Code / Codex / Gemini CLI provider configs. Pulled from the latest GitHub release (signed + notarized).".to_string(),
            category: Category::AiCli,
            homepage: "https://github.com/farion1231/cc-switch".to_string(),
            order: 80,
            requires: vec!["homebrew".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        match detect::app_bundle(APP_NAME) {
            Some(_) => Status::installed(None),
            None => Status::NotInstalled,
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        if self.detect().is_installed() {
            emit.info("CC Switch already installed — skipping.");
            return Ok(InstallOutcome::AlreadyInstalled);
        }

        emit.phase("Resolving latest cc-switch release");
        let release = github::latest_release("farion1231", "cc-switch").ok_or_else(|| {
            CoreError::Download {
                what: "cc-switch".to_string(),
                detail: "could not reach the GitHub API (rate-limited?)".to_string(),
            }
        })?;
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("latest");
        // Prefer the macOS .zip (simplest to extract); fall back to .dmg.
        let url = github::asset_url(&release, |n| {
            n.contains("macOS") && n.ends_with(".zip")
        })
        .or_else(|| github::asset_url(&release, |n| n.contains("macOS") && n.ends_with(".dmg")))
        .ok_or_else(|| CoreError::Download {
            what: "cc-switch".to_string(),
            detail: format!("no macOS asset in release {tag}"),
        })?;
        emit.info(format!("Downloading cc-switch {tag}"));

        // Single self-contained script: mktemp -> curl -> unzip -> find .app -> cp -> dequarantine -> clean.
        let script = format!(
            r#"set -e
TMP="$(mktemp -d -t cc-switch.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
curl -fL "{url}" -o "$TMP/cc-switch.zip"
unzip -q -o "$TMP/cc-switch.zip" -d "$TMP/extract"
APP="$(find "$TMP/extract" -maxdepth 2 -iname "*.app" -print -quit)"
if [ -z "$APP" ]; then echo "no .app found in archive" >&2; exit 1; fi
cp -R "$APP" /Applications/
xattr -dr com.apple.quarantine "/Applications/{APP_NAME}.app" 2>/dev/null || true
echo "Installed: /Applications/{APP_NAME}.app"
"#,
            url = url,
            APP_NAME = APP_NAME,
        );
        shell::run_sh(emit, &script)?;
        Ok(InstallOutcome::Installed)
    }
}
