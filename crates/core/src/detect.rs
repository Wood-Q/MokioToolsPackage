//! Cheap detection helpers used by installer `detect()` implementations.

use crate::shell;
use std::path::PathBuf;

/// Is the given Homebrew cask/formula installed?
pub fn brew_installed(unit: &str) -> bool {
    // `brew list --cask/--formula <unit>` is the canonical check, but slow when
    // invoked many times. `brew --prefix <unit>` is fast and exits non-zero if
    // absent.
    shell::capture("brew", &["--prefix", unit]).is_some()
}

/// Is a `.app` bundle present in /Applications or ~/Applications?
pub fn app_bundle(name: &str) -> Option<PathBuf> {
    for base in [
        PathBuf::from("/Applications"),
        dirs::home_dir()?.join("Applications"),
    ] {
        let p = base.join(format!("{name}.app"));
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

/// First line of `<cmd> --version` (or any args), trimmed. `None` if missing.
pub fn command_version(cmd: &str, args: &[&str]) -> Option<String> {
    let out = shell::capture(cmd, args)?;
    out.lines().next().map(|s| s.trim().to_string())
}

/// Resolve the currently active Node's `npm`. On a fresh machine it may not
/// exist yet, which is fine.
pub fn npm_available() -> bool {
    shell::which("npm").is_some()
}

/// Is the global npm package installed? Checks `npm ls -g`.
pub fn npm_global_present(pkg: &str) -> bool {
    shell::capture("npm", &["ls", "-g", "--depth=0", pkg]).is_some()
}
