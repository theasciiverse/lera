# Lera

The Asciiverse MUD client: pick a world from the list and be in the game in
seconds, in your terminal, on Linux, macOS and Windows.

Swedish: lera = clay, mud. A MUD is a Multi-User Dungeon, so Lera is
Multi-User Clay.

## Playing

- `lera` opens the world picker: every MUD in the Asciiverse catalog, plus
  worlds of your own (Ctrl+N adds one)
- `lera <world-id>` connects to a world from the list; av's Connect runs this
- `lera <host> <port> [--tls]` connects anywhere

F1 shows the keys wherever you are. In a session: PageUp and the mouse
wheel scroll back while the live text keeps running below, Up and Down
recall earlier commands, Ctrl+R reconnects and Ctrl+Q quits. Shift+drag
selects text to copy.

## What it speaks

TCP and TLS; telnet with window size (NAWS), terminal type and MTTS, server
echo for hidden password input, prompts marked by EOR or GA, MCCP2
compression and UTF-8 through CHARSET. ANSI colour in 16, 256 and truecolor.
Health, mana and the like in the status line when the MUD sends GMCP
`Char.Vitals`.

Every session is logged as plain text per world. With `RUST_LOG=lera=debug`,
a diagnostic log goes to `lera.log` in the Asciiverse cache folder.

## Building

With Rust 1.88 or newer:

```sh
cargo build --release
```

## License

MIT, see LICENSE.
