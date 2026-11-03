//! Lera's state and key handling: the world picker and a session in a world.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::net::{self, NetCmd, NetEvent};
use crate::paths::Paths;
use crate::scrollback::Scrollback;
use crate::worlds::{self, World};

/// rows one notch of the mouse wheel scrolls
const WHEEL_ROWS: usize = 3;

pub enum Screen {
    Picker,
    Session,
}

pub struct App {
    pub paths: Paths,
    pub screen: Screen,
    pub worlds: Vec<World>,
    pub has_catalog: bool,
    pub picker: Picker,
    pub session: Option<Session>,
    pub help: bool,
    pub quit: bool,
    /// Ctrl+Q while connected asks once more
    quit_armed: Option<Instant>,
    /// output size for a session started from the picker or the command line
    pub view: (u16, u16),
}

#[derive(Default)]
pub struct Picker {
    pub selected: usize,
    pub filter: String,
    pub form: Option<Form>,
    pub message: String,
}

/// Adding a world by hand.
#[derive(Default)]
pub struct Form {
    pub field: usize,
    pub name: String,
    pub host: String,
    pub port: String,
    pub tls: bool,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Conn {
    Connecting,
    Connected { tls: bool },
    Closed(String),
}

pub struct Session {
    pub world: World,
    pub conn: Conn,
    pub compressed: bool,
    net: Option<Sender<NetCmd>>,
    events: Receiver<NetEvent>,
    pub out: Scrollback,
    pub input: Input,
    history: Vec<String>,
    history_pos: Option<usize>,
    /// the server echoes, so it is asking for a password: mask and don't log
    pub server_echo: bool,
    /// rows scrolled back from the newest
    pub scroll: usize,
    /// rows of output the screen showed last frame, for NAWS and paging
    pub view: (u16, u16),
    sent_view: (u16, u16),
    /// GMCP Char.Vitals and Char.MaxStats, keys lowercased
    pub vitals: BTreeMap<String, String>,
    log: Option<std::fs::File>,
}

#[derive(Default)]
pub struct Input {
    pub text: String,
    /// cursor position in chars
    pub cursor: usize,
}

impl Input {
    fn byte(&self, ch: usize) -> usize {
        self.text.char_indices().nth(ch).map(|(i, _)| i).unwrap_or(self.text.len())
    }
    fn len(&self) -> usize {
        self.text.chars().count()
    }
    fn insert(&mut self, c: char) {
        let b = self.byte(self.cursor);
        self.text.insert(b, c);
        self.cursor += 1;
    }
    fn set(&mut self, s: &str) {
        self.text = s.to_string();
        self.cursor = self.len();
    }
    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let b = self.byte(self.cursor);
            self.text.remove(b);
        }
    }
    fn delete(&mut self) {
        if self.cursor < self.len() {
            let b = self.byte(self.cursor);
            self.text.remove(b);
        }
    }
    fn delete_word(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = self.cursor;
        while i > 0 && chars[i - 1] == ' ' {
            i -= 1;
        }
        while i > 0 && chars[i - 1] != ' ' {
            i -= 1;
        }
        self.text = chars[..i].iter().chain(&chars[self.cursor..]).collect();
        self.cursor = i;
    }
    /// Shared line editing keys; true if the key was one of them.
    fn edit(&mut self, k: &KeyEvent) -> bool {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('a') if ctrl => self.cursor = 0,
            KeyCode::Char('e') if ctrl => self.cursor = self.len(),
            KeyCode::Char('u') if ctrl => self.set(""),
            KeyCode::Char('w') if ctrl => self.delete_word(),
            KeyCode::Char(c) if !ctrl => self.insert(c),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.len()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.len(),
            _ => return false,
        }
        true
    }
}

impl App {
    pub fn new(paths: Paths) -> Self {
        let mut app = App {
            paths,
            screen: Screen::Picker,
            worlds: vec![],
            has_catalog: false,
            picker: Picker::default(),
            session: None,
            help: false,
            quit: false,
            quit_armed: None,
            view: (80, 24),
        };
        app.load_worlds();
        app
    }

    fn load_worlds(&mut self) {
        let mut ws = worlds::from_catalog(&self.paths.catalog());
        self.has_catalog = !ws.is_empty();
        ws.extend(worlds::load_own(&self.paths.worlds_file()));
        log::info!("worlds: {} ({} from the catalog)", ws.len(), ws.iter().filter(|w| w.from_catalog).count());
        self.worlds = ws;
    }

    pub fn find_world(&self, id: &str) -> Option<World> {
        self.worlds.iter().find(|w| w.id == id).cloned()
    }

