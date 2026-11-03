//! The connection, on its own thread: TCP or TLS, telnet negotiation through
//! `libmudtelnet`, MCCP2 decompression, and text decoding. The UI talks to it
//! over two channels.
//!
//! Options Lera takes part in: NAWS, TTYPE (with MTTS), CHARSET (UTF-8 only)
//! on our side; ECHO, SGA, EOR, GMCP and MCCP2 on the server's. Everything
//! else is refused.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

use flate2::{Decompress, FlushDecompress, Status};
use libmudtelnet::Parser;
use libmudtelnet::bytes::Bytes;
use libmudtelnet::compatibility::CompatibilityTable;
use libmudtelnet::events::TelnetEvents;
use libmudtelnet::telnet::op_command::{DO, DONT, EOR, GA, WILL, WONT};
use libmudtelnet::telnet::op_option::{ECHO, EOR as OPT_EOR, GMCP, MCCP2, NAWS, SGA, TTYPE};

/// RFC 2066; libmudtelnet has no constant for it
const CHARSET: u8 = 42;
const CHARSET_REQUEST: u8 = 1;
const CHARSET_ACCEPTED: u8 = 2;
const CHARSET_REJECTED: u8 = 3;
const TTYPE_IS: u8 = 0;

/// MTTS bits (tintin.mudhalla.net/protocols/mtts): ANSI, VT100, UTF-8,
/// 256 colours and truecolor; SSL is added on a TLS connection.
const MTTS: u32 = 1 | 2 | 4 | 8 | 256;
const MTTS_SSL: u32 = 2048;

pub enum NetEvent {
    Connected { tls: bool },
    Text(String),
    /// IAC GA or IAC EOR: the text so far ends in a prompt
    Prompt,
    /// the server echoes (it is asking for a password) or stops echoing
    ServerEcho(bool),
    Gmcp(String, String),
    Compressed(bool),
    Closed(String),
}

pub enum NetCmd {
    Line(String),
    Naws(u16, u16),
}

/// Start a connection thread. Dropping the returned sender closes the connection.
pub fn spawn(host: String, port: u16, tls: bool, size: (u16, u16), events: Sender<NetEvent>) -> Sender<NetCmd> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let reason = match run(&host, port, tls, size, &events, rx) {
            Ok(()) => "connection closed".to_string(),
            Err(e) => e.to_string(),
        };
        log::info!("net {host}:{port} ended: {reason}");
        let _ = events.send(NetEvent::Closed(reason));
    });
    tx
}

enum Conn {
    Plain(TcpStream, Vec<u8>),
    Tls(Box<rustls::ClientConnection>, TcpStream),
}

