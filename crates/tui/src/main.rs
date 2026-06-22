//! mokio — one-click dev toolchain bootstrap.
//!
//! * `mokio`                    → interactive TUI (default, Chinese)
//! * `mokio list`               → print detected status of every tool and exit
//! * `mokio install [ids...]`   → non-interactive install (all tools, or given ids)
//!
//! Pass `--lang en` (or `-L en`) anywhere to switch to English; `--lang zh` for 中文.

mod app;
mod ui;

use std::io;
use std::sync::mpsc::channel;
use std::time::Duration;

use crossterm::cursor::Hide;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use mokio_core::event::{Emitter, Event as CoreEvent};
use mokio_core::i18n;
use mokio_core::{Catalog, Lang};

use crate::app::{App, Level, TuiMsg};

fn main() {
    // Pull a `--lang`/`-L` flag out of the arg list (anywhere), default 中文.
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut lang = Lang::default();
    let mut args: Vec<String> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--lang" | "-L" => {
                if i + 1 < raw.len() {
                    lang = Lang::from_str(&raw[i + 1]);
                    i += 2;
                    continue;
                }
            }
            s if s.starts_with("--lang=") => {
                lang = Lang::from_str(&s["--lang=".len()..]);
                i += 1;
                continue;
            }
            _ => {}
        }
        args.push(raw[i].clone());
        i += 1;
    }

    let code = match args.as_slice() {
        [] => run_tui(lang).map(|_| 0).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            1
        }),
        [cmd, rest @ ..] if cmd == "list" || cmd == "ls" => {
            run_list(lang, rest).map(|_| 0).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        [cmd, rest @ ..] if cmd == "install" || cmd == "i" => {
            run_install(lang, rest).map(|_| 0).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        [cmd, ..] if cmd == "-h" || cmd == "--help" || cmd == "help" => {
            print_help(lang);
            0
        }
        _ => {
            eprintln!("{}", i18n::ui(lang, "unknown_command"));
            2
        }
    };
    std::process::exit(code);
}

fn print_help(lang: Lang) {
    println!("{}", i18n::ui(lang, "usage"));
    println!("USAGE:");
    println!("{}", i18n::ui(lang, "cli_tui"));
    println!("{}", i18n::ui(lang, "cli_list"));
    println!("{}", i18n::ui(lang, "cli_install"));
    println!();
    println!("{}  (-L zh | -L en)", i18n::ui(lang, "tool_ids_header"));
    let catalog = Catalog::new();
    for info in catalog.localized_infos(lang) {
        println!("    {:<12} {}", info.id, info.name);
    }
}

// ---------------------------------------------------------------------------
// `mokio list`
// ---------------------------------------------------------------------------

fn run_list(lang: Lang, _rest: &[String]) -> io::Result<()> {
    let catalog = Catalog::new();
    println!(
        "{:<4} {:<12} {:<22} {}",
        "",
        i18n::ui(lang, "list_id"),
        i18n::ui(lang, "list_name"),
        i18n::ui(lang, "list_status")
    );
    for info in catalog.localized_infos(lang) {
        let status = catalog.get(&info.id).map(|i| i.detect()).unwrap_or_default();
        let (glyph, detail) = fmt_status(lang, &status);
        println!("{glyph:<4} {:<12} {:<22} {detail}", info.id, info.name);
    }
    Ok(())
}

fn fmt_status(lang: Lang, status: &mokio_core::Status) -> (&'static str, String) {
    use mokio_core::Status;
    match status {
        Status::Installed { version: Some(v) } => (
            "✓",
            i18n::ui(lang, "st_installed_v").replace("{v}", v),
        ),
        Status::Installed { version: None } => ("✓", i18n::ui(lang, "st_installed").to_string()),
        Status::NotInstalled => ("○", i18n::ui(lang, "st_not_installed").to_string()),
        Status::Unknown => ("·", i18n::ui(lang, "st_unknown").to_string()),
    }
}

// ---------------------------------------------------------------------------
// `mokio install [ids...]`
// ---------------------------------------------------------------------------

struct StdEmitter;
impl Emitter for StdEmitter {
    fn emit(&self, event: CoreEvent) {
        match event {
            CoreEvent::Phase(s) => println!("▶ {s}"),
            CoreEvent::Info(s) => println!("  {s}"),
            CoreEvent::Log(s) => println!("  {s}"),
            CoreEvent::Warn(s) => eprintln!("! {s}"),
            CoreEvent::Status { .. } => {}
            CoreEvent::Progress { done, total } => println!("  [{done}/{total}]"),
        }
    }
}

