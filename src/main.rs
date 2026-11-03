//! Lera, the Asciiverse MUD client.
//!
//! `lera` opens the world picker; `lera <world-id>` connects to a world from
//! the catalog or your own list (av's Connect runs this); `lera <host> <port>
//! [--tls]` connects anywhere.

mod app;
mod net;
mod paths;
mod scrollback;
mod theme;
mod ui;
mod worlds;

use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use ratatui::crossterm::execute;

use app::App;
use worlds::World;

const USAGE: &str = "\
Lera, the Asciiverse MUD client

usage:
  lera                     choose a world
  lera <world-id>          connect to a world from the list
  lera <host> <port>       connect anywhere
       [--tls]             ...over TLS

Run with RUST_LOG=lera=debug to write a session log to lera.log in the
Asciiverse cache folder.";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("lera {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let paths = paths::Paths::new()?;
    init_logging(&paths);
    log::info!("lera {} starting, args {args:?}", env!("CARGO_PKG_VERSION"));

    let mut app = App::new(paths);
    let tls = args.iter().any(|a| a == "--tls");
    let rest: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let target = match rest.as_slice() {
        [] => None,
        [id] => match app.find_world(id) {
            Some(w) => Some(w),
            None => bail!("no world called {id:?}; run lera without arguments to see the list"),
        },
        [host, port] => {
            let Ok(port) = port.parse::<u16>() else { bail!("{port:?} is not a port number") };
            let known = app.worlds.iter().find(|w| w.host == **host && (w.port == port || w.tls_port == Some(port))).cloned();
            Some(known.unwrap_or_else(|| World {
                id: worlds::slug(host),
                name: host.to_string(),
                host: host.to_string(),
                port,
                tls,
                ..Default::default()
            }))
        }
        _ => bail!("{USAGE}"),
    };

    let mut terminal = ratatui::init();
    // the mouse wheel scrolls back (selecting text then takes Shift+drag);
    // a panic leaves mouse reporting off again, as ratatui's hook leaves raw mode
    let _ = execute!(std::io::stdout(), EnableMouseCapture);
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        hook(info);
    }));
    if let Some(w) = target {
        let size = terminal.size()?;
        app.connect_sized(w, (size.width, size.height.saturating_sub(2)));
    }
    let res = run(&mut terminal, &mut app);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    res
}

/// Session logging per the Asciiverse convention: with RUST_LOG set, every
/// key, telnet negotiation and state change goes to lera.log in the cache dir.
fn init_logging(paths: &paths::Paths) {
    if std::env::var_os("RUST_LOG").is_none() {
        return;
    }
    let file = paths.cache.join("lera.log");
    if let Ok(f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        env_logger::Builder::from_default_env().target(env_logger::Target::Pipe(Box::new(f))).init();
    }
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    let mut last_snapshot = Instant::now();
    while !app.quit {
        app.tick();
        terminal.draw(|f| ui::draw(f, app))?;
        // short frames: MUD text should appear the moment it arrives
        let mut wait = Duration::from_millis(16);
        while event::poll(wait)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => app.on_key(k),
                Event::Mouse(m) => app.on_mouse(m),
                _ => {}
            }
            if app.quit {
                break;
            }
            wait = Duration::ZERO;
        }
        if last_snapshot.elapsed() > Duration::from_millis(500) {
            last_snapshot = Instant::now();
            app.snapshot();
        }
    }
    Ok(())
}
