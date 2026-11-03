//! Drawing: the world picker, the session screen, and F1 help.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Conn, Screen, Session};
use crate::theme;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    app.view = (area.width, area.height.saturating_sub(2));
    f.render_widget(Block::default().style(Style::default().bg(theme::BG).fg(theme::TEXT)), area);
    match app.screen {
        Screen::Picker => picker(f, app, area),
        Screen::Session => {
            if let Some(s) = &mut app.session {
                session(f, s, area);
            }
        }
    }
    if app.help {
        help(f, app, area);
    }
}

fn picker(f: &mut Frame, app: &App, area: Rect) {
    let [head, body, foot] = Layout::vertical([Constraint::Length(3), Constraint::Min(3), Constraint::Length(2)]).areas(area);
    let title = Line::from(vec![
        Span::styled(" Lera ", Style::default().fg(theme::BG).bg(theme::GREEN).add_modifier(Modifier::BOLD)),
        Span::styled("  choose a world", Style::default().fg(theme::DIM)),
    ]);
    let filter = if app.picker.filter.is_empty() {
        Line::from(Span::styled(" type to filter", Style::default().fg(theme::FAINT)))
    } else {
        Line::from(vec![Span::styled(" filter: ", Style::default().fg(theme::DIM)), Span::styled(app.picker.filter.clone(), Style::default().fg(theme::AMBER))])
    };
    f.render_widget(Paragraph::new(vec![Line::default(), title, filter]), head);

    let visible = app.visible();
    let count = format!(" {} of {} ", visible.len(), app.worlds.len());
    let block = frame(theme::LINE)
        .title_top(Line::from(Span::styled(" Worlds ", Style::default().fg(theme::TEXT_HI).add_modifier(Modifier::BOLD))).left_aligned())
        .title_top(Line::from(Span::styled(count, Style::default().fg(theme::DIM))).right_aligned())
        .padding(Padding::new(1, 1, 1, 0));
    let body_frame = body;
    let body = block.inner(body_frame);
    f.render_widget(block, body_frame);
    let mut lines: Vec<Line> = Vec::new();
    if app.worlds.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(" No worlds yet.", Style::default().fg(theme::TEXT_HI))));
        lines.push(Line::from(Span::styled(" Run av once to fetch the Asciiverse catalog of MUDs, or add a world with Ctrl+N.", Style::default().fg(theme::DIM))));
    } else if !app.has_catalog {
        lines.push(Line::from(Span::styled(" (no Asciiverse catalog found: run av once to list its MUDs here)", Style::default().fg(theme::FAINT))));
    }
    // two rows per world; keep the selection in view
    let per_page = (body.height as usize / 2).max(1);
    let first = app.picker.selected.saturating_sub(per_page - 1);
    for (row, &i) in visible.iter().enumerate().skip(first).take(per_page) {
        let w = &app.worlds[i];
        let sel = row == app.picker.selected;
        let (port, tls) = w.endpoint();
        let marker = if sel { Span::styled(" > ", Style::default().fg(theme::AMBER)) } else { Span::raw("   ") };
        let name_style = if sel { Style::default().fg(theme::GREEN_HI).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme::TEXT_HI) };
        let mut l1 = vec![marker, Span::styled(w.name.clone(), name_style)];
        if let Some(y) = w.since {
            l1.push(Span::styled(format!("  since {y}"), Style::default().fg(theme::DIM)));
        }
        l1.push(Span::styled(format!("  {}:{}{}", w.host, port, if tls { " TLS" } else { "" }), Style::default().fg(theme::FAINT)));
        if !w.from_catalog {
            l1.push(Span::styled("  yours", Style::default().fg(theme::AMBER_SOFT)));
        }
        lines.push(Line::from(l1));
        lines.push(Line::from(Span::styled(format!("   {}", w.description), Style::default().fg(theme::DIM))));
    }
    if !app.worlds.is_empty() && visible.is_empty() {
        lines.push(Line::from(Span::styled(" No world matches the filter.", Style::default().fg(theme::DIM))));
    }
    f.render_widget(Paragraph::new(lines), body);

    let hint = " Enter connect   Ctrl+N add a world   Ctrl+D remove yours   F1 help   Esc quit";
    let foot_lines = vec![
        Line::from(Span::styled(format!(" {}", app.picker.message), Style::default().fg(theme::AMBER_SOFT))),
        Line::from(Span::styled(hint, Style::default().fg(theme::FAINT))),
    ];
    f.render_widget(Paragraph::new(foot_lines), foot);

    if let Some(form) = &app.picker.form {
        let w = 54.min(area.width);
        let r = Rect { x: area.x + (area.width - w) / 2, y: area.y + area.height.saturating_sub(10) / 2, width: w, height: 10.min(area.height) };
        f.render_widget(Clear, r);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::GREEN))
            .title(" Add a world ")
            .style(Style::default().bg(theme::PANEL));
        let inner = block.inner(r);
        f.render_widget(block, r);
        let field = |i: usize, label: &str, value: String| {
            let st = if form.field == i { Style::default().fg(theme::AMBER) } else { Style::default().fg(theme::TEXT_HI) };
            Line::from(vec![Span::styled(format!(" {label:<6}"), Style::default().fg(theme::DIM)), Span::styled(value, st)])
        };
        let cursor = |i: usize| if form.field == i { "_" } else { "" };
        let lines = vec![
            Line::default(),
            field(0, "Name", format!("{}{}", form.name, cursor(0))),
            field(1, "Host", format!("{}{}", form.host, cursor(1))),
            field(2, "Port", format!("{}{}", form.port, cursor(2))),
            field(3, "TLS", format!("[{}] (Space)", if form.tls { "x" } else { " " })),
            Line::default(),
            Line::from(Span::styled(format!(" {}", form.error), Style::default().fg(theme::AMBER))),
            Line::from(Span::styled(" Tab next   Enter save   Esc cancel", Style::default().fg(theme::FAINT))),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn session(f: &mut Frame, s: &mut Session, area: Rect) {
    // frames need room; a tiny terminal gets the bare text
    if area.height < 10 || area.width < 40 {
        return session_bare(f, s, area);
    }
    let [out, input, status] = Layout::vertical([Constraint::Min(3), Constraint::Length(3), Constraint::Length(1)]).areas(area);

    let mut block = frame(theme::LINE)
        .title_top(Line::from(vec![Span::raw(" "), Span::styled(s.world.name.clone(), Style::default().fg(theme::TEXT_HI).add_modifier(Modifier::BOLD)), Span::raw(" ")]).left_aligned())
        .title_top(Line::from(conn_spans(s)).right_aligned())
        .padding(Padding::horizontal(1));
    if s.scroll > 0 {
        block = block.title_bottom(
            Line::from(Span::styled(format!(" scrolled back {} rows · PageDown or Esc returns ", s.scroll), Style::default().fg(theme::AMBER_SOFT))).right_aligned(),
        );
    }
    let inner = block.inner(out);
    f.render_widget(block, out);
    s.view = (inner.width, inner.height);
    output(f, s, inner);

    input_box(f, s, input);
    status_line(f, s, status);
}

fn frame(color: ratatui::style::Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(theme::BG))
}

