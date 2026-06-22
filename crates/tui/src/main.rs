//! mokio — one-click dev toolchain bootstrap.
//!
//! * `mokio`           → interactive TUI (default)
//! * `mokio list`      → print detected status of every tool and exit
//! * `mokio install [ids...]` → non-interactive install (all tools, or the given ids)

mod app;
mod ui;

use std::io;
use std::sync::mpsc::channel;
use std::time::Duration;

use crossterm::cursor::Hide;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use mokio_core::event::{Emitter, Event as CoreEvent};
use mokio_core::Catalog;

use crate::app::{App, Level, TuiMsg};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.as_slice() {
        [] => run_tui().map(|_| 0).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            1
        }),
        [cmd, rest @ ..] if cmd == "list" || cmd == "ls" => run_list(rest).map(|_| 0).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            1
        }),
        [cmd, rest @ ..] if cmd == "install" || cmd == "i" => {
            run_install(rest).map(|_| 0).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        [cmd, ..] if cmd == "-h" || cmd == "--help" || cmd == "help" => {
            print_help();
            0
        }
        _ => {
            eprintln!("unknown command. try: mokio help");
            2
        }
    };
    std::process::exit(code);
}

fn print_help() {
    println!("mokio — one-click dev toolchain bootstrap\n");
    println!("USAGE:");
    println!("    mokio                 Interactive TUI (default)");
    println!("    mokio list            Print detected status of every tool");
    println!("    mokio install [ids]   Install everything, or just the listed tool ids");
    println!();
    println!("TOOL IDS:");
    let catalog = Catalog::new();
    for info in catalog.infos() {
        println!("    {:<12} {}", info.id, info.name);
    }
}

// ---------------------------------------------------------------------------
// `mokio list`
// ---------------------------------------------------------------------------

fn run_list(_rest: &[String]) -> io::Result<()> {
    let catalog = Catalog::new();
    println!(
        "{:<4} {:<12} {:<22} {}",
        "", "ID", "NAME", "STATUS"
    );
    for info in catalog.infos() {
        let status = catalog.get(&info.id).map(|i| i.detect()).unwrap_or_default();
        let (glyph, detail) = fmt_status(&status);
        println!("{glyph:<4} {:<12} {:<22} {detail}", info.id, info.name);
    }
    Ok(())
}

fn fmt_status(status: &mokio_core::Status) -> (&'static str, String) {
    use mokio_core::Status;
    match status {
        Status::Installed { version: Some(v) } => ("✓", format!("installed ({v})")),
        Status::Installed { version: None } => ("✓", "installed".to_string()),
        Status::NotInstalled => ("○", "not installed".to_string()),
        Status::Unknown => ("·", "unknown".to_string()),
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

fn run_install(ids: &[String]) -> io::Result<()> {
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
        println!("✅ All tools installed successfully.");
    } else {
        eprintln!("⚠ Finished with failures: {}", failed.join(", "));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// `mokio` (TUI)
// ---------------------------------------------------------------------------

fn run_tui() -> Result<(), io::Error> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = channel::<TuiMsg>();
    let mut app = App::new();
    app.push_log(
        Level::Info,
        "Welcome to Mokio. Space toggles tools, 'i' installs them.",
    );
    app.push_log(
        Level::Info,
        "Selections auto-include prerequisites (e.g. Codex pulls in Node).",
    );

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
            if let Event::Key(key) = event::read()? {
                if should_quit(&key) {
                    return Ok(());
                }
                if !app.running {
                    handle_key(app, tx, &key);
                }
            }
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                TuiMsg::Start(id) => {
                    app.current = Some(id.clone());
                    app.push_log(Level::Phase, format!("starting {id}"));
                }
                TuiMsg::Event(e) => app.handle_event(&e),
                TuiMsg::DoneOne { id, ok } => {
                    app.done_count += 1;
                    if ok {
                        if let Some(inst) = app.catalog.get(&id) {
                            app.statuses.insert(id.clone(), inst.detect());
                        }
                        app.push_log(Level::Info, format!("✓ done: {id}"));
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
                        app.push_log(Level::Info, "✅ All selected tools installed successfully.");
                    } else {
                        app.push_log(
                            Level::Warn,
                            format!(
                                "Finished with {} failure(s): {}",
                                failed.len(),
                                failed.join(", ")
                            ),
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
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