fn run_install(lang: Lang, ids: &[String]) -> io::Result<()> {
    let catalog = Catalog::new();
    let requested: Vec<String> = if ids.is_empty() {
        catalog.infos().iter().map(|i| i.id.clone()).collect()
    } else {
        ids.iter().cloned().collect()
    };
    let ordered = catalog.expand_with_deps(&requested);

    let emitter = StdEmitter;
    let emit: &dyn Emitter = &emitter;
    let mut failed: Vec<String> = Vec::new();

    for id in &ordered {
        println!("\n=== {id} ===");
        match catalog.get(id) {
            Some(inst) => {
                if let Err(e) = inst.install(emit) {
                    eprintln!("✗ {id} failed: {e}");
                    failed.push(id.clone());
                }
            }
            None => {
                eprintln!("unknown tool: {id}");
                failed.push(id.clone());
            }
        }
    }

    println!();
    if failed.is_empty() {
        println!("{}", i18n::ui(lang, "install_all_ok"));
    } else {
        eprintln!(
            "{}",
            i18n::ui(lang, "install_failed").replace("{list}", &failed.join(", "))
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// `mokio` (TUI)
// ---------------------------------------------------------------------------

fn run_tui(lang: Lang) -> Result<(), io::Error> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = channel::<TuiMsg>();
    let mut app = App::new();
    app.set_lang(lang);
    app.push_log(Level::Info, i18n::ui(app.lang, "log_welcome"));
    app.push_log(Level::Info, i18n::ui(app.lang, "log_tips"));

    let result = run_tui_loop(&mut terminal, &mut app, &tx, &rx);
    restore_terminal()?;
    result
}

fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tx: &std::sync::mpsc::Sender<TuiMsg>,
    rx: &std::sync::mpsc::Receiver<TuiMsg>,
) -> Result<(), io::Error> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(60))? {
            match event::read()? {
                Event::Key(key) => {
                    if should_quit(&key) {
                        return Ok(());
                    }
                    if !app.running {
                        handle_key(app, tx, &key);
                    }
                }
                Event::Mouse(m) => {
                    // Grabbing / dragging the log panel's top border resizes it.
                    if matches!(m.kind, MouseEventKind::Down(_) | MouseEventKind::Drag(_)) {
                        let h = terminal.size()?.height;
                        let divider = h.saturating_sub(app.log_height).saturating_sub(1);
                        if (m.row as i32 - divider as i32).abs() <= 1 {
                            app.drag_log_to(m.row, h);
                        }
                    }
                }
                _ => {}
            }
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                TuiMsg::Start(id) => {
                    app.current = Some(id.clone());
                    app.push_log(
                        Level::Phase,
                        i18n::ui(app.lang, "log_starting").replace("{id}", &id),
                    );
                }
                TuiMsg::Event(e) => app.handle_event(&e),
                TuiMsg::DoneOne { id, ok } => {
                    app.done_count += 1;
                    if ok {
                        if let Some(inst) = app.catalog.get(&id) {
                            app.statuses.insert(id.clone(), inst.detect());
                        }
                        app.push_log(
                            Level::Info,
                            i18n::ui(app.lang, "log_done").replace("{id}", &id),
                        );
                    }
                    app.current = None;
                }
                TuiMsg::AllFinished { failed } => {
                    app.running = false;
                    app.finished = true;
                    app.current = None;
                    for info in &app.infos {
                        if let Some(inst) = app.catalog.get(&info.id) {
                            app.statuses.insert(info.id.clone(), inst.detect());
                        }
                    }
                    if failed.is_empty() {
                        app.push_log(Level::Info, i18n::ui(app.lang, "log_all_ok"));
                    } else {
                        app.push_log(
                            Level::Warn,
                            i18n::ui(app.lang, "log_failed")
                                .replace("{n}", &failed.len().to_string())
                                .replace("{list}", &failed.join(", ")),
                        );
                    }
                }
            }
        }
    }
}

fn handle_key(app: &mut App, tx: &std::sync::mpsc::Sender<TuiMsg>, key: &KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => app.move_cursor(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_cursor(-1),
        KeyCode::Char(' ') => app.toggle_selected(),
        KeyCode::Char('i') | KeyCode::Enter => app.start_install(tx.clone()),
        KeyCode::Char('a') => app.select_all(true),
        KeyCode::Char('n') => app.select_all(false),
        KeyCode::Char('r') => app.redetect(),
        KeyCode::Char('l') => app.toggle_lang(),
        KeyCode::Char(']') => app.grow_log(),
        KeyCode::Char('[') => app.shrink_log(),
        _ => {}
    }
}

fn should_quit(key: &KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

fn restore_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;
    Ok(())
}
