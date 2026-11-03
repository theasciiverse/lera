# Lera

The Asciiverse MUD client: one terminal program to play any MUD in, comfortably,
on Linux, macOS and Windows, made to fit Asciiverse closely. Rust + Ratatui + crossterm.

Swedish: lera = clay, mud. MUD = Multi-User Dungeon, so Lera = Multi-User Clay.

**Scope (settled 2026-10-10):** a central TUI for playing any and all MUDs, with
every comfort a player could want, and very close integration with Asciiverse.
Nothing else for now. The earlier ideas (Braille room rendering, combat effects,
companion window, phone notifications, an original Lera world) are parked in
`LeraIdeasParked.md`, not dropped for good.

## Why build it

1. **Integration with Asciiverse.** No existing client knows about the
   catalog, av, the shared folders or the house look. Lera is how MUDs become
   first-class Asciiverse titles instead of a `telnet` fallback. This is
   the main reason.
2. **A gap among existing clients** (checked 2026-10-10):

   | Client | Kind | Status | Gap Lera fills |
   |---|---|---|---|
   | Mudlet | desktop GUI, Lua | 5.0.1, 2026-09-02 | not a terminal program |
   | Blightmud | terminal, Rust, GPL-3, Lua | v5.7.1, 2026-05-31, active | termion, so no native Windows; no mapper; for technical users |
   | TinTin++ | terminal, C, own script language | 2.02.61, 2026-01-29 | steep to learn; Windows only via WSL |
   | Durthang | terminal, Rust + Ratatui, GPL-3 | 0.2.0, 2026-07-07, 1 star | very young, one person's project |
   | TinyFugue, MUSHclient | veterans | | dated; Windows-only or Unix-only |

   Nobody has the combination of picking a world from a list and being in the
   game in seconds, with no config language to learn, a polished screen by
   default, and native support on all three OSes.

## License: MIT (decided 2026-10-10)

MIT. Not because it is the norm among MUD clients (it isn't: Mudlet,
Blightmud and Durthang are GPL-3, TinTin++ LGPL-2.1), but because:

1. **Our standards should spread.** Lera will define new GMCP messages (for
   graphics, among others). MIT code can go into GPL clients, not the reverse,
   so Mudlet, Blightmud, MUD servers and tools under any license can take
   Lera's parser or renderer directly.
2. **It matches the stack.** Ratatui, `libmudtelnet`, `mlua` and
   `ratatui-image` are all MIT.
3. **It keeps Asciiverse open.** Parts of Lera that become shared crates
   (telnet and GMCP, the map) can be used by any title, open or closed, paid
   or free.

The cost: anyone may fork Lera closed. For a free client that risk is small;
the original, best-integrated version is the answer to it.

What follows from it:

- Dependencies must be permissive (MIT, BSD, Apache, Zlib and the like).
  Candidates checked 2026-10-10: `libmudtelnet` MIT, `mlua` MIT,
  `ratatui-image` MIT. Add `cargo deny` with a license allow-list once there
  is code, so a copyleft crate can't slip in.
- No code copied from Blightmud, Mudlet or Durthang (GPL-3) or TinTin++
  (LGPL-2.1, as GitHub reports it). Reading them to learn how they handle a
  protocol corner is fine; writing our own code from the specs is the rule.
- Protocol specs (GMCP, MSDP, MSSP, MCCP2, MTTS, MXP, MSP, MCMP) are free to
  implement.

## Integration with Asciiverse

This is what makes Lera Lera. av today (`av/src/main.rs`, `launch`) runs the
system `telnet host port` for any title with `kind = "mud"`; Lera replaces that.

- **av launches Lera.** Connect on a MUD runs `lera <world-id>` (Lera finds the
  host and port in the catalog) as a separate process, the same way av
  launches every title. If Lera isn't installed, av offers to install it, then
  falls back to `telnet`.
- **One world list.** Lera reads the same `catalog.json` av caches
  (`<cache>/catalog.json`), so its own world picker shows the same MUDs, with
  the same descriptions, `since` years, OLD!/NEW! banners and previews.
  Run on its own (`lera`), it opens that picker; worlds the player adds by
  hand (`host:port`) sit beside the catalog ones.
