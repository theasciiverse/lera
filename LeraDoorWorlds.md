# Lera: door worlds (future development)

BBS door games brought back as MUDs, played in Lera. Discussed 2026-10-20;
nothing built, nothing decided beyond the framing below. Belongs after
version 1, alongside the graphics spec in `CLAUDE.md`.

## The framing

A BBS door game is a MUD with a daily turn budget. Doors (LORD, Usurper,
Operation: Overkill II, Barren Realms Elite) were persistent shared worlds:
each player got a set number of turns or fights a day, played a short
session, and next time read the news of what everyone else had done
meanwhile, including attacks on their character while they were offline.
All of that is rules on a MUD server: a turn limit per 24 hours, offline
characters left in the world, a nightly maintenance pass, a news feed.

So there is no separate "modern door" medium to invent, and no door/MUD
choice to make per game. One shape covers both:

- **One engine**: a small MUD-style server, played in Lera.
- **Pacing is a setting per world**: turns per day for a door-style world,
  unlimited for a real-time one. The host of a world sets it.
- **Solo play**: the same server running on the player's own machine, with
  NPCs taking their own daily turns so the world lives without other
  players (Usurper Reborn's approach, see below).
- **Shared play**: someone hosts the server and friends join it.

## What Lera would add

Nothing door-specific in the protocol layer; door worlds are ordinary MUDs.
What they want from Lera is what the graphics spec already plans:

- **Semigraphical screens** through the open GMCP graphics spec: cell art
  for the ANSI screens doors were known for, scenes for maps and fights.
  Text stays complete, as the spec requires.
- **Panes for the door habits**: turns left today, the daily news, messages
  from other players. These could be messages in the spec's namespace so any
  door-style server can use them.
- **Starting a solo world**: Lera (or av) launching a local server for one
  player and connecting to it, so a door world plays like any installed
  game, offline.
- **Catalog**: a "Doors" shelf in av and on the website once there are
  titles for it, with a line explaining what a door was. Curation, not code.

## Doors with released source (checked 2026-10-20)

Most famous doors were closed DOS shareware and still are: LORD, Trade Wars
2002, Barren Realms Elite, Solar Realms Elite. The open things around them
are remakes (Legend of the Green Dragon, a PHP web game; Blacknova Traders),
not the originals. What was found with source:

- **Usurper** (Jakob Dangarden, 1993): GPL-2.0-or-later. Rick Parrish's
  32/64-bit Free Pascal port of 0.20e at github.com/rickparrish/Usurper
  (bug fixing by Dan Zingaro); a separate C# remake, Usurper Reborn,
  GPL-2.0, at github.com/binary-knight/usurper-reborn (active, a persistent
  world of 60+ autonomous NPCs, also runs as a BBS door via DOOR32.SYS and
  DOOR.SYS). The original's own blurb says "Sex, drugs and steroids", so it
  has to be checked against the no-sex-content rule before anything else.
- **The Clans** (Allen Ussher, 1997): source released 2002,
  sourceforge.net/projects/theclans. SourceForge lists GPLv2 and MIT; a
  GitHub copy (ryanfantus/clans-src) says BSD-3-Clause. The real terms have
  to be read in the source itself.
- **Thieves' Guild** (Paul Witte, Atari ST): source archived with
  permission at github.com/Kirkman/mythyn-archive; licence not checked.
- **Scott Baker's doors** (smbaker.com/games/old-source-code): only
  unfinished ones (NewGal, two RPG attempts, a trivia door); he keeps the
  copyright and asks 30% of any money made. Not worth pursuing.

There are probably more in the 1990s archives; no full sweep done yet.

## Operation: Overkill II

Dustin Nulf, with Bryan Turner of Vagabond Software on v1.12 (1998) and
v1.20 (2001); post-apocalyptic Earth in 2060; betas from 1990. v1.20 was
declared freeware with a public registration key; that frees the binary,
not the code. No source release found. The "OOII Source Kit" is the data
file formats for sysops, not the game. It is a 16-bit DOS door needing a
FOSSIL driver and a dropfile (GAP or DOOR.SYS). A fan site,
ooii.starflt.com, is still up.

Routes, in order of preference:

1. Ask Nulf and Turner for the source; with it, a restoration like Usurper.
2. A clean-room remake under our own name: same rules and feel, our own
   text and art.
3. Running the binary in DOSBox: not a port, skip.

## Open questions

- **Server engine.** `CLAUDE.md` names Evennia (BSD-3, Python) as the likely
  base for David's own MUDs. A solo world started on a player's machine
  would then need a bundled Python runtime (`asciiverse/BUNDLE-PYTHON.md`);
  a Rust server would not. Not decided.
- **Transport for shared worlds.** Lera speaks TCP and TLS; Asciiverse's
  multiplayer titles join over iroh. Whether a hosted door world is a plain
  TCP/TLS MUD, reachable over iroh, or both, is open.
- **Running on real BBSes.** A door world could also accept a DOOR32.SYS
  dropfile so it runs on today's BBSes (Synchronet, Mystic), as Usurper
  Reborn does. A cheap homage; not designed.
- **Shared layer.** No common "door kit" up front. Build one or two door
  worlds first and extract what they both hand-write (daily turn reset,
  nightly maintenance, news feed, world hosting), the same "validate before
  generalizing" order as elsewhere in Asciiverse.