    /// Worlds matching the picker's filter, as indexes into `worlds`.
    pub fn visible(&self) -> Vec<usize> {
        let f = self.picker.filter.to_lowercase();
        (0..self.worlds.len())
            .filter(|&i| {
                let w = &self.worlds[i];
                f.is_empty() || w.name.to_lowercase().contains(&f) || w.host.contains(&f) || w.description.to_lowercase().contains(&f)
            })
            .collect()
    }

    /// Connect before the first frame, when only the terminal size is known.
    pub fn connect_sized(&mut self, world: World, view: (u16, u16)) {
        self.view = view;
        self.connect(world);
    }

    pub fn connect(&mut self, world: World) {
        let (port, tls) = world.endpoint();
        log::info!("session start {} {}:{} tls={tls}", world.id, world.host, port);
        let (tx, rx) = std::sync::mpsc::channel();
        let view = self.session.as_ref().map(|s| s.view).unwrap_or(self.view);
        let net = net::spawn(world.host.clone(), port, tls, view, tx);
        let log = open_log(&self.paths, &world);
        let mut out = Scrollback::default();
        out.note(&format!("connecting to {} at {}:{}{}", world.name, world.host, port, if tls { " with TLS" } else { "" }));
        self.session = Some(Session {
            world,
            conn: Conn::Connecting,
            compressed: false,
            net: Some(net),
            events: rx,
            out,
            input: Input::default(),
            history: vec![],
            history_pos: None,
            server_echo: false,
            scroll: 0,
            view,
            sent_view: view,
            vitals: BTreeMap::new(),
            log,
        });
        self.screen = Screen::Session;
    }

    /// Take in what the connection thread has sent since the last frame.
    pub fn tick(&mut self) {
        let Some(s) = &mut self.session else { return };
        while let Ok(ev) = s.events.try_recv() {
            match ev {
                NetEvent::Connected { tls } => {
                    log::info!("state connected tls={tls}");
                    s.conn = Conn::Connected { tls };
                }
                NetEvent::Text(t) => s.out.feed(&t),
                NetEvent::Prompt => {}
                NetEvent::ServerEcho(on) => {
                    log::info!("state server echo {on}");
                    s.server_echo = on;
                }
                NetEvent::Gmcp(pkg, json) => s.on_gmcp(&pkg, &json),
                NetEvent::Compressed(on) => s.compressed = on,
                NetEvent::Closed(reason) => {
                    log::info!("state closed: {reason}");
                    s.out.note(&format!("{reason}. Ctrl+R reconnects, Esc goes back to the worlds."));
                    s.conn = Conn::Closed(reason);
                    s.net = None;
                    s.server_echo = false;
                    s.compressed = false;
                }
            }
        }
        if let Some(f) = &mut s.log {
            for line in s.out.finished.drain(..) {
                let _ = writeln!(f, "{line}");
            }
        } else {
            s.out.finished.clear();
        }
        if s.view != s.sent_view {
            s.sent_view = s.view;
            if let Some(n) = &s.net {
                let _ = n.send(NetCmd::Naws(s.view.0, s.view.1));
            }
        }
    }

