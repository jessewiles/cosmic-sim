# Cosmic Sim

A terminal-based space exploration game set 3.8 million years before the common era. You are a digitized Martian consciousness, departing Mars Station Ares-7 alongside two fellow travelers — Dr. Yael Orin and Reza Terani — as your civilization embarks on an interstellar voyage into an unmapped galaxy.

As you travel, time dilation closes the gap between your departure and the present day. The universe you explore is built from real star catalog data, with procedurally generated planetary systems.

## Features

- **Real star catalog** — navigate to actual nearby stars with procedurally generated planets
- **AI companions** — converse freely with Yael and Reza via the Anthropic API; they have personalities, opinions, and grief
- **ARIA** — your ship's AI, context-aware of your current system and discoveries
- **Resource loop** — mine He-3 and deuterium, refine fuel pellets, extend your range
- **Infrastructure risk** — planetary conditions determine whether your digital substrate survives landing
- **Time dilation** — travel speed affects the passage of time relative to the wider universe

## Requirements

- An [Anthropic API key](https://console.anthropic.com/)
- A terminal with ANSI color support

## Installation

### Pre-built binaries

Download the latest release for your platform from the [Releases](../../releases) page.

### Build from source

```bash
cargo build --release
./target/release/cosmic-sim
```

## Setup

On first run, the game will prompt you for your Anthropic API key if `ANTHROPIC_API_KEY` is not set in your environment. The key is stored locally for future sessions.

## Gameplay

```
[1] System scan      — survey the current star system
[2] Travel           — jump to a nearby star
[3] Land             — descend to a planet surface
[r] Ship status      — inventory, fuel, hull integrity; refine fuel pellets here
[c] Fleet comms      — talk to Yael or Reza
[a] Hail ARIA        — ask your ship's AI anything about your situation
```

**Objective:** mine He-3 and deuterium from planetary surfaces, refine them into fuel pellets via ship status `[r]`, and travel further into the galaxy. Infrastructure risk ratings (`MINIMAL` → `EXTREME`) determine landing safety — extreme-risk environments will destroy your substrate.