impl Conn {
    /// Ok(None) when nothing has arrived, Ok(Some(0)) when the server closed.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>> {
        match self {
            Conn::Plain(s, _) => match s.read(buf) {
                Ok(n) => Ok(Some(n)),
                Err(e) if would_block(&e) => Ok(None),
                Err(e) => Err(e),
            },
            Conn::Tls(c, s) => {
                match c.read_tls(s) {
                    Ok(0) => return Ok(Some(0)),
                    Ok(_) => {
                        if let Err(e) = c.process_new_packets() {
                            let _ = c.write_tls(s);
                            return Err(io::Error::other(format!("TLS: {e}")));
                        }
                    }
                    Err(e) if would_block(&e) => {}
                    Err(e) => return Err(e),
                }
                match c.reader().read(buf) {
                    Ok(n) => Ok(Some(n)),
                    Err(e) if would_block(&e) => Ok(None),
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        match self {
            Conn::Plain(_, out) => out.extend_from_slice(data),
            Conn::Tls(c, _) => c.writer().write_all(data)?,
        }
        self.flush()
    }

    /// Send whatever the socket will take now; the rest waits for the next turn.
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Conn::Plain(s, out) => {
                while !out.is_empty() {
                    match s.write(out) {
                        Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                        Ok(n) => drop(out.drain(..n)),
                        Err(e) if would_block(&e) => break,
                        Err(e) => return Err(e),
                    }
                }
            }
            Conn::Tls(c, s) => {
                while c.wants_write() {
                    match c.write_tls(s) {
                        Ok(_) => {}
                        Err(e) if would_block(&e) => break,
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        Ok(())
    }
}

fn would_block(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted)
}

fn connect(host: &str, port: u16, tls: bool) -> anyhow::Result<Conn> {
    let addrs: Vec<_> = (host, port).to_socket_addrs()?.collect();
    let mut last = None;
    let mut stream = None;
    for a in &addrs {
        match TcpStream::connect_timeout(a, Duration::from_secs(15)) {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(e) => last = Some(e),
        }
    }
    let Some(s) = stream else {
        return Err(match last {
            Some(e) => anyhow::anyhow!("cannot reach {host}:{port}: {e}"),
            None => anyhow::anyhow!("{host} has no address"),
        });
    };
    s.set_nodelay(true)?;
    s.set_nonblocking(true)?;
    if !tls {
        return Ok(Conn::Plain(s, Vec::new()));
    }
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from(host.to_string())?;
    let c = rustls::ClientConnection::new(Arc::new(config), name)?;
    Ok(Conn::Tls(Box::new(c), s))
}

struct Session<'a> {
    conn: Conn,
    parser: Parser,
    inflate: Option<Decompress>,
    pending: Vec<u8>,
    ttype_step: u8,
    tls: bool,
    size: (u16, u16),
    naws: bool,
    events: &'a Sender<NetEvent>,
}

fn run(host: &str, port: u16, tls: bool, size: (u16, u16), events: &Sender<NetEvent>, rx: Receiver<NetCmd>) -> anyhow::Result<()> {
    log::info!("net connecting {host}:{port} tls={tls}");
    let conn = connect(host, port, tls)?;
    let mut table = CompatibilityTable::default();
    for opt in [NAWS, TTYPE, CHARSET] {
        table.support_local(opt);
    }
    for opt in [ECHO, SGA, OPT_EOR] {
        table.support_remote(opt);
    }
    // the server enables these, but their subnegotiations only come through
    // when marked local too (libmudtelnet's rule); see on_negotiation
    for opt in [GMCP, MCCP2] {
        table.support(opt);
    }
    let mut s = Session {
        conn,
        parser: Parser::with_support(table),
        inflate: None,
        pending: Vec::new(),
        ttype_step: 0,
        tls,
        size,
        naws: false,
        events,
    };
    let _ = events.send(NetEvent::Connected { tls });
    log::info!("net connected");

    let mut buf = vec![0u8; 32 * 1024];
    loop {
        let mut busy = false;
        loop {
            match rx.try_recv() {
                Ok(NetCmd::Line(line)) => {
                    let out = s.parser.send_text(&line);
                    s.send(out)?;
                    busy = true;
                }
                Ok(NetCmd::Naws(w, h)) => {
                    s.size = (w, h);
                    if s.naws {
                        s.send_naws()?;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
        match s.conn.read(&mut buf)? {
            Some(0) => return Ok(()),
            Some(n) => {
                s.ingest(&buf[..n])?;
                busy = true;
            }
            None => {}
        }
        s.conn.flush()?;
        if !busy {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Session<'_> {
    fn send(&mut self, ev: TelnetEvents) -> io::Result<()> {
        self.conn.write(&ev.to_bytes())
    }

    /// Raw bytes from the socket: through MCCP2 if it is on, then the telnet parser.
    fn ingest(&mut self, raw: &[u8]) -> anyhow::Result<()> {
        let mut raw = raw.to_vec();
        loop {
            if let Some(z) = &mut self.inflate {
                let (plain, rest) = inflate(z, &raw)?;
                if rest.is_some() {
                    log::info!("net MCCP2 compression ended");
                    self.inflate = None;
                    let _ = self.events.send(NetEvent::Compressed(false));
                }
                self.feed(&plain)?;
                match rest {
                    Some(r) if !r.is_empty() => raw = r,
                    _ => return Ok(()),
                }
            } else {
                match self.feed(&raw)? {
                    Some(rest) => {
                        log::info!("net MCCP2 compression started");
                        self.inflate = Some(Decompress::new(true));
                        let _ = self.events.send(NetEvent::Compressed(true));
                        raw = rest.to_vec();
                    }
                    None => return Ok(()),
                }
            }
        }
    }

    /// Plain telnet bytes into the parser; returns the rest of the buffer if
    /// the server just turned compression on.
    fn feed(&mut self, data: &[u8]) -> anyhow::Result<Option<Bytes>> {
        for ev in self.parser.receive(data) {
            match ev {
                TelnetEvents::DataReceive(b) => {
                    let text = decode(&mut self.pending, &b);
                    if !text.is_empty() {
                        let _ = self.events.send(NetEvent::Text(text));
                    }
                }
                TelnetEvents::DataSend(b) => self.conn.write(&b)?,
                TelnetEvents::IAC(iac) if iac.command == GA || iac.command == EOR => {
                    let _ = self.events.send(NetEvent::Prompt);
                }
                TelnetEvents::IAC(_) => {}
                TelnetEvents::Negotiation(n) => self.on_negotiation(n.command, n.option)?,
                TelnetEvents::Subnegotiation(sub) => self.on_subnegotiation(sub.option, &sub.buffer)?,
                TelnetEvents::DecompressImmediate(rest) => return Ok(Some(rest)),
            }
        }
        Ok(None)
    }

    fn on_negotiation(&mut self, cmd: u8, opt: u8) -> anyhow::Result<()> {
        log::debug!("net telnet {} {}", cmd_name(cmd), opt);
        match (cmd, opt) {
            (WILL, ECHO) => drop(self.events.send(NetEvent::ServerEcho(true))),
            (WONT, ECHO) => drop(self.events.send(NetEvent::ServerEcho(false))),
            (WILL, GMCP | MCCP2) => {
                let mut e = self.parser.options.get_option(opt);
                e.local_state = true;
                self.parser.options.set_option(opt, e);
                if opt == GMCP {
                    let hello = format!(r#"Core.Hello {{"client":"Lera","version":"{}"}}"#, env!("CARGO_PKG_VERSION"));
                    self.gmcp(&hello)?;
                    self.gmcp(r#"Core.Supports.Set ["Char 1", "Char.Vitals 1"]"#)?;
                }
            }
            (DO, NAWS) => {
                self.naws = true;
                self.send_naws()?;
            }
            (DONT, NAWS) => self.naws = false,
            (DONT, TTYPE) => self.ttype_step = 0,
            _ => {}
        }
        Ok(())
    }

    fn on_subnegotiation(&mut self, opt: u8, data: &[u8]) -> anyhow::Result<()> {
        match opt {
            GMCP => {
                let text = String::from_utf8_lossy(data);
                let (pkg, json) = text.split_once(' ').unwrap_or((&text, ""));
                log::trace!("net gmcp {pkg} {json}");
                let _ = self.events.send(NetEvent::Gmcp(pkg.to_string(), json.to_string()));
            }
            TTYPE => {
                // MTTS: client name, then terminal type, then the MTTS bits, repeated
                let name = match self.ttype_step {
                    0 => "LERA".to_string(),
                    1 => "XTERM-256COLOR".to_string(),
                    _ => format!("MTTS {}", MTTS | if self.tls { MTTS_SSL } else { 0 }),
                };
                self.ttype_step = self.ttype_step.saturating_add(1);
                log::debug!("net ttype is {name}");
                let mut body = vec![TTYPE_IS];
                body.extend_from_slice(name.as_bytes());
                if let Some(ev) = self.parser.subnegotiation(TTYPE, body) {
                    self.send(ev)?;
                }
            }
            CHARSET if data.first() == Some(&CHARSET_REQUEST) && data.len() > 2 => {
                let sep = data[1] as char;
                let offered = String::from_utf8_lossy(&data[2..]).to_string();
                let utf8 = offered.split(sep).find(|c| c.eq_ignore_ascii_case("utf-8"));
                log::debug!("net charset offered {offered:?}");
                let body = match utf8 {
                    Some(c) => [&[CHARSET_ACCEPTED][..], c.as_bytes()].concat(),
                    None => vec![CHARSET_REJECTED],
                };
                if let Some(ev) = self.parser.subnegotiation(CHARSET, body) {
                    self.send(ev)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn gmcp(&mut self, msg: &str) -> io::Result<()> {
        match self.parser.subnegotiation(GMCP, msg.as_bytes().to_vec()) {
            Some(ev) => self.send(ev),
            None => Ok(()),
        }
    }

    fn send_naws(&mut self) -> io::Result<()> {
        let (w, h) = self.size;
        log::debug!("net naws {w}x{h}");
        let body = [(w >> 8) as u8, w as u8, (h >> 8) as u8, h as u8].to_vec();
        match self.parser.subnegotiation(NAWS, body) {
            Some(ev) => self.send(ev),
            None => Ok(()),
        }
    }
}

fn cmd_name(c: u8) -> &'static str {
    match c {
        WILL => "WILL",
        WONT => "WONT",
        DO => "DO",
        DONT => "DONT",
        _ => "?",
    }
}

/// Decompress what has arrived. The second value is Some(rest) when the
/// compressed stream ended, with whatever plain bytes came after it.
fn inflate(z: &mut Decompress, mut input: &[u8]) -> anyhow::Result<(Vec<u8>, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut chunk = vec![0u8; 32 * 1024];
    loop {
        let (in0, out0) = (z.total_in(), z.total_out());
        let status = z.decompress(input, &mut chunk, FlushDecompress::None)?;
        let used = (z.total_in() - in0) as usize;
        let made = (z.total_out() - out0) as usize;
        out.extend_from_slice(&chunk[..made]);
        input = &input[used..];
        if status == Status::StreamEnd {
            return Ok((out, Some(input.to_vec())));
        }
        if (input.is_empty() && made < chunk.len()) || (used == 0 && made == 0) {
            return Ok((out, None));
        }
    }
}

/// Bytes to text: UTF-8 where it is valid, Latin-1 for any byte that isn't
/// (older MUDs send Latin-1), and an unfinished UTF-8 sequence at the end kept
/// for the next read.
pub fn decode(pending: &mut Vec<u8>, data: &[u8]) -> String {
    pending.extend_from_slice(data);
    let buf = std::mem::take(pending);
    let mut out = String::with_capacity(buf.len());
    let mut i = 0;
    loop {
        match std::str::from_utf8(&buf[i..]) {
            Ok(s) => {
                out.push_str(s);
                break;
            }
            Err(e) => {
                let valid = e.valid_up_to();
                out.push_str(std::str::from_utf8(&buf[i..i + valid]).unwrap_or_default());
                i += valid;
                match e.error_len() {
                    Some(n) => {
                        out.extend(buf[i..i + n].iter().map(|&b| b as char));
                        i += n;
                    }
                    None => {
                        pending.extend_from_slice(&buf[i..]);
                        break;
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    #[test]
    fn decode_split_utf8_and_latin1() {
        let mut p = Vec::new();
        let a = "å".as_bytes();
        assert_eq!(decode(&mut p, &a[..1]), "");
        assert_eq!(decode(&mut p, &a[1..]), "å");
        assert_eq!(decode(&mut p, b"caf\xe9!"), "café!");
    }

    #[test]
    fn inflate_stops_at_stream_end() {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(b"hello world").unwrap();
        let mut data = e.finish().unwrap();
        data.extend_from_slice(b"plain");
        let mut z = Decompress::new(true);
        let (half, rest) = inflate(&mut z, &data[..5]).unwrap();
        assert!(rest.is_none());
        let (more, rest) = inflate(&mut z, &data[5..]).unwrap();
        assert_eq!([half, more].concat(), b"hello world");
        assert_eq!(rest.unwrap(), b"plain");
    }
}