    /// The periodic state snapshot of the logging convention.
    pub fn snapshot(&self) {
        match &self.session {
            Some(s) => log::trace!(
                "snapshot world={} conn={:?} scroll={} input_len={} echo={} history={} vitals={:?}",
                s.world.id,
                s.conn,
                s.scroll,
                s.input.text.chars().count(),
                s.server_echo,
                s.history_len(),
                s.vitals
            ),
            None => log::trace!("snapshot picker selected={} filter={:?} worlds={}", self.picker.selected, self.picker.filter, self.worlds.len()),
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) {
        log::debug!("key {:?} {:?}", k.code, k.modifiers);
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if k.code == KeyCode::F(1) {
            self.help = !self.help;
            return;
        }
        if self.help {
            self.help = false;
            return;
        }
        if ctrl && matches!(k.code, KeyCode::Char('q') | KeyCode::Char('c')) {
            let live = matches!(self.session.as_ref().map(|s| &s.conn), Some(Conn::Connected { .. } | Conn::Connecting));
            let armed = self.quit_armed.is_some_and(|t| t.elapsed().as_secs() < 3);
            if !live || armed {
                log::info!("quit");
                self.quit = true;
            } else {
                self.quit_armed = Some(Instant::now());
                if let Some(s) = &mut self.session {
                    s.out.note("still connected: press Ctrl+Q again to quit");
                }
            }
            return;
        }
        match self.screen {
            Screen::Picker => self.picker_key(k),
            Screen::Session => self.session_key(k),
        }
    }

    /// The mouse wheel: scrollback in a session, the list in the picker.
    /// Clicks and motion are ignored.
    pub fn on_mouse(&mut self, m: MouseEvent) {
        let up = match m.kind {
            MouseEventKind::ScrollUp => true,
            MouseEventKind::ScrollDown => false,
            _ => return,
        };
        log::debug!("wheel {}", if up { "up" } else { "down" });
        if self.help {
            return;
        }
        match self.screen {
            Screen::Session => {
                if let Some(s) = &mut self.session {
                    s.scroll = if up { s.scroll + WHEEL_ROWS } else { s.scroll.saturating_sub(WHEEL_ROWS) };
                }
            }
            Screen::Picker if self.picker.form.is_none() => {
                let last = self.visible().len().saturating_sub(1);
                let p = &mut self.picker;
                p.selected = if up { p.selected.saturating_sub(1) } else { (p.selected + 1).min(last) };
            }
            Screen::Picker => {}
        }
    }

    fn picker_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if self.picker.form.is_some() {
            return self.form_key(k);
        }
        let visible = self.visible();
        let p = &mut self.picker;
        match k.code {
            KeyCode::Up => p.selected = p.selected.saturating_sub(1),
            KeyCode::Down => p.selected = (p.selected + 1).min(visible.len().saturating_sub(1)),
            KeyCode::PageUp => p.selected = p.selected.saturating_sub(10),
            KeyCode::PageDown => p.selected = (p.selected + 10).min(visible.len().saturating_sub(1)),
            KeyCode::Enter => {
                if let Some(&i) = visible.get(p.selected) {
                    let w = self.worlds[i].clone();
                    self.connect(w);
                }
            }
            KeyCode::Char('n') if ctrl => p.form = Some(Form { port: "23".into(), ..Default::default() }),
            KeyCode::Char('d') if ctrl => {
                if let Some(&i) = visible.get(p.selected) {
                    if self.worlds[i].from_catalog {
                        p.message = "Catalog worlds come from Asciiverse; only your own can be removed".into();
                    } else {
                        let w = self.worlds.remove(i);
                        log::info!("world removed {}", w.id);
                        p.message = format!("Removed {}", w.name);
                        if let Err(e) = worlds::save_own(&self.paths.worlds_file(), &self.worlds) {
                            p.message = format!("Could not save worlds.toml: {e}");
                        }
                    }
                }
            }
            KeyCode::Esc => {
                if p.filter.is_empty() {
                    if self.session.is_some() {
                        self.screen = Screen::Session;
                    } else {
                        self.quit = true;
                    }
                } else {
                    p.filter.clear();
                    p.selected = 0;
                }
            }
            KeyCode::Backspace => {
                p.filter.pop();
                p.selected = 0;
            }
            KeyCode::Char(c) if !ctrl => {
                p.filter.push(c);
                p.selected = 0;
            }
            _ => {}
        }
    }

    fn form_key(&mut self, k: KeyEvent) {
        let Some(f) = &mut self.picker.form else { return };
        match k.code {
            KeyCode::Esc => self.picker.form = None,
            KeyCode::Tab | KeyCode::Down => f.field = (f.field + 1) % 4,
            KeyCode::BackTab | KeyCode::Up => f.field = (f.field + 3) % 4,
            KeyCode::Char(' ') if f.field == 3 => f.tls = !f.tls,
            KeyCode::Enter => {
                let port = f.port.trim().parse::<u16>();
                let (name, host) = (f.name.trim().to_string(), f.host.trim().to_string());
                if name.is_empty() || host.is_empty() {
                    f.error = "A world needs a name and a host".into();
                } else if port.is_err() {
                    f.error = "The port is a number from 1 to 65535".into();
                } else {
                    let w = World { id: worlds::slug(&name), name, host, port: port.unwrap(), tls: f.tls, ..Default::default() };
                    log::info!("world added {} {}:{}", w.id, w.host, w.port);
                    self.picker.message = format!("Added {}", w.name);
                    self.worlds.push(w);
                    if let Err(e) = worlds::save_own(&self.paths.worlds_file(), &self.worlds) {
                        self.picker.message = format!("Could not save worlds.toml: {e}");
                    }
                    self.picker.form = None;
                    self.picker.filter.clear();
                    self.picker.selected = self.visible().len().saturating_sub(1);
                }
            }
            KeyCode::Backspace => {
                field(f).pop();
            }
            KeyCode::Char(c) if f.field != 3 && !k.modifiers.contains(KeyModifiers::CONTROL) => {
                if f.field != 2 || c.is_ascii_digit() {
                    field(f).push(c);
                }
            }
            _ => {}
        }

        fn field(f: &mut Form) -> &mut String {
            match f.field {
                0 => &mut f.name,
                1 => &mut f.host,
                _ => &mut f.port,
            }
        }
    }