/// Connection state for the output frame's title.
fn conn_spans(s: &Session) -> Vec<Span<'static>> {
    let (state, color) = match &s.conn {
        Conn::Connecting => ("connecting", theme::AMBER),
        Conn::Connected { .. } => ("connected", theme::GREEN_HI),
        Conn::Closed(_) => ("disconnected", theme::RED),
    };
    let mut v = vec![Span::raw(" "), Span::styled(state, Style::default().fg(color))];
    if matches!(s.conn, Conn::Connected { tls: true }) {
        v.push(Span::styled(" · TLS", Style::default().fg(theme::DIM)));
    }
    if s.compressed {
        v.push(Span::styled(" · MCCP", Style::default().fg(theme::DIM)));
    }
    v.push(Span::raw(" "));
    v
}

/// The MUD's text: the live end, or history on top with the live end below.
fn output(f: &mut Frame, s: &mut Session, area: Rect) {
    if s.scroll == 0 || area.height < 6 {
        let (rows, _) = s.out.rows(area.width, area.height as usize, 0);
        let at = bottom_aligned(area, rows.len());
        f.render_widget(Paragraph::new(rows), at);
        return;
    }
    let live_h = (area.height / 3).max(3);
    let [top, sep, live] = Layout::vertical([Constraint::Min(1), Constraint::Length(1), Constraint::Length(live_h)]).areas(area);
    let (_, total) = s.out.rows(area.width, top.height as usize, s.scroll);
    s.scroll = s.scroll.min(total.saturating_sub(top.height as usize)).max(1);
    let (rows, _) = s.out.rows(area.width, top.height as usize, s.scroll);
    f.render_widget(Paragraph::new(rows), top);
    f.render_widget(Paragraph::new(Span::styled("┄".repeat(sep.width as usize), Style::default().fg(theme::LINE))), sep);
    let (rows, _) = s.out.rows(area.width, live.height as usize, 0);
    f.render_widget(Paragraph::new(rows), live);
}

