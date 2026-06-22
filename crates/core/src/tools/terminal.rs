//! The terminal suite: Ghostty + Maple Mono NF CN font + Yazi (with preview
//! deps) + Oh-My-Zsh + autosuggestions / syntax-highlighting / thefuck / zoxide,
//! then writes the bundled Ghostty & Yazi configs and patches `~/.zshrc` with a
//! managed block that faithfully reproduces the developer's existing shell.

use std::fs;
use std::path::PathBuf;

use include_dir::{include_dir, Dir};

use crate::detect;
use crate::event::Emitter;
use crate::error::Result;
use crate::installer::{Category, InstallOutcome, Installer, Status, ToolInfo};
use crate::shell;

static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub struct Terminal;

const ID: &str = "terminal";
const ZSHRC_TERMINAL_MARKER: &str = "# >>> mokio terminal setup >>>";

const OHMYZSH_BLOCK: &str = r#"# >>> mokio oh-my-zsh >>>
# Managed by Mokio. Edits here are preserved; the block is regenerated on reinstall.
export ZSH="$HOME/.oh-my-zsh"
ZSH_THEME="robbyrussell"
DISABLE_AUTO_UPDATE="true"
plugins=(git)
source $ZSH/oh-my-zsh.sh
# <<< mokio oh-my-zsh <<<"#;

const TERMINAL_BLOCK: &str = r#"# >>> mokio terminal setup >>>
# Managed by Mokio — Ghostty + Yazi + zsh niceties.

# Homebrew shellenv (Apple Silicon path, with Intel fallback).
eval "$(/opt/homebrew/bin/brew shellenv 2>/dev/null || /usr/local/bin/brew shellenv 2>/dev/null)"

export LANG=en_US.UTF-8
# Hide user@host in agnoster-style themes.
export DEFAULT_USER="$(whoami)"
ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE=fg=30

