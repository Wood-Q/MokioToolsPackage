//! Internationalisation. Two languages — Chinese (`Zh`, the default) and
//! English (`En`) — with a single source of truth shared by the TUI, the
//! desktop app, and the non-interactive CLI.

use serde::{Deserialize, Serialize};

use crate::installer::Category;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Zh,
    En,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Lang::Zh => Lang::En,
            Lang::En => Lang::Zh,
        }
    }

    /// Word used on the language-switch control.
    pub fn switch_label(self) -> &'static str {
        // Always shows the *other* language, so a click toggles to it.
        match self {
            Lang::Zh => "EN",
            Lang::En => "中文",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "en" | "english" => Lang::En,
            _ => Lang::Zh,
        }
    }
}

/// Translate a UI chrome string by key. Unknown keys fall through to themselves
/// so missing translations are obvious during development.
pub fn ui(lang: Lang, key: &str) -> &'static str {
    match lang {
        Lang::Zh => zh(key),
        Lang::En => en(key),
    }
}

fn zh(key: &str) -> &'static str {
    match key {
        "tagline" => "一键配置 macOS 开发工具链",
        "panel_tools" => " 工具 ",
        "panel_details" => " 详情 ",
        "panel_log" => " 日志 ",
        "label_category" => "分类：  ",
        "label_homepage" => "主页：  ",
        "label_selected" => "已选：  ",
        "label_status" => "状态：  ",
        "sel_yes" => "是",
        "sel_no" => "否",
        "foundation" => " [基础]",
        "needs" => "  依赖：{list}",
        "st_installed_v" => "已安装（{v}）",
        "st_installed" => "已安装",
        "st_not_installed" => "未安装",
        "st_unknown" => "未知",
        "footer_help" => " ↑/↓ 移动 · Space 切换 · i 安装 · a 全选 · n 全不选 · r 重检测 · l 中/英 · [/] 日志大小 · 拖动日志边框 · q 退出 ",
        "hdr_ready" => "就绪 — Space 切换 · i 安装 · r 重检测 · q 退出",
        "hdr_finished" => "✓ 本次完成 — 按 <r> 重检测，<i> 再跑一次",
        "hdr_count" => " 已安装 {installed}/{total} 个工具 ",
        "gauge_label" => "正在安装 {current}（{done}/{total}）",
        "log_welcome" => "欢迎使用 Mokio。按 Space 切换工具，i 开始安装。",
        "log_tips" => "所选工具会自动带上依赖（例如选 Codex 会自动加 Node）。",
        "log_redetect" => "已重新检测所有工具状态。",
        "log_install_plan" => "即将安装 {n} 个工具：{list}",
        "log_starting" => "开始 {id}",
        "log_done" => "✓ 完成：{id}",
        "log_all_ok" => "✅ 所选工具全部安装成功。",
        "log_failed" => "完成，但有 {n} 个失败：{list}",
        "cat_foundational" => "基础",
        "cat_runtime" => "运行时与包管理器",
        "cat_vcs" => "版本控制",
        "cat_editor" => "编辑器",
        "cat_browser" => "浏览器",
        "cat_terminal" => "终端与 Shell",
        "cat_aicli" => "AI 编程代理",
        "lang_toggle" => "语言：中文（按 l 切换为英文）",
        "homepage" => "主页",
        "foundation_label" => "基础",
        // desktop
        "btn_install" => "安装所选",
        "btn_installing" => "安装中…",
        "btn_select_all" => "全选",
        "btn_clear" => "清空",
        "btn_redetect" => "重新检测",
        "btn_log_clear" => "清空",
        "summary" => "已安装 {installed}/{total} · 已选 {selected}",
        "footer_desktop" => "所选工具会自动带上依赖。终端套件会在 ~/.zshrc 写入受管理的片段。",
        // cli
        "usage" => "mokio — 一键配置 macOS 开发工具链\n",
        "cli_tui" => "    mokio                 交互式 TUI（默认）",
        "cli_list" => "    mokio list            打印每个工具的检测状态",
        "cli_install" => "    mokio install [ids]   安装全部，或只装指定的工具 id",
        "tool_ids_header" => "工具 ID：",
        "list_id" => "ID",
        "list_name" => "名称",
        "list_status" => "状态",
        "install_all_ok" => "✅ 所有工具安装成功。",
        "install_failed" => "⚠ 完成但有失败：{list}",
        "unknown_command" => "未知命令。试试：mokio help",
        _ => "",
    }
}