fn input_box(f: &mut Frame, s: &Session, area: Rect) {
    let (color, title) = match (&s.conn, s.server_echo) {
        (Conn::Closed(_), _) => (theme::FAINT, " not connected · Ctrl+R reconnects ".to_string()),
        (_, true) => (theme::AMBER, " password, hidden ".to_string()),
        _ => (theme::GREEN, " > ".to_string()),
    };
    let block = frame(color).title_top(Span::styled(title, Style::default().fg(color))).padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);
    input_line(f, s, inner, None);
}

/// The input text, scrolled so the cursor stays in view, with an optional prompt.
fn input_line(f: &mut Frame, s: &Session, area: Rect, prompt: Option<Span<'static>>) {
    let shown: String = if s.server_echo { "•".repeat(s.input.text.chars().count()) } else { s.input.text.clone() };
    let lead = prompt.as_ref().map(|p| p.content.width()).unwrap_or(0);
    let before: String = shown.chars().take(s.input.cursor).collect();
    let room = (area.width as usize).saturating_sub(lead + 1);
    let skip = before.width().saturating_sub(room);
    let mut w = 0;
    let visible: String = shown
        .chars()
        .skip_while(|c| {
            if w < skip {
                w += unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0);
                true
            } else {
                false
            }
        })
        .collect();
    let mut spans: Vec<Span> = prompt.into_iter().collect();
    spans.push(Span::styled(visible, Style::default().fg(theme::TEXT_HI)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
    let cx = area.x + (lead + before.width() - skip) as u16;
    f.set_cursor_position(Position { x: cx.min(area.right().saturating_sub(1)), y: area.y });
}

/// Rows fill from the bottom up, as in a terminal, until the screen is full.
fn bottom_aligned(area: Rect, rows: usize) -> Rect {
    let h = (rows as u16).min(area.height);
    Rect { y: area.y + area.height - h, height: h, ..area }
}

/// Vitals as gauges on the left, help and the Lera badge on the right.
fn status_line(f: &mut Frame, s: &Session, area: Rect) {
    let mut left: Vec<Span> = vec![Span::raw(" ")];
    let bars = s.vital_bars();
    for (name, cur, max) in &bars {
        let ratio = if *max > 0.0 { (cur / max).clamp(0.0, 1.0) } else { 0.0 };
        let c = if ratio > 0.6 {
            theme::GREEN_HI
        } else if ratio > 0.3 {
            theme::AMBER
        } else {
            theme::RED
        };
        let full = (ratio * 10.0).round() as usize;
        left.push(Span::styled(format!("{name} "), Style::default().fg(theme::DIM)));
        left.push(Span::styled("█".repeat(full), Style::default().fg(c)));
        left.push(Span::styled("░".repeat(10 - full), Style::default().fg(theme::LINE)));
        left.push(Span::styled(format!(" {cur}/{max}   "), Style::default().fg(c)));
    }
    if bars.is_empty() {
        left.push(Span::styled("PageUp scroll back   Ctrl+R reconnect   Ctrl+Q quit", Style::default().fg(theme::FAINT)));
    }
    let right = vec![
        Span::styled("F1 help ", Style::default().fg(theme::FAINT)),
        Span::styled(" Lera ", Style::default().fg(theme::BG).bg(theme::GREEN).add_modifier(Modifier::BOLD)),
    ];
    let lw: usize = left.iter().map(|s| s.content.width()).sum();
    let rw: usize = right.iter().map(|s| s.content.width()).sum();
    left.push(Span::raw(" ".repeat((area.width as usize).saturating_sub(lw + rw))));
    left.extend(right);
    f.render_widget(Paragraph::new(Line::from(left)), area);
}

/// No room for frames: text, one status row, the input row.
fn session_bare(f: &mut Frame, s: &mut Session, area: Rect) {
    let [out, status, input] = Layout::vertical([Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)]).areas(area);
    s.view = (out.width, out.height);
    output(f, s, out);
    let mut l = vec![Span::styled(format!(" {}", s.world.name), Style::default().fg(theme::TEXT_HI))];
    l.extend(conn_spans(s));
    f.render_widget(Paragraph::new(Line::from(l)).style(Style::default().bg(theme::PANEL_HI)), status);
    input_line(f, s, input, Some(Span::styled("> ", Style::default().fg(theme::GREEN))));
}

