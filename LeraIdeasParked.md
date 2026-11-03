# Lera: parked ideas

The original Lera design (written July 2026), parked on 2026-10-10 when Lera was
rescoped to a MUD client for every MUD, built to fit Asciiverse closely (see
`CLAUDE.md`). Kept as written for later; parts of it are out of date (it assumes an
Asciiverse server, a shared Lua layer, a companion window and an av mobile app,
none of which exist).

---


Asciiverse MUD client. Rust + Ratatui + Drawille.
Swedish: lera = clay, mud. MUD = Multi-User Dungeon = Multi-User Clay.
The primordial material from which worlds are shaped.

## The name

Lera — a perfect translation.
MUD → Lera. Multi-User Dungeon → Multi-User Clay.

English speakers hear a soft, memorable, slightly mysterious name.
Swedish speakers get the full translation immediately.
Anyone who digs deeper finds the clay metaphor: something malleable,
something that takes the shape of what you make in it. A more honest
description of what MUDs are than "dungeon" ever was.

Players don't play in a dungeon. They play in clay.
Lera — where worlds are made.

## Why MUDs matter for Asciiverse

726 active MUDs listed on MUDStats right now. The genre didn't die —
it became invisible to mainstream gaming culture. The survivors have
active daily playerbases, regular development, and gameplay depth that
most modern MMOs never achieved.

MUDs are the deep roots of multiplayer gaming. Text-based persistent
worlds, real communities, player-driven politics, permadeath, decades
of accumulated history. This is where online multiplayer began.

The problem: almost none of them have a proper TUI client with real
rendering. They're played through Mudlet (Lua-scriptable telnet client
with GUI) or raw telnet. Functional but visually bare — scrolling text,
maybe some colour, no spatial rendering of the world.

Lera brings the Asciiverse visual language to 726 existing games.
One client, every MUD.

## The surviving MUDs worth knowing

| MUD | Running since | Notable |
|---|---|---|
| Achaea | 1997 | Longest continuously developed MUD. Six city-states, player-run governments |
| Aardwolf | 1996 | 558,000 accounts, 27.6M quests. Best screen-reader MUD anywhere |
| Discworld MUD | Early 1990s | 1M+ rooms, Terry Pratchett's world, volunteer maintained, still active 2026 |
| BatMUD | 1990 | 30+ years, real-life player meetups, genuine longevity |
| Armageddon | 1993 | Hardcore permadeath roleplay, brutal desert world, mandatory in-character |
| Sindome | 1997 | Cyberpunk, Blade Runner meets Snow Crash, player economy, faction warfare |
| GemStone IV | 1987 | One of the oldest, commercial, still active |
| Legends of the Jedi | Active | Star Wars roleplay, iconic locations, accessible |

726 total active MUDs — these are the flagships.

## What Lera does differently

Current MUD clients (Mudlet, raw telnet, web clients) give you:
- Scrolling text output
- Basic colour (ANSI)
- Maybe a simple map panel
- Scripting for automation

Lera gives you the Asciiverse visual language applied to the entire genre:

**Spatial room rendering:**
Room descriptions are not just scrolling text. Lera constructs a spatial
representation of the room from the description and exit data — a Braille
rendered environment that gives you a sense of place. The room description
is still there, but the space is also visible.

**Client-side map construction:**
As you move through the world, Lera builds a map — not from ASCII art the
server sends (though it handles that too) but from movement data and room
connections. The map accumulates as you explore. Drawille rendering,
zoomable, persistent across sessions.

**Combat visualisation:**
Combat events rendered with Braille particle effects — not instead of
the text output but alongside it. The battle is happening in text AND
in visual space simultaneously.

**Colour system:**
The Asciiverse colour palette applied intelligently. Room types, NPC types,
item rarities all get consistent visual treatment. The world becomes readable
through colour in a way raw telnet never achieves.

**Typography:**
The Asciiverse house font loaded as the terminal font. Every MUD becomes
part of the same visual family.

## Architecture

Lera speaks standard MUD protocols:
- **Telnet** — universal baseline, every MUD
- **GMCP** (Generic Mud Communication Protocol) — structured data from
  server to client. Room data, character stats, map data, combat events.
  Most modern MUDs support this. Lera uses it for rich rendering when available.
- **MSDP** (Mud Server Data Protocol) — similar to GMCP, different syntax
- **MXP** (MUD eXtension Protocol) — markup for clickable links, images
- **MSP** (MUD Sound Protocol) — audio triggers

When a MUD supports GMCP, Lera gets structured data and renders richly.
When it doesn't, Lera falls back to intelligent text parsing.
Every MUD works. Better MUDs look better.

## Lua scripting layer

Mudlet's great strength is Lua scripting — players automate, create triggers,
build custom UIs. Lera inherits this with the Asciiverse Lua embedding
(mlua crate). The same Lua scripting layer used across Asciiverse games.

A Lera script is a Lua file. It can:
- Define triggers (text pattern → action)
- Define aliases (command shortcuts)
- Build custom HUD panels in Ratatui
- Drive the map construction logic
- Automate repetitive tasks
- Connect to the companion window event stream