    fn session_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let Some(s) = &mut self.session else { return };
        let page = (s.view.1 as usize).saturating_sub(2).max(1);
        match k.code {
            KeyCode::Char('r') if ctrl => {
                let w = s.world.clone();
                s.net = None;
                self.connect(w);
            }
            KeyCode::Char('d') if ctrl && s.net.is_some() => {
                log::info!("disconnect");
                s.net = None;
            }
            KeyCode::PageUp => s.scroll += page,
            KeyCode::PageDown => s.scroll = s.scroll.saturating_sub(page),
            KeyCode::End if ctrl => s.scroll = 0,
            KeyCode::Esc if s.scroll > 0 => s.scroll = 0,
            KeyCode::Esc if matches!(s.conn, Conn::Closed(_)) => {
                self.session = None;
                self.load_worlds();
                self.screen = Screen::Picker;
            }
            KeyCode::Enter => s.send_input(),
            KeyCode::Up if !s.server_echo => s.history_step(true),
            KeyCode::Down if !s.server_echo => s.history_step(false),
            _ => {
                s.input.edit(&k);
            }
        }
    }
}

impl Session {
    fn send_input(&mut self) {
        let line = std::mem::take(&mut self.input.text);
        self.input.cursor = 0;
        self.history_pos = None;
        self.scroll = 0;
        let Some(net) = &self.net else {
            self.out.note("not connected: Ctrl+R reconnects");
            return;
        };
        if self.server_echo {
            log::info!("send (hidden, {} chars)", line.chars().count());
            self.out.echo("");
        } else {
            log::info!("send {line:?}");
            self.out.echo(&line);
            if !line.trim().is_empty() && self.history.last() != Some(&line) {
                self.history.push(line.clone());
            }
        }
        let _ = net.send(NetCmd::Line(line));
    }

    fn history_step(&mut self, back: bool) {
        if self.history.is_empty() {
            return;
        }
        let pos = match (self.history_pos, back) {
            (None, true) => Some(self.history.len() - 1),
            (None, false) => None,
            (Some(p), true) => Some(p.saturating_sub(1)),
            (Some(p), false) if p + 1 < self.history.len() => Some(p + 1),
            (Some(_), false) => None,
        };
        self.history_pos = pos;
        match pos {
            Some(p) => self.input.set(&self.history[p].clone()),
            None => self.input.set(""),
        }
    }

    fn on_gmcp(&mut self, pkg: &str, json: &str) {
        let pkg = pkg.to_lowercase();
        if pkg != "char.vitals" && pkg != "char.maxstats" {
            return;
        }
        let Ok(serde_json::Value::Object(m)) = serde_json::from_str(json) else { return };
        for (k, v) in m {
            let v = match v {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                _ => continue,
            };
            self.vitals.insert(k.to_lowercase(), v);
        }
    }

    /// Vitals that come in pairs (`hp` and `maxhp`): label, current, maximum.
    /// Health and mana first, then the rest alphabetically.
    pub fn vital_bars(&self) -> Vec<(String, f64, f64)> {
        let mut out: Vec<(String, f64, f64)> = self
            .vitals
            .iter()
            .filter(|(k, _)| !k.starts_with("max"))
            .filter_map(|(k, v)| {
                let max = self.vitals.get(&format!("max{k}"))?.parse::<f64>().ok()?;
                Some((k.to_uppercase(), v.parse::<f64>().ok()?, max))
            })
            .collect();
        let rank = |k: &str| match k {
            "HP" | "HEALTH" => 0,
            "MP" | "MANA" | "SP" => 1,
            _ => 2,
        };
        out.sort_by(|a, b| rank(&a.0).cmp(&rank(&b.0)).then(a.0.cmp(&b.0)));
        out
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

/// `<state>/lera/<world>/logs/<date>.txt`, appended to.
fn open_log(paths: &Paths, w: &World) -> Option<std::fs::File> {
    let dir = paths.world_dir(&w.id).join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let file = dir.join(format!("{}.txt", today()));
    log::info!("session log {}", file.display());
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file).ok()?;
    let _ = writeln!(f, "\n--- {} {}:{} ---", w.name, w.host, w.endpoint().0);
    Some(f)
}

/// Today's date (UTC) as YYYY-MM-DD, without a date crate.
fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = civil(secs as i64 / 86_400);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_736), (2026, 10, 10));
    }

    #[test]
    fn input_editing() {
        let mut i = Input::default();
        for c in "say hi there".chars() {
            i.insert(c);
        }
        i.delete_word();
        assert_eq!(i.text, "say hi ");
        i.cursor = 0;
        i.insert('>');
        assert_eq!(i.text, ">say hi ");
    }
}