# nvm (Node Version Manager)
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
[ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"

# Ghostty: set the window title to the current directory.
if [[ -n "${GHOSTTY_RESOURCES_DIR:-}" ]]; then
    ghostty_set_title() {
        local dir="${PWD/#$HOME/~}"
        printf '\033]2;%s\033\\' "$dir"
    }
    autoload -Uz add-zsh-hook
    add-zsh-hook chpwd ghostty_set_title
    add-zsh-hook precmd ghostty_set_title
    add-zsh-hook preexec ghostty_set_title
    ghostty_set_title
fi

# Yazi: cd into the last directory on exit.
function y() {
    local tmp="$(mktemp -t "yazi-cwd.XXXXXX")" cwd
    yazi "$@" --cwd-file="$tmp"
    if cwd="$(command cat -- "$tmp")" && [ -n "$cwd" ] && [ "$cwd" != "$PWD" ]; then
        builtin cd -- "$cwd"
    fi
    /bin/rm -f -- "$tmp"
}

# Completions, autosuggestions, syntax-highlighting (loaded last).
FPATH="$(brew --prefix)/share/zsh/site-functions:$FPATH"
autoload -Uz compinit
compinit
source "$(brew --prefix)/share/zsh-autosuggestions/zsh-autosuggestions.zsh" 2>/dev/null
source "$(brew --prefix)/share/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh" 2>/dev/null

# thefuck
eval "$(thefuck --alias 2>/dev/null)"

# zoxide (smart `cd` via `z`)
eval "$(zoxide init zsh 2>/dev/null)"
# <<< mokio terminal setup <<<"#;

/// Formulae Yazi wants for full previews, plus the zsh niceties we source below.
const BREW_FORMULAE: &[&str] = &[
    "yazi",
    "ffmpeg",
    "sevenzip",
    "jq",
    "poppler",
    "fd",
    "ripgrep",
    "fzf",
    "zoxide",
    "imagemagick",
    "zsh-autosuggestions",
    "zsh-syntax-highlighting",
    "thefuck",
];

impl Installer for Terminal {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: ID.to_string(),
            name: "Terminal suite".to_string(),
            description: "Ghostty + Maple Mono NF CN + Yazi (with preview deps) + Oh-My-Zsh + autosuggestions/syntax-highlighting/thefuck/zoxide, plus your exact Ghostty & Yazi configs and a managed ~/.zshrc block.".to_string(),
            category: Category::Terminal,
            homepage: "https://ghostty.org".to_string(),
            order: 40,
            requires: vec!["git".to_string()],
            default_off: false,
        }
    }

    fn detect(&self) -> Status {
        let ghostty_app = detect::app_bundle("Ghostty").is_some();
        let ohmyzsh = dirs::home_dir()
            .map(|h| h.join(".oh-my-zsh").is_dir())
            .unwrap_or(false);
        let ghostty_cfg = dirs::config_dir()
            .map(|c| c.join("ghostty/config").is_file())
            .unwrap_or(false);
        let zshrc_block = zshrc_has_marker(ZSHRC_TERMINAL_MARKER);
        if ghostty_app && ohmyzsh && ghostty_cfg && zshrc_block {
            Status::installed(None)
        } else {
            Status::NotInstalled
        }
    }

    fn install(&self, emit: &dyn Emitter) -> Result<InstallOutcome> {
        // 1. Ghostty + font.
        if detect::app_bundle("Ghostty").is_none() {
            shell::brew_cask(emit, "ghostty")?;
        } else {
            emit.info("Ghostty already installed.");
        }
        // The font cask name; presence checked via the Font Book is hard, so just install.
        shell::brew_cask(emit, "font-maple-mono-nf-cn")?;

        // 2. Yazi + preview deps + zsh plugins.
        emit.phase("Installing Yazi, preview deps, and zsh plugins");
        let mut args: Vec<&str> = vec!["install"];
        args.extend_from_slice(BREW_FORMULAE);
        shell::run(emit, "brew", &args)?;

        // 3. Oh-My-Zsh (clone without the install.sh so it never mutates rc files).
        let ohmyzsh = dirs::home_dir()
            .map(|h| h.join(".oh-my-zsh"))
            .unwrap_or_else(|| PathBuf::from(".oh-my-zsh"));
        if !ohmyzsh.is_dir() {
            emit.phase("Cloning Oh-My-Zsh");
            shell::run(
                emit,
                "git",
                &[
                    "clone",
                    "--depth=1",
                    "https://github.com/ohmyzsh/ohmyzsh.git",
                    &ohmyzsh.to_string_lossy(),
                ],
            )?;
        } else {
            emit.info("Oh-My-Zsh already cloned.");
        }

        // 4. Write bundled configs.
        write_asset("ghostty/config", emit)?;
        write_asset("yazi/yazi.toml", emit)?;
        write_asset("yazi/keymap.toml", emit)?;
        write_asset("yazi/theme.toml", emit)?;

        // 5. Patch ~/.zshrc with managed blocks.
        patch_zshrc(emit)?;

        emit.info("Terminal suite configured. Open Ghostty (or restart your shell) to use it.");
        Ok(InstallOutcome::Installed)
    }
}

fn write_asset(rel: &str, emit: &dyn Emitter) -> Result<()> {
    let file = ASSETS
        .get_file(rel)
        .ok_or_else(|| crate::error::CoreError::other(format!("missing embedded asset {rel}")))?;
    let dest = dirs::config_dir()
        .map(|c| c.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dest, file.contents())?;
    emit.info(format!("Wrote {}", dest.display()));
    Ok(())
}

fn patch_zshrc(emit: &dyn Emitter) -> Result<()> {
    let zshrc = dirs::home_dir()
        .map(|h| h.join(".zshrc"))
        .unwrap_or_else(|| PathBuf::from(".zshrc"));
    let mut content = fs::read_to_string(&zshrc).unwrap_or_default();

    // Drop any previously-managed versions of both blocks.
    content = strip_block(&content, "# >>> mokio oh-my-zsh >>>", "# <<< mokio oh-my-zsh <<<");
    content = strip_block(&content, "# >>> mokio terminal setup >>>", "# <<< mokio terminal setup <<<");

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    for block in [OHMYZSH_BLOCK, TERMINAL_BLOCK] {
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(block);
        content.push('\n');
    }
    fs::write(&zshrc, content)?;
    emit.info(format!("Patched {}", zshrc.display()));
    Ok(())
}

fn strip_block(content: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = content;
    loop {
        match rest.find(open) {
            Some(start) => {
                out.push_str(&rest[..start]);
                let after = &rest[start..];
                match after.find(close) {
                    Some(end_rel) => rest = &after[end_rel + close.len()..],
                    None => {
                        out.push_str(after);
                        break;
                    }
                }
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

fn zshrc_has_marker(marker: &str) -> bool {
    dirs::home_dir()
        .map(|h| h.join(".zshrc"))
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|s| s.contains(marker))
        .unwrap_or(false)
}