fn en(key: &str) -> &'static str {
    match key {
        "tagline" => "one-click dev toolchain bootstrap",
        "panel_tools" => " Tools ",
        "panel_details" => " Details ",
        "panel_log" => " Log ",
        "label_category" => "Category:  ",
        "label_homepage" => "Homepage:  ",
        "label_selected" => "Selected:  ",
        "label_status" => "Status:    ",
        "sel_yes" => "yes",
        "sel_no" => "no",
        "foundation" => " [foundation]",
        "needs" => "  needs: {list}",
        "st_installed_v" => "installed ({v})",
        "st_installed" => "installed",
        "st_not_installed" => "not installed",
        "st_unknown" => "unknown",
        "footer_help" => " ↑/↓ move · Space toggle · i install · a all · n none · r re-detect · l lang · [/] log size · drag log border · q quit ",
        "hdr_ready" => "ready — Space toggle · i install · r re-detect · q quit",
        "hdr_finished" => "✓ run finished — press <r> to re-detect, <i> to run again",
        "hdr_count" => " {installed}/{total} tools installed ",
        "gauge_label" => "Installing {current} ({done}/{total})",
        "log_welcome" => "Welcome to Mokio. Space toggles tools, 'i' installs them.",
        "log_tips" => "Selections auto-include prerequisites (e.g. Codex pulls in Node).",
        "log_redetect" => "Re-ran detection for all tools.",
        "log_install_plan" => "Installing {n} tool(s): {list}",
        "log_starting" => "starting {id}",
        "log_done" => "✓ done: {id}",
        "log_all_ok" => "✅ All selected tools installed successfully.",
        "log_failed" => "Finished with {n} failure(s): {list}",
        "cat_foundational" => "Foundational",
        "cat_runtime" => "Runtimes & package managers",
        "cat_vcs" => "Version control",
        "cat_editor" => "Editor",
        "cat_browser" => "Browser",
        "cat_terminal" => "Terminal & shell",
        "cat_aicli" => "AI coding agents",
        "lang_toggle" => "Language: English (press l for 中文)",
        "homepage" => "homepage",
        "foundation_label" => "foundation",
        "btn_install" => "Install selected",
        "btn_installing" => "Installing…",
        "btn_select_all" => "Select all",
        "btn_clear" => "Clear",
        "btn_redetect" => "Re-detect",
        "btn_log_clear" => "clear",
        "summary" => "{installed}/{total} installed · {selected} selected",
        "footer_desktop" => "Selections auto-include prerequisites. The terminal suite writes managed blocks into ~/.zshrc.",
        "usage" => "mokio — one-click dev toolchain bootstrap\n",
        "cli_tui" => "    mokio                 Interactive TUI (default)",
        "cli_list" => "    mokio list            Print detected status of every tool",
        "cli_install" => "    mokio install [ids]   Install everything, or just the listed tool ids",
        "tool_ids_header" => "TOOL IDS:",
        "list_id" => "ID",
        "list_name" => "NAME",
        "list_status" => "STATUS",
        "install_all_ok" => "✅ All tools installed successfully.",
        "install_failed" => "⚠ Finished with failures: {list}",
        "unknown_command" => "unknown command. try: mokio help",
        _ => "",
    }
}

