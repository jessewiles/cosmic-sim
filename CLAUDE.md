# Cosmic Sim — Claude Code Guide

## Build & Run

```bash
cargo build           # dev build
cargo build --release # release build
cargo run             # run in dev mode
```

Requires `ANTHROPIC_API_KEY` in the environment, or the game will prompt on first run and store it locally.

## Project Structure

```
src/
  main.rs               # all game loop logic, menus, UI rendering
  save.rs               # save/load game state (JSON via serde)
  ai/
    companion.rs        # Yael Orin and Reza Terani — personalities, system prompts
    computer.rs         # ShipComputer — wraps Anthropic API calls (streaming)
    mod.rs
  ui/
    terminal.rs         # ANSI helpers, prompt(), read_key(), menu_key(), typewrite()
    display.rs          # planet/system display formatting
  universe/
    catalog.rs          # real star catalog + known planets (Sol system etc.)
    system.rs           # StarSystem — procedural generation + catalog loading
    planet.rs           # Planet, PlanetType, InfraRisk enum
    star.rs             # Star properties
    galaxy.rs           # galaxy-level layout
  player/
    state.rs            # GameState — current system, location, hull, etc.
    ship.rs             # Ship — fuel, hull integrity
    inventory.rs        # resource inventory (He-3, D, fuel pellets)
  physics/
    orbital.rs          # Keplerian orbital mechanics
    relativity.rs       # time dilation calculations
    constants.rs
  chemistry/
    atmosphere.rs       # atmospheric composition generation
    element.rs          # element definitions
    compound.rs
  world/
    planet_surface.rs   # surface resource generation
    biome.rs
    resource.rs
```

## Key Concepts

### Narrative
The player is a digitized Martian consciousness departing Mars Station Ares-7 ~3.8 million years before the common era. Companions Yael Orin and Reza Terani travel in their own ships. ARIA is the player's ship AI. As the fleet travels, relativistic time dilation brings them closer to the present day.

### AI Companions
Each companion has a static personality string in `src/ai/companion.rs` injected into every API call via `companion_system_prompt()` in `main.rs`. The `ShipComputer` in `src/ai/computer.rs` handles the Anthropic API call with streaming output.

To add a new companion: add an entry to `default_companions()` in `companion.rs`.

### InfraRisk
Planets expose an `infrastructure_risk() -> InfraRisk` method based on temperature, pressure, and planet type. This replaces any biological framing — the player and companions are digital, so risk is about substrate survival, not breathability.

- `Minimal` / `Low` — safe to land, mine, operate
- `Moderate` — proceed with caution
- `High` — hull damage on landing (10–30% random)
- `Extreme` — instant destruction, game over

### Input
- `menu_key()` — single keypress (no Enter) via crossterm raw mode, used for all menu navigation
- `prompt()` — full line input via rustyline (emacs bindings: ctrl+a/e/k/u/w), used for free-text entry (travel destination, ARIA/companion chat)
- `typewrite()` — character-by-character output with punctuation-aware delays, used for ARIA narration and intro

### Resource Loop
Mine He-3 and deuterium from planetary surfaces → refine into fuel pellets via ship status `[r]` → use pellets to extend travel range. He-3 and D spawn on accessible (low-risk) planets.

### Save System
`save.rs` serializes `GameState` to JSON at `~/.cosmic-sim/save.json`. Auto-saved on quit.

## Releases

```bash
cargo release patch   # 0.2.1 → 0.2.2
cargo release minor   # 0.2.1 → 0.3.0
cargo release major   # 0.2.1 → 1.0.0
```

CI builds binaries for Linux (musl), macOS (x86_64 + arm64), and Windows on every tagged release. See `.github/workflows/release.yml`.

## Things to Know

- `main.rs` is large — all menus and game flow live there. The natural next refactor would be splitting menus into `src/ui/`.
- The game hardcodes `GalaxyMode::RealUniverse` — procedural galaxy mode exists in the code but is hidden from the UI.
- Mars is index 3 in the Sol planet list in `catalog.rs` and is set as `OceanWorld` for narrative purposes (habitable at departure time).
- `reqwest` uses `rustls-tls` with `default-features = false` — no system OpenSSL dependency, required for the musl cross-compile target.