- **MUDs join by PR to `hub`**, like any title. `kind = "mud"` with
  `connect = { host, port }`, plus `since` and `website`, is already read by av
  but isn't in `hub/catalog/SCHEMA.md` yet; it goes in, adding a TLS port.
  Lera can read MSSP from a server to fill in or check an entry.
- **Shared folders.** Lera uses av's buckets (`av/src/paths.rs`): per-world
  data (profiles, the map, logs, aliases and triggers) under
  `<state>/lera/<world-id>/`, settings under `<config>/lera/`, nothing in
  a folder of its own.
- **Shared state with av.** av's wishlist is Lera's favourites. Lera records
  when a world was last played, so av can show it.
- **Same look.** av's palette (`av/src/theme.rs`: dark panels, green, amber),
  the same key conventions and help style. Copied at first, not a shared
  crate, until a second program needs it too (the house "validate before
  generalizing" order).
- **Previews come from Lera.** Lera can record a session as an asciicast v3
  `.cast` at 100x30 (`hub/RECORDING.md`), so every MUD in the catalog
  gets a preview recorded in Lera itself, and players can record their own.
- **David's own MUDs** are ordinary catalog entries. Any extra rendering they
  want comes through standard GMCP, so other MUDs can use the same thing.
- **Session logging** follows the Asciiverse convention: every key, every
  protocol negotiation and state change, and a periodic snapshot, via `log` +
  `env_logger`.

## Version 1 (the current goal)

A plain, solid client to play any MUD in every day. Everything else in this
file is v2 or later. Done when Aardwolf, Achaea and Discworld play
comfortably in Lera on Linux, macOS and Windows.

- `lera` opens a world picker: the MUDs in av's cached catalog plus worlds
  added by hand; `lera <world-id>` connects straight away; so does
  `lera <host> <port> [--tls]`
- TCP and TLS; reconnect with one key
- Telnet through `libmudtelnet`: NAWS, TTYPE/MTTS, ECHO (masked password
  input), EOR/GA prompts, MCCP2, CHARSET (UTF-8); every other option refused
- ANSI colour: 16, 256 and truecolor
- Output pane with scrollback that splits while scrolled back; input line
  with history; a status line (world, connection, and GMCP `Char.Vitals` when
  the MUD sends it)
- A session log per world, plain text
- av's palette, F1 help, session logging per the Asciiverse convention
- av: Connect runs Lera when it is installed, `telnet` otherwise
- hub: `kind = "mud"` written into `hub/catalog/SCHEMA.md`

## The full client (v2 and later, apart from what Version 1 lists)

### Connection and protocols
- TCP and TLS (many MUDs offer a TLS port), reconnect with one key, MCCP2 compression
- Telnet negotiation: NAWS (window size), TTYPE/MTTS (announces 256-colour and
  UTF-8 support), CHARSET (UTF-8), EOR/GA for prompt detection, ECHO for
  hiding password input
- ANSI colour: 16, 256 and truecolor
- GMCP and MSDP for structured data; MSSP for server info
- Telnet layer: `libmudtelnet` (MIT, Blightmud's own parser, 2.0.2 from
  2026-05-26) is the candidate; try it before writing our own

### Screen
- Main output with scrollback; scrolling up splits the view so live output
  stays visible underneath
- An input line with history, history search, word completion from what the
  MUD has sent, and masking when the server asks for a password
- Prompt shown on its own line when the MUD marks it (EOR/GA)
- **Vitals bars** from GMCP `Char.Vitals` (health, mana and the like)
- **Map pane** built from GMCP `Room.Info` as the player walks; saved per
  world; click or name a room to walk there (speedwalk)
- **Channels pane**: tells and chat from GMCP `Comm.Channel` collected apart
  from the main flow
- Links clickable with the mouse, scrollback search, copy
- Panes only appear when the MUD sends the data for them; a MUD with
  only plain telnet gets a clean full-width text screen
- Several worlds open at once, in tabs
- A screen-reader mode: plain linear output, no panes

### Comforts
- Aliases, triggers, highlights, gags and timers, edited in a screen inside
  Lera (not a config language), stored as TOML per world
- Command stacking (`;`) and repeat (`#5 north`)
- Session logs, plain text and HTML
- Saved characters per world; passwords in the OS keyring, if the player
  wants
- F1 help and a first-run tour

## Graphics in MUDs: an open GMCP spec (v2 or later)

No common way exists for a MUD to send pictures to a terminal client (MCMP,
Mudlet's GMCP media standard, covers sound and video; images not confirmed).
Lera fills that with an open spec, published on its own, versioned, MIT, so
any client or server can adopt it. Lera is the first and best implementation,
not a privileged one. Working name for the GMCP namespace: TBD (neutral, not
"Lera" or "Asciiverse", so other clients adopt it without feeling they
advertise us).

Rules:
- **Opt-in by the client.** Lera announces support the standard GMCP way
  (`Core.Supports.Set`); a server only sends graphics to clients that asked.
- **Text stays complete.** Graphics add to the text, never replace it; a
  player in plain telnet or a screen reader misses nothing needed to play.
- **Terminal-native first.** Content kinds, simplest first:
  1. *Cell art*: a grid of glyphs and colours (ANSI art, Braille, half-blocks),
     drawn identically in every terminal
  2. *Images*: by URL with a sha256, cached; drawn with `ratatui-image`
     (Kitty, Sixel or iTerm2 protocol where the terminal has one, half-blocks
     elsewhere)
  3. *Scenes*: a tile grid with sprites the server moves with small updates
     (tactical maps, fights, next-generation MUDs)
  4. *Map hints*: room coordinates and areas, to make Lera's map exact
- **Panes by name.** The server says where content goes (`scene`, `portrait`,
  ...); the player decides whether and where that pane shows.

David's own MUDs are the first servers to use it. Evennia (BSD-3, Python,
very active) is the likely base: it speaks GMCP and MSDP out of the box and
lets a game send its own GMCP packages (`Foo.Bar` from `foo_bar`; Evennia
docs, checked 2026-10-10). A small Evennia contrib that emits the spec would
be the reference server.

## After version 1

In rough order:
1. GMCP panes: vitals bars, then the map, then channels
2. Aliases and triggers, tabs, `.cast` recording, keyring passwords
3. The graphics spec: cell art and images first, proved with a small Evennia
   test world; then David's own MUDs
4. Lua scripting (mlua, aiming at compatibility with common Mudlet scripts),
   MXP and MSP
5. Anything in `LeraIdeasParked.md`
6. Door worlds: BBS door games as MUDs with a daily turn budget, solo or
   shared, played in Lera; see `LeraDoorWorlds.md` (2026-10-20)

## Code

```
src/main.rs        arguments, logging, the event loop
src/net.rs         the connection thread: TCP/TLS (rustls, webpki roots), telnet
                   via libmudtelnet, MCCP2 (flate2), MTTS, CHARSET, NAWS, GMCP
src/scrollback.rs  ANSI parsing into styled lines, scrollback, word wrap
src/app.rs         state and keys: world picker, session, input, history, logs
src/ui.rs          drawing: picker, session (split when scrolled back), F1 help
src/worlds.rs      catalog MUDs plus <config>/lera/worlds.toml
src/paths.rs       av's folders
src/theme.rs       av's palette
```

```bash
cargo run --release                         # world picker
cargo run --release -- discworld            # a world from the list
cargo run --release -- dunemud.net 6788 --tls
RUST_LOG=lera=trace cargo run --release     # log to <cache>/lera.log
cargo test
```

Lessons from the first cut:
- libmudtelnet only passes on a server's subnegotiations (GMCP, MCCP2) when the
  option is also marked enabled on our side; `net.rs` sets that when the server
  says WILL.
- Test passwords, GMCP and the like against a local fake server, never
  against a real account on a real MUD.
- Achaea, Aardwolf and Discworld document no TLS port (searched 2026-10-10);
  Dune MUD's 6788 is a known good TLS test.

## Status

v1 working, 2026-10-10, tested on Aardwolf, Achaea, Discworld, BatMUD and
(TLS) Dune MUD, plus a local fake server for password masking, MTTS, CHARSET
and GMCP vitals. av's Connect opens Lera (av change not committed yet).
Not yet checked: Windows and macOS builds. Local git repo, nothing committed,
not on GitHub yet.

Name confirmed: Lera. Conceived 2026-07-02, scout camp; rescoped 2026-10-10.
