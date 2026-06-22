//! Subprocess helpers. Commands stream their stdout/stderr line-by-line
//! through the [`Emitter`](crate::event::Emitter) so the UI shows live output.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{CoreError, Result};
use crate::event::Emitter;

/// Locate an executable on PATH.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Run a command, streaming merged stdout+stderr to the emitter. Errors on a
/// non-zero exit code. If the program is missing, returns
/// [`CoreError::CommandNotFound`].
pub fn run(emit: &dyn Emitter, program: &str, args: &[&str]) -> Result<()> {
    let display = format_cmd(program, args);
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(CoreError::CommandNotFound {
                cmd: program.to_string(),
            });
        }
        Err(e) => return Err(e.into()),
    };

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    // Borrow the (non-'static) emitter into scoped reader threads.
    std::thread::scope(|s| {
        s.spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                emit.log(line);
            }
        });
        for line in BufReader::new(stdout).lines().flatten() {
            emit.log(line);
        }
    });

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(CoreError::CommandFailed {
            cmd: display,
            code: status.code().unwrap_or(-1),
        })
    }
}

/// Run a command through `sh -c`. Useful for pipes / curl installers.
pub fn run_sh(emit: &dyn Emitter, script: &str) -> Result<()> {
    run(emit, "/bin/sh", &["-c", script])
}

/// Capture stdout (trimmed), no streaming. Returns `None` if the command is
/// missing or exits non-zero.
pub fn capture(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `brew install --cask <cask>` — installs / upgrades a macOS app.
pub fn brew_cask(emit: &dyn Emitter, cask: &str) -> Result<()> {
    emit.phase(format!("Installing Homebrew cask {cask}"));
    run(emit, "brew", &["install", "--cask", cask])
}

/// `brew install <formula>`.
pub fn brew_formula(emit: &dyn Emitter, formula: &str) -> Result<()> {
    emit.phase(format!("Installing Homebrew formula {formula}"));
    run(emit, "brew", &["install", formula])
}

/// `npm install -g <pkg>` using whichever npm is first on PATH.
pub fn npm_global_install(emit: &dyn Emitter, pkg: &str) -> Result<()> {
    emit.phase(format!("npm install -g {pkg}"));
    run(emit, "npm", &["install", "-g", pkg])
}

fn format_cmd(program: &str, args: &[&str]) -> String {
    let mut s = program.to_string();
    for a in args {
        s.push(' ');
        s.push_str(a);
    }
    s
}
