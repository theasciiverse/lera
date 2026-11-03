//! What the MUD has said: ANSI text parsed into styled lines, kept in a
//! bounded scrollback, and wrapped to the screen from the bottom up.

use std::collections::VecDeque;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use crate::theme;

const MAX_LINES: usize = 20_000;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Fg {
    #[default]
    Default,
    /// one of the 8 basic colours, which bold brightens, as MUDs expect
    Basic(u8),
    Other(Color),
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Pen {
    fg: Fg,
    bg: Option<Color>,
    mods: Modifier,
}

impl Pen {
    fn style(&self) -> Style {
        let bold = self.mods.contains(Modifier::BOLD);
        let fg = match self.fg {
            Fg::Default => theme::TEXT,
            Fg::Basic(n) => basic(if bold { n + 8 } else { n }),
            Fg::Other(c) => c,
        };
        let mut s = Style::default().fg(fg).add_modifier(self.mods);
        if let Some(bg) = self.bg {
            s = s.bg(bg);
        }
        s
    }
}

fn basic(n: u8) -> Color {
    match n {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        _ => Color::White,
    }
}

/// Where the parser is in an escape sequence.
#[allow(clippy::enum_variant_names)]
enum Esc {
    None,
    Esc,
    Csi(String),
    Osc,
    OscEsc,
}

pub struct Scrollback {
    lines: VecDeque<Line<'static>>,
    /// the line still being written: usually a prompt, or text mid-packet
    partial: Vec<Span<'static>>,
    run: String,
    pen: Pen,
    esc: Esc,
    /// finished lines as plain text, for the session log
    pub finished: Vec<String>,
}

impl Default for Scrollback {
    fn default() -> Self {
        Scrollback { lines: VecDeque::new(), partial: vec![], run: String::new(), pen: Pen::default(), esc: Esc::None, finished: vec![] }
    }
}

impl Scrollback {
    pub fn feed(&mut self, text: &str) {
        for c in text.chars() {
            match std::mem::replace(&mut self.esc, Esc::None) {
                Esc::None => self.plain(c),
                Esc::Esc => {
                    self.esc = match c {
                        '[' => Esc::Csi(String::new()),
                        ']' => Esc::Osc,
                        _ => Esc::None,
                    }
                }
                Esc::Csi(mut p) => {
                    if ('\x40'..='\x7e').contains(&c) {
                        if c == 'm' {
                            self.sgr(&p);
                        }
                    } else if p.len() < 64 {
                        p.push(c);
                        self.esc = Esc::Csi(p);
                    }
                }
                Esc::Osc => {
                    self.esc = match c {
                        '\x07' => Esc::None,
                        '\x1b' => Esc::OscEsc,
                        _ => Esc::Osc,
                    }
                }
                Esc::OscEsc => self.esc = if c == '\\' { Esc::None } else { Esc::Osc },
            }
        }
        self.flush_run();
    }

    fn plain(&mut self, c: char) {
        match c {
            '\x1b' => self.esc = Esc::Esc,
            '\n' => self.end_line(),
            '\t' => {
                let col = self.partial_width() + self.run.chars().count();
                self.run.extend(std::iter::repeat_n(' ', 8 - col % 8));
            }
            '\x08' => {
                if self.run.pop().is_none()
                    && let Some(s) = self.partial.last_mut()
                {
                    let mut t = s.content.to_string();
                    t.pop();
                    s.content = t.into();
                }
            }
            c if (c as u32) < 0x20 || c == '\x7f' => {}
            c => self.run.push(c),
        }
    }

    fn partial_width(&self) -> usize {
        self.partial.iter().map(|s| s.content.chars().count()).sum()
    }

    fn flush_run(&mut self) {
        if !self.run.is_empty() {
            let text = std::mem::take(&mut self.run);
            self.partial.push(Span::styled(text, self.pen.style()));
        }
    }

    fn sgr(&mut self, params: &str) {
        self.flush_run();
        let nums: Vec<u32> = params.split([';', ':']).map(|p| p.trim_start_matches('?').parse().unwrap_or(0)).collect();
        let mut i = 0;
        let p = &mut self.pen;
        while i < nums.len() {
            let n = nums[i];
            match n {
                0 => *p = Pen::default(),
                1 => p.mods.insert(Modifier::BOLD),
                2 => p.mods.insert(Modifier::DIM),
                3 => p.mods.insert(Modifier::ITALIC),
                4 => p.mods.insert(Modifier::UNDERLINED),
                5 | 6 => p.mods.insert(Modifier::SLOW_BLINK),
                7 => p.mods.insert(Modifier::REVERSED),
                8 => p.mods.insert(Modifier::HIDDEN),
                9 => p.mods.insert(Modifier::CROSSED_OUT),
                21 | 22 => p.mods.remove(Modifier::BOLD | Modifier::DIM),
                23 => p.mods.remove(Modifier::ITALIC),
                24 => p.mods.remove(Modifier::UNDERLINED),
                25 => p.mods.remove(Modifier::SLOW_BLINK),
                27 => p.mods.remove(Modifier::REVERSED),
                28 => p.mods.remove(Modifier::HIDDEN),
                29 => p.mods.remove(Modifier::CROSSED_OUT),
                30..=37 => p.fg = Fg::Basic((n - 30) as u8),
                39 => p.fg = Fg::Default,
                40..=47 => p.bg = Some(basic((n - 40) as u8)),
                49 => p.bg = None,
                90..=97 => p.fg = Fg::Other(basic((n - 90 + 8) as u8)),
                100..=107 => p.bg = Some(basic((n - 100 + 8) as u8)),
                38 | 48 => {
                    let color = match nums.get(i + 1) {
                        Some(5) => {
                            let c = nums.get(i + 2).map(|&c| Color::Indexed(c as u8));
                            i += 2;
                            c
                        }
                        Some(2) => {
                            let g = |k| nums.get(i + k).copied().unwrap_or(0) as u8;
                            let c = Color::Rgb(g(2), g(3), g(4));
                            i += 4;
                            Some(c)
                        }
                        _ => None,
                    };
                    if let Some(c) = color {
                        if n == 38 {
                            p.fg = Fg::Other(c);
                        } else {
                            p.bg = Some(c);
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn end_line(&mut self) {
        self.flush_run();
        let spans = std::mem::take(&mut self.partial);
        self.finished.push(spans.iter().map(|s| s.content.as_ref()).collect());
        self.lines.push_back(Line::from(spans));
        if self.lines.len() > MAX_LINES {
            self.lines.pop_front();
        }
    }

    /// The player's command, shown after the prompt it answers.
    pub fn echo(&mut self, cmd: &str) {
        self.flush_run();
        self.partial.push(Span::styled(cmd.to_string(), Style::default().fg(theme::AMBER_SOFT)));
        self.end_line();
    }

    /// A note from Lera itself, on a line of its own.
    pub fn note(&mut self, text: &str) {
        self.flush_run();
        if !self.partial.is_empty() {
            self.end_line();
        }
        self.partial.push(Span::styled(format!("[lera] {text}"), Style::default().fg(theme::GREEN)));
        self.end_line();
    }

    /// The last `skip + height` screen rows, wrapped to `width`, oldest first,
    /// minus the newest `skip`. Also returns how many rows exist in total, at
    /// least as far back as was needed.
    pub fn rows(&self, width: u16, height: usize, skip: usize) -> (Vec<Line<'static>>, usize) {
        let width = width.max(1) as usize;
        let want = skip + height;
        let mut rows: Vec<Line<'static>> = Vec::new();
        let partial = (!self.partial.is_empty() || !self.run.is_empty()).then(|| Line::from(self.partial.clone()));
        let all = self.lines.iter().rev().cloned();
        for line in partial.into_iter().chain(all) {
            let mut wrapped = wrap(&line, width);
            wrapped.reverse();
            rows.extend(wrapped);
            if rows.len() >= want {
                break;
            }
        }
        let total = rows.len();
        let skip = skip.min(total);
        let mut view: Vec<_> = rows.into_iter().skip(skip).take(height).collect();
        view.reverse();
        (view, total)
    }
}

/// Word wrap a styled line into screen rows.
fn wrap(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    let cells: Vec<(char, Style)> = line.spans.iter().flat_map(|s| s.content.chars().map(move |c| (c, s.style))).collect();
    if cells.is_empty() {
        return vec![Line::default()];
    }
    let mut rows = Vec::new();
    let mut start = 0;
    while start < cells.len() {
        let mut w = 0;
        let mut end = start;
        let mut last_space = None;
        while end < cells.len() {
            let cw = cells[end].0.width().unwrap_or(0);
            if w + cw > width && end > start {
                break;
            }
            w += cw;
            if cells[end].0 == ' ' {
                last_space = Some(end);
            }
            end += 1;
        }
        if end < cells.len()
            && let Some(sp) = last_space.filter(|&sp| sp > start)
        {
            end = sp + 1;
        }
        rows.push(styled(&cells[start..end]));
        start = end;
    }
    rows
}

fn styled(cells: &[(char, Style)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut text = String::new();
    let mut style = cells.first().map(|c| c.1).unwrap_or_default();
    for &(c, s) in cells {
        if s != style {
            spans.push(Span::styled(std::mem::take(&mut text), style));
            style = s;
        }
        text.push(c);
    }
    spans.push(Span::styled(text, style));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(l: &Line) -> String {
        l.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn colours_and_lines() {
        let mut sb = Scrollback::default();
        sb.feed("\x1b[1;31mred\x1b[0m plain\r\nnext");
        assert_eq!(sb.finished, vec!["red plain"]);
        assert_eq!(sb.lines[0].spans[0].style.fg, Some(Color::LightRed));
        let (rows, _) = sb.rows(80, 10, 0);
        assert_eq!(rows.iter().map(text).collect::<Vec<_>>(), vec!["red plain", "next"]);
    }

    #[test]
    fn escapes_split_across_reads() {
        let mut sb = Scrollback::default();
        sb.feed("a\x1b[3");
        sb.feed("2mb\n");
        assert_eq!(sb.finished, vec!["ab"]);
        assert_eq!(sb.lines[0].spans[1].style.fg, Some(Color::Green));
    }

    #[test]
    fn truecolor_and_osc() {
        let mut sb = Scrollback::default();
        sb.feed("\x1b]0;title\x07\x1b[38;2;1;2;3mx\n");
        assert_eq!(sb.finished, vec!["x"]);
        assert_eq!(sb.lines[0].spans[0].style.fg, Some(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn word_wrap_and_scroll() {
        let mut sb = Scrollback::default();
        sb.feed("one two three\n");
        let (rows, total) = sb.rows(8, 10, 0);
        assert_eq!(rows.iter().map(text).collect::<Vec<_>>(), vec!["one two ", "three"]);
        assert_eq!(total, 2);
        let (rows, _) = sb.rows(8, 1, 1);
        assert_eq!(text(&rows[0]), "one two ");
    }
}