The Lera scripting community is the most natural entry point for
developers who want to build on Asciiverse — they're already scripting,
they already know Lua, the step to building a full Asciiverse game is small.

## The companion window

Lera integrates naturally with the Asciiverse companion window:
- Character stats panel — health, mana, experience, level
- The constructed map in a dedicated window
- Combat log with visualisation
- Inventory and equipment overview
- Communication channels (tells, guild chat, global) separated from
  the main output stream

The companion window for a MUD session is the most complex and richest
use of the companion window system across the entire platform.

## AV mobile integration — persistence and notifications

MUDs are persistent worlds — things happen when you're not logged in.
Your character can be attacked. Guild politics shift. A quest timer
expires. An auction ends. The mobile companion app (av mobile) is the
natural notification layer for a world that never sleeps. This is the
most natural cross-platform connection in the whole Asciiverse catalog.

**Notifications:**
- Combat alerts — someone attacked your character
- Guild/faction events — political changes, declarations of war
- Tell messages — someone tried to reach you
- Quest timers — your planted crops are ready, your crafting finished
- Economic alerts — your auction item sold, market price changed
- World events — a dragon spawned, a siege began

**Persistence awareness:**
- Your character's current status visible on the phone — location,
  health, active effects
- The map you've built in Lera accessible on mobile — where you've
  been, where you haven't
- Session history — what happened while you were away, summarised

**Light mobile interaction:**
Not full MUD gameplay on mobile — the keyboard problem makes that
miserable. But lightweight actions:
- Reply to a tell
- Accept or decline a guild invitation
- Check auction status
- Set an away message
- Safe logout from anywhere

**The architecture:**
Lera runs on your machine, maintains the connection, proxies the mobile
app via the Asciiverse server or directly via `iroh` (P2P/QUIC) if on the
home network — same networking layer as the platform's multiplayer titles,
just point-to-point between desktop and phone instead of peer-to-peer
between players. The mobile app subscribes to your Lera session's event
stream — the same companion window event system, now extended to the phone.

This is also the pattern that makes the original Lera MUD world (below)
extraordinary — a world designed from day one knowing that its players
have a persistent mobile presence.

## Original Lera MUD world

Lera is a client. But Lera can also be the name of a world.

An original Asciiverse MUD built on Lera's infrastructure — not a
fantasy dungeon, not a Tolkien derivative. Something with the biological
depth of Antibiosis, the Nordic soul of Mið Journey, or something
entirely new. A world designed from first principles for the terminal
aesthetic, using Lera's full GMCP data stream for rich client rendering.

The world that shows what Lera can do when the server is built for it.

**Possible world concept:**
The clay itself as the game world. A creation myth where the world is
literally being formed as players explore it — the terra incognita
rendered as unmapped grey Braille until discovered, each room becoming
real as it is named and described. Players as co-creators of the world,
not just inhabitants. The MUD as a living collaborative fiction.

This is a separate design project. File separately when ready.

## MUD community outreach

The MUD community is small, passionate, and deeply knowledgeable.
They've been doing text-based multiplayer for forty years. They will
have opinions about Lera and most of them will be useful.

Key communities:
- MUDVerse (mudverse.com) — directory, rankings, community
- r/MUD (Reddit) — active discussion
- Top MUD Sites (topmudsites.com) — rankings, forums
- MUD Dev mailing list (historical but alumni still active)
- Mudlet community — the current client standard, worth engaging directly

The Mudlet team specifically — Lera is not trying to replace Mudlet
for its existing users. Lera is the Asciiverse experience for MUDs.
Different audiences, different aesthetics, room for both. Engage
the Mudlet community as peers, not competitors.

## Open questions / next steps

- [ ] GMCP implementation — the protocol that unlocks rich rendering.
      Study the spec, implement cleanly in Rust.
- [ ] Room rendering prototype — how does a room description become
      a spatial Braille rendering? What does the mapping heuristic look like?
- [ ] Map construction algorithm — from movement + room data to a
      persistent navigable map. This is a graph problem.
- [ ] Lua scripting integration — mlua crate, same as Asciiverse SDK.
      Ensure trigger/alias system is compatible with common Mudlet scripts
      so migration is easy.
- [ ] Test against major MUDs — Achaea, Aardwolf, Discworld.
      Confirm GMCP works, confirm fallback text parsing works.
- [ ] Companion window integration — design the MUD companion panels
- [ ] AV mobile event stream — design the proxy/subscription architecture
      connecting a live Lera session to the phone app
- [ ] Original Lera world — separate design document when ready
- [ ] MUD community engagement — post on r/MUD, MUDVerse when prototype exists

## Catalog position

Lera is different from every other Asciiverse title — it's a client,
not a game. But it brings 726 existing games into the Asciiverse
ecosystem in one move. The platform's reach multiplies overnight.

Every player who uses Lera to connect to Achaea or Discworld is an
Asciiverse user. Every MUD developer who recommends Lera to their
players is an Asciiverse advocate. The client is the platform's
most powerful distribution mechanism.

## Status

Design/ideation stage only. No code yet.
Name confirmed: Lera. Swedish for clay/mud. The translation is the concept.
Conceived 2026-07-02, scout camp.
GMCP protocol study and room rendering prototype are the first steps.