fn help(f: &mut Frame, app: &App, area: Rect) {
    let keys: &[(&str, &str)] = match app.screen {
        Screen::Picker => &[
            ("Up, Down, wheel", "choose a world"),
            ("type", "filter the list"),
            ("Enter", "connect"),
            ("Ctrl+N", "add a world of your own"),
            ("Ctrl+D", "remove one of your own"),
            ("Esc", "clear the filter, or quit"),
        ],
        Screen::Session => &[
            ("Enter", "send the line (an empty line too)"),
            ("Up, Down", "earlier commands"),
            ("PageUp, PageDown", "scroll back; the live end stays below"),
            ("mouse wheel", "scroll back too"),
            ("Shift+drag", "select text to copy"),
            ("Esc, Ctrl+End", "back to the live end"),
            ("Ctrl+A, Ctrl+E", "start, end of the line"),
            ("Ctrl+W, Ctrl+U", "delete a word, the whole line"),
            ("Ctrl+R", "reconnect"),
            ("Ctrl+D", "disconnect"),
            ("Esc (disconnected)", "back to the worlds"),
            ("Ctrl+Q", "quit"),
        ],
    };
    let w = 60.min(area.width);
    let h = (keys.len() as u16 + 6).min(area.height);
    let r = Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h };
    f.render_widget(Clear, r);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::GREEN))
        .title(" Lera help ")
        .style(Style::default().bg(theme::PANEL));
    let mut lines = vec![Line::default()];
    for (k, what) in keys {
        lines.push(Line::from(vec![Span::styled(format!(" {k:<20}"), Style::default().fg(theme::AMBER)), Span::styled(*what, Style::default().fg(theme::TEXT))]));
    }
    lines.push(Line::default());
    let logs = app.paths.world_dir("<world>").join("logs");
    lines.push(Line::from(Span::styled(format!(" Logs: {}", logs.display()), Style::default().fg(theme::FAINT))));
    lines.push(Line::from(Span::styled(" Any key closes this", Style::default().fg(theme::FAINT))));
    f.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: false }), r);
}