/// Localised label for a tool category.
pub fn category_label(lang: Lang, cat: Category) -> &'static str {
    let key = match cat {
        Category::Foundational => "cat_foundational",
        Category::Runtime => "cat_runtime",
        Category::Vcs => "cat_vcs",
        Category::Editor => "cat_editor",
        Category::Browser => "cat_browser",
        Category::Terminal => "cat_terminal",
        Category::AiCli => "cat_aicli",
    };
    ui(lang, key)
}

/// Localised display name for a tool, keyed by installer id.
pub fn tool_name(lang: Lang, id: &str) -> &'static str {
    let pair: (&str, &str) = match id {
        "homebrew" => ("Homebrew", "Homebrew"),
        "git" => ("Git", "Git"),
        "node" => ("Node.js + npm（nvm）", "Node.js + npm (nvm)"),
        "uv" => ("uv", "uv"),
        "vscode" => ("Visual Studio Code", "Visual Studio Code"),
        "chrome" => ("Google Chrome", "Google Chrome"),
        "terminal" => ("终端套件", "Terminal suite"),
        "codex" => ("Codex（命令行 + 桌面端）", "Codex (CLI + Desktop)"),
        "claude-code" => ("Claude Code", "Claude Code"),
        "cc-switch" => ("CC Switch", "CC Switch"),
        _ => return "",
    };
    match lang {
        Lang::Zh => pair.0,
        Lang::En => pair.1,
    }
}

/// Localised description for a tool, keyed by installer id.
pub fn tool_desc(lang: Lang, id: &str) -> &'static str {
    let pair: (&str, &str) = match id {
        "homebrew" => (
            "macOS 缺失的包管理器——几乎所有其他工具都依赖它。",
            "The missing package manager for macOS — required by nearly everything else.",
        ),
        "git" => (
            "分布式版本控制系统。安装 Homebrew 版并确保 Xcode 命令行工具就绪。",
            "Distributed version control. Installs the Homebrew formula and ensures the Xcode Command Line Tools are present.",
        ),
        "node" => (
            "通过 nvm 管理的 Node.js LTS（含 npm），复刻你现有的 ~/.nvm 环境。",
            "Node.js LTS (with npm) managed by nvm, the Node Version Manager. Replicates your current ~/.nvm setup.",
        ),
        "uv" => (
            "极速的 Python 包安装器 / 解析器（Astral 出品）。",
            "Extremely fast Python package installer / resolver (Astral). Replaces pip + virtualenv workflows.",
        ),
        "vscode" => (
            "代码编辑器。安装 cask 并配置好 code 命令行。",
            "The editor. Installs the cask and wires up the `code` CLI.",
        ),
        "chrome" => (
            "大多数 Web 工具链默认的浏览器。",
            "The browser most web tooling expects.",
        ),
        "terminal" => (
            "Ghostty + Maple Mono NF CN + Yazi（含预览依赖）+ Oh-My-Zsh + 自动建议/语法高亮/thefuck/zoxide，并写入你的 Ghostty、Yazi 配置和受管理的 ~/.zshrc 片段。",
            "Ghostty + Maple Mono NF CN + Yazi (with preview deps) + Oh-My-Zsh + autosuggestions/syntax-highlighting/thefuck/zoxide, plus your exact Ghostty & Yazi configs and a managed ~/.zshrc block.",
        ),
        "codex" => (
            "OpenAI Codex 编程代理——命令行版（npm）加桌面端应用。",
            "OpenAI Codex coding agent — the `codex` CLI via npm, plus the Codex desktop app.",
        ),
        "claude-code" => (
            "Anthropic 的命令行编程工具（claude）。",
            "Anthropic's agentic command-line coding tool (`claude`).",
        ),
        "cc-switch" => (
            "切换 Claude Code / Codex / Gemini CLI 供应商配置的桌面应用。从 GitHub 最新发布版拉取（已签名公证）。",
            "Desktop app for switching Claude Code / Codex / Gemini CLI provider configs. Pulled from the latest GitHub release (signed + notarized).",
        ),
        _ => return "",
    };
    match lang {
        Lang::Zh => pair.0,
        Lang::En => pair.1,
    }
}
