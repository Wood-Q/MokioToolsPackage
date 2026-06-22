# Mokio

> One-click bootstrap for a complete macOS dev toolchain — TUI **and** desktop.

Mokio installs and configures everything a developer's Mac needs, in one pass,
with idempotent re-runs and live progress:

| # | Tool | How Mokio installs it |
|---|------|-----------------------|
| 1 | **Visual Studio Code** | `brew install --cask visual-studio-code` |
| 2 | **Git** | `brew install git` (+ ensures Xcode Command Line Tools) |
| 3 | **Google Chrome** | `brew install --cask google-chrome` |
| 4 | **Terminal suite** | Ghostty + Maple Mono NF CN font + Yazi (with preview deps) + Oh-My-Zsh + zsh-autosuggestions / syntax-highlighting / thefuck / zoxide, **plus your exact Ghostty & Yazi configs and a managed `~/.zshrc` block** |
| 5 | **Codex** (CLI + Desktop) | `npm i -g @openai/codex` + `brew install --cask codex-app` |
| 6 | **Claude Code** | `npm i -g @anthropic-ai/claude-code` |
| 7 | **cc-switch** | latest signed `.zip` release from `farion1231/cc-switch` → `/Applications/CC Switch.app` |
| 8 | **Node.js** | nvm + Node LTS (replicates an `~/.nvm` setup) |
| 9 | **uv** | `brew install uv` |

Homebrew is auto-installed first as the foundation everything else leans on.

---

## Two front-ends, one core

All install logic lives in [`crates/core`](crates/core) (`mokio-core`). Two thin
front-ends render it:

- **`mokio`** — an interactive TUI built with [ratatui](https://ratatui.rs).
- **`mokio-desktop`** — a native desktop app built with [Tauri 2](https://tauri.app).

Both show live detection, let you pick tools, and stream install output in real
time. There's also a non-interactive CLI mode for scripts / CI.

---

## Requirements

- macOS 13+ on Apple Silicon or Intel
- Rust toolchain (`rustup`; Mokio builds with stable, currently tested on 1.96)
- Xcode Command Line Tools (for the system compiler Git uses): `xcode-select --install`

> Mokio installs Homebrew itself if missing, so you don't need it up front.

---

## Build

```bash
# TUI + core
cargo build --release -p mokio-tui
# → ./target/release/mokio

# Desktop (Tauri) — links the system WebKit
cargo build --release -p mokio-desktop
# → ./target/release/mokio-desktop
```

Everything:

```bash
cargo build --release
```

### Bundle the desktop app into a `.app` / `.dmg`

The Tauri CLI produces a distributable bundle (icons are committed in
[`desktop/icons`](desktop/icons); regenerate with
`python3 desktop/icons/gen_icons.py`):

```bash
cargo install tauri-cli --version "^2" --locked
cargo tauri build            # run from the repo root; app config is desktop/tauri.conf.json
```

Output appears under `desktop/target/release/bundle/`.

---

## Use

### Interactive TUI

```bash
./target/release/mokio
```

Keys: `↑/↓` move · `Space` toggle · `i` install · `a` all · `n` none · `r`
re-detect · **`l` switch 中文 / English** · `q` quit. Selecting a tool
auto-includes its prerequisites (e.g. Codex pulls in Node).

### Non-interactive

```bash
mokio list                       # print detected status of every tool
mokio install                    # install everything
mokio install node uv claude-code   # install a subset (prerequisites auto-added)
```

### Language

The UI defaults to **中文**. Switch it:

- **TUI** — press `l`.
- **Desktop** — the `EN` / `中文` button in the top bar.
- **CLI** — pass `--lang en` (or `-L en`) anywhere: `mokio list --lang en`.

Translations live in one shared module, [`crates/core/src/i18n.rs`](crates/core/src/i18n.rs).



### Desktop

```bash
./target/release/mokio-desktop
```

---

## The terminal suite, in detail

This is item #4 and the most opinionated step. It reproduces the developer's
existing Ghostty + Yazi + Oh-My-Zsh setup on any Mac:

1. `brew install --cask ghostty`
2. `brew install --cask font-maple-mono-nf-cn`
3. `brew install yazi ffmpeg sevenzip jq poppler fd ripgrep fzf zoxide imagemagick zsh-autosuggestions zsh-syntax-highlighting thefuck`
4. Clones Oh-My-Zsh into `~/.oh-my-zsh` (without running its `install.sh`, so it
   never clobbers your rc files).
5. Writes the bundled configs:
   - `~/.config/ghostty/config` — Catppuccin Mocha, Maple Mono NF CN, Quake-style
     quick terminal, full keybind set.
   - `~/.config/yazi/{yazi,keymap,theme}.toml` — your ratios, openers, git
     fetcher, `g h`/`g c`/… jump keys.
6. Patches `~/.zshrc` with two **managed blocks** delimited by markers:
   - `# >>> mokio oh-my-zsh >>>` … `# <<< mokio oh-my-zsh <<<`
   - `# >>> mokio terminal setup >>>` … `# <<< mokio terminal setup <<<`

   The setup block sets `LANG`, a dynamic `DEFAULT_USER`, the Ghostty
   cwd-window-title hook, the Yazi `y()` wrapper that cds into the last
   directory, nvm loading, `compinit`, autosuggestions + syntax-highlighting
   (loaded last), `thefuck`, and `zoxide`.

Re-running the installer **replaces** anything between those markers and leaves
the rest of your `~/.zshrc` untouched.

> It does **not** run `chsh` for you (that needs your password interactively).
> If `zsh` isn't your default shell, run `chsh -s /bin/zsh` once.

---

## How it works

```
crates/core          installer trait, shell/detect helpers, all 10 tool modules,
                     embedded Ghostty/Yazi configs (include_dir)
crates/tui           ratatui TUI + non-interactive `list`/`install` subcommands
desktop              Tauri 2 shell; bridges core events to the webview
  └─ frontend        static HTML/CSS/JS (no bundler) — talks to Rust via Tauri
```

- Every tool is an [`Installer`](crates/core/src/installer.rs) with `detect()`
  and idempotent `install()`. The UIs are dumb renderers over a shared catalog.
- Subprocess output is streamed line-by-line through an `Emitter` trait — the
  TUI pipes it to a channel, the desktop pipes it to Tauri events.
- Prerequisites are resolved transitively via
  [`Catalog::expand_with_deps`](crates/core/src/catalog.rs).

---

## Project layout

```
.
├── Cargo.toml              # workspace
├── crates/
│   ├── core/               # mokio-core: all installation logic + embedded configs
│   │   ├── src/
│   │   │   ├── tools/      # one module per tool
│   │   │   └── ...
│   │   └── assets/         # ghostty/ + yazi/ configs baked in at compile time
│   └── tui/                # `mokio` binary (ratatui)
└── desktop/                # Tauri 2 app (`mokio-desktop`)
    ├── src/
    ├── frontend/           # index.html, styles.css, main.js
    ├── icons/              # committed app icons (+ gen_icons.py to regenerate)
    └── tauri.conf.json
```

---

## Notes & caveats

- `brew install --cask` for GUI apps (VS Code, Chrome, Ghostty, Codex) may
  prompt for your password once via macOS; Mokio can't type it for you, so run
  the TUI/desktop in a real terminal session.
- Codex's desktop cask (`codex-app`) is installed best-effort — if it's
  unavailable on your macOS version, Mokio warns and continues with the CLI.
- cc-switch is downloaded from GitHub's release API, which is rate-limited (60
  requests/hour per IP) when unauthenticated. Normal use is fine.

## License

MIT — see [LICENSE](LICENSE).
