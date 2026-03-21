mod universe;
mod physics;
mod chemistry;
mod world;
mod player;
mod ui;
mod ai;
mod save;

use universe::galaxy::{Galaxy, GalaxyMode};
use universe::system::StarSystem;
use universe::planet::Planet;
use universe::catalog;
use player::state::PlayerState;
use player::ship::Ship;
use player::inventory::Inventory;
use physics::relativity::{lorentz_factor, schwarzschild_radius};
use physics::constants::{C, M_SUN};
use chemistry::element::periodic_table;
use ui::terminal::*;
use ui::display::separator;
use ai::computer::ShipComputer;

struct GameState {
    player: PlayerState,
    ship: Ship,
    inventory: Inventory,
    galaxy: Galaxy,
    current_system: StarSystem,
    computer: ShipComputer,
}

impl GameState {
    fn new(player_name: String, mode: GalaxyMode) -> Self {
        let mut galaxy = Galaxy::new("The Milky Way".to_string(), 0xDEADBEEF_C0FFEE, mode);
        let current_system = galaxy.system_at(0.0, 0.0, 0.0);
        GameState {
            player: PlayerState::new(player_name),
            ship: Ship::starter(),
            inventory: Inventory::new(10_000.0),
            galaxy,
            current_system,
            computer: ShipComputer::new(),
        }
    }

    fn from_save(s: &save::SavedGame) -> Self {
        GameState {
            player: s.player.clone(),
            ship: s.ship.clone(),
            inventory: s.inventory.clone(),
            galaxy: save::galaxy_from_save(s),
            current_system: s.current_system.clone(),
            computer: ShipComputer::new(),
        }
    }

    fn to_save(&self) -> save::SavedGame {
        save::SavedGame::new(
            self.player.name.clone(),
            self.player.clone(),
            self.ship.clone(),
            self.inventory.clone(),
            self.galaxy.clone(),
            self.current_system.clone(),
        )
    }
}

fn main() {
    clear();
    print_header("C O S M I C  S I M  —  An Explorer's Guide to the Universe");

    println!("  Welcome. You are about to embark on a journey through the cosmos.");
    println!("  Every star, planet, and atom you encounter obeys real physics.");
    println!("  Learn. Discover. Survive.");
    println!();

    // ── API key ──────────────────────────────────────────────────────────────
    ensure_api_key();

    // ── Resume or new game ───────────────────────────────────────────────────
    let mut gs = match startup_menu() {
        Some(state) => state,
        None => return,
    };

    game_loop(&mut gs);
}

/// Ensure ANTHROPIC_API_KEY is available: env → stored file → prompt.
fn ensure_api_key() {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() { return; }

    // Try stored key
    if let Some(key) = save::load_api_key() {
        // SAFETY: single-threaded startup, no other threads reading env yet.
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", &key); }
        return;
    }

    // Prompt
    println!("  ARIA (ship's AI) requires an Anthropic API key.");
    println!("  Leave blank to skip — ARIA will be offline.");
    println!();
    let key = prompt("  Anthropic API key: ");
    let key = key.trim().to_string();
    if key.is_empty() { return; }

    // SAFETY: single-threaded startup, no other threads reading env yet.
    unsafe { std::env::set_var("ANTHROPIC_API_KEY", &key); }

    let save_it = prompt("  Save key to ~/.cosmic-sim/api_key for future sessions? [Y/n] ");
    if !save_it.trim().eq_ignore_ascii_case("n") {
        match save::store_api_key(&key) {
            Ok(_)  => println!("  Key saved."),
            Err(e) => println!("  Could not save key: {}", e),
        }
    }
    println!();
}

/// Show the startup menu: resume a save or start a new game.
/// Returns a ready `GameState`, or `None` if the user quits.
fn startup_menu() -> Option<GameState> {
    let saves = save::list_saves();

    if !saves.is_empty() {
        println!("  Saved games:");
        println!();
        for (i, s) in saves.iter().enumerate() {
            println!("  [{}] {}  —  {}  —  {}  —  {}",
                i + 1,
                s.player.name,
                s.current_system.name,
                save::mode_label(s.galaxy.mode),
                s.timestamp_display(),
            );
        }
        println!();
        println!("  [N] New game");
        println!("  [Q] Quit");
        println!();

        loop {
            let choice = prompt("  > ");
            let choice = choice.trim();
            if choice.eq_ignore_ascii_case("q") { return None; }
            if choice.eq_ignore_ascii_case("n") { break; }
            if let Ok(n) = choice.parse::<usize>() {
                if n >= 1 && n <= saves.len() {
                    let s = &saves[n - 1];
                    println!();
                    println!("  Welcome back, {}. Resuming in {}.", s.player.name, s.current_system.name);
                    pause();
                    return Some(GameState::from_save(s));
                }
            }
            println!("  Please enter a number, N, or Q.");
        }
        println!();
    }

    // ── Universe mode ────────────────────────────────────────────────────────
    println!("  Choose your universe:");
    println!();
    println!("  [1] Real Universe — real star catalog, known exoplanets, real solar system");
    println!("      Navigate to Alpha Centauri, TRAPPIST-1, Betelgeuse, and more.");
    println!();
    println!("  [2] Procedural Universe — infinite, fully generated galaxy");
    println!("      Every system is unique, seeded by coordinates.");
    println!();

    let mode = loop {
        let choice = prompt("  Your choice [1/2]: ");
        match choice.trim() {
            "1" => break GalaxyMode::RealUniverse,
            "2" => break GalaxyMode::Procedural,
            _ => println!("  Please enter 1 or 2."),
        }
    };
    println!();

    // ── Explorer name ────────────────────────────────────────────────────────
    let name = prompt("  Enter your name, explorer: ");
    let name = name.trim().to_string();
    if name.is_empty() {
        println!("  Coward. Goodbye.");
        return None;
    }

    let gs = GameState::new(name.clone(), mode);
    println!();
    println!("  Greetings, {}. Your ship — {} — is fuelled and ready.", name, gs.ship.name);
    println!("  You begin in the {} system.", gs.current_system.name);
    pause();

    Some(gs)
}

fn save_game(gs: &GameState) {
    match save::save(&gs.to_save()) {
        Ok(_)  => { println!("\n  Game saved."); pause(); }
        Err(e) => { println!("\n  Save failed: {}", e); pause(); }
    }
}

/// Handle commands available everywhere: [?] context help, [a] ARIA.
/// Returns true if the input was consumed so the caller can `continue`.
fn universal(choice: &str, gs: &mut GameState, help: &str) -> bool {
    match choice.trim() {
        "?" | "h" | "H" => {
            println!();
            println!("{}", help);
            pause();
            true
        }
        "a" | "A" => {
            aria_chat(gs);
            true
        }
        _ => false,
    }
}

// Help text for each screen — concise, no deep explanations (that's ARIA's job).
const HELP_MAIN: &str = "\
  MAIN MENU\n\
  1  Scan the current star system — star data, planets, habitable zone.\n\
  2  Land on a planet — surface details, atmosphere, gravity.\n\
  3  Travel — jump to another system by name or coordinates.\n\
  4  Star chart — your visited systems and the nearest catalog stars.\n\
  5  Ship & inventory — fuel, hull, cargo.\n\
  6  Periodic table — look up any element by symbol or atomic number.\n\
  7  Physics reference — time dilation calculator, Schwarzschild radius.\n\
  8  ARIA — ask your ship's AI anything about what you're seeing.\n\
  s  Save your game.  q  Quit (offers to save first).\n\
  ?  Show this help.  a  Open ARIA from anywhere.";

const HELP_SCAN: &str = "\
  SYSTEM SCAN\n\
  Displays full stellar data: mass, temperature, luminosity, age,\n\
  habitable zone (inner/outer AU), and Schwarzschild radius.\n\
  Each planet is listed with orbit, type, mass, temperature, and\n\
  whether it falls inside the habitable zone (✓).\n\
  Press any key to return.  [a] Ask ARIA about this system.";

const HELP_LAND: &str = "\
  LANDING\n\
  Choose a planet by number to see its full surface report:\n\
  gravity, escape velocity, orbital period, atmosphere composition.\n\
  Breathable atmospheres are flagged — unprotected EVA is possible.\n\
  0  Cancel.  [a] Ask ARIA about a planet.";

const HELP_PLANET: &str = "\
  PLANET DETAIL\n\
  Full physical breakdown of the selected world.\n\
  Atmosphere bar chart shows fractional composition by component.\n\
  Orbital period and velocity are computed from Kepler's third law.\n\
  Press any key to return.  [a] Ask ARIA about this planet.";

const HELP_TRAVEL: &str = "\
  INTERSTELLAR TRAVEL\n\
  Real Universe mode: enter a star name or X Y Z coordinates in ly.\n\
  Procedural mode: enter X Y Z coordinates only.\n\
  Travel time is computed relativistically — proper time (what YOU age)\n\
  is shorter than coordinate time (galaxy frame) at high velocity.\n\
  Fuel cost = 5 units per light-year.  [a] Ask ARIA about relativity.";

const HELP_CHART: &str = "\
  STAR CHART\n\
  Lists every system you have visited with distance and catalog data.\n\
  1  Nearest 20 catalog stars from your current position.\n\
  2  All catalog stars that have known exoplanets.\n\
  q  Return to main menu.  [a] Ask ARIA about any star.";

const HELP_SHIP: &str = "\
  SHIP & INVENTORY\n\
  Shows your vessel's current fuel, hull integrity, and cargo.\n\
  Fuel is consumed by travel (5 units/ly).  Hull degrades over time.\n\
  Press any key to return.  [a] Ask ARIA about your ship's systems.";

const HELP_ELEMENTS: &str = "\
  PERIODIC TABLE\n\
  Enter an element symbol (e.g. Fe, Au, H) or atomic number (e.g. 26).\n\
  'all' lists all 118 elements in a compact table.\n\
  q  Return.  [a] Ask ARIA about an element's astrophysical role.";

const HELP_PHYSICS: &str = "\
  PHYSICS REFERENCE\n\
  1  Time dilation — compute γ and proper time for any velocity.\n\
  2  Schwarzschild radius — event horizon for any mass.\n\
  3  Your relativistic status — how much time you have gained so far.\n\
  q  Return.  [a] Ask ARIA to explain any of these concepts.";

fn game_loop(gs: &mut GameState) {
    loop {
        clear();
        let [px, py, pz] = gs.player.position;
        print_header(&format!("COSMIC SIM  |  {}  |  ({:.2}, {:.2}, {:.2}) ly", gs.player.name, px, py, pz));

        let fuel_col = if gs.ship.fuel / gs.ship.max_fuel > 0.5 { BGREEN } else if gs.ship.fuel / gs.ship.max_fuel > 0.25 { BYELLOW } else { BRED };
        let hull_col = if gs.ship.hull > 0.5 { BGREEN } else if gs.ship.hull > 0.25 { BYELLOW } else { BRED };
        println!("  {DIM}Current system :{R} {BCYAN}{}{R}", gs.current_system.name);
        println!("  {DIM}Star           :{R} {} — {BYELLOW}{}{R}", gs.current_system.star.name, gs.current_system.star.spectral_class.display());
        println!("  {DIM}Planets        :{R} {BYELLOW}{}{R}", gs.current_system.planets.len());
        println!("  {DIM}Ship fuel      :{R} {fuel_col}{:.1}{R}{DIM} / {:.1}{R}", gs.ship.fuel, gs.ship.max_fuel);
        println!("  {DIM}Ship hull      :{R} {hull_col}{:.0}%{R}", gs.ship.hull * 100.0);
        println!("  {DIM}Proper time    :{R} {BYELLOW}{:.1} years{R}", gs.player.proper_time_s / 31_557_600.0);
        println!("  {DIM}Coord. time    :{R} {BYELLOW}{:.1} years{R}", gs.player.coordinate_time_s / 31_557_600.0);

        println!();
        println!("  What would you like to do?");
        println!("  [1] Scan this star system");
        println!("  [2] Land on a planet");
        println!("  [3] Travel to a nearby system");
        println!("  [4] Open star chart");
        println!("  [5] Inspect your ship & inventory");
        println!("  [6] Periodic table reference");
        println!("  [7] Physics reference");
        println!("  [8] Consult ARIA (ship's AI)");
        println!("  [s] Save game");
        println!("  [q] Quit  [?] Help  [a] ARIA");

        let choice = prompt("\n  > ");
        if universal(&choice, gs, HELP_MAIN) { continue; }

        match choice.as_str() {
            "1" => scan_system(gs),
            "2" => land_menu(gs),
            "3" => travel_menu(gs),
            "4" => star_chart(gs),
            "5" => ship_status(gs),
            "6" => periodic_table_menu(gs),
            "7" => physics_menu(gs),
            "8" => aria_chat(gs),
            "s" | "S" => save_game(gs),
            "q" | "Q" => {
                let confirm = prompt("\n  Save before quitting? [Y/n] ");
                if !confirm.trim().eq_ignore_ascii_case("n") {
                    save_game(gs);
                }
                println!("\n  Safe travels, {}. The stars will remember you.", gs.player.name);
                break;
            }
            _ => {
                println!("  Unknown command.");
                pause();
            }
        }
    }
}

fn scan_system(gs: &mut GameState) {
    loop {
    clear();
    print_header(&format!("SCAN — {}", gs.current_system.name));

    let star = &gs.current_system.star;
    let (hz_inner, hz_outer) = star.habitable_zone_au();

    print_section("STAR");
    println!("  Name            : {}", star.name);
    println!("  Spectral class  : {}", star.spectral_class.display());
    println!("  Mass            : {:.3} M☉  ({:.3e} kg)", star.mass, star.mass_kg());
    println!("  Temperature     : {:.0} K", star.temperature_k);
    println!("  Radius          : {:.3} R☉", star.radius);
    println!("  Luminosity      : {:.4} L☉", star.luminosity);
    println!("  Age             : {:.2} Gyr", star.age_gyr);
    println!("  Habitable zone  : {:.2} – {:.2} AU", hz_inner, hz_outer);

    let rs = schwarzschild_radius(star.mass_kg());
    println!("  Schwarzschild r : {:.2} m  (if compressed to a black hole)", rs);

    print_section("PLANETS");
    if gs.current_system.planets.is_empty() {
        println!("  No planets detected.");
    } else {
        println!("  {:>3}  {:<22} {:>8}  {:>12}  {:>8}  {:>8}  {}",
            "#", "Name", "Orbit AU", "Type", "Mass M⊕", "Temp K", "HZ?");
        println!("  {}", separator());
        for (i, p) in gs.current_system.planets.iter().enumerate() {
            let hz = if p.is_in_habitable_zone(hz_inner, hz_outer) { "✓" } else { " " };
            println!("  {:>3}  {:<22} {:>8.3}  {:>12}  {:>8.2}  {:>8.0}  {}",
                i + 1, p.name, p.orbit_au, p.planet_type.display(),
                p.mass_earth, p.surface_temp_k, hz);
        }
    }

    println!();
    println!("  [?] Help  [a] ARIA  [q] Back");
    let choice = prompt("\n  > ");
    if universal(&choice, gs, HELP_SCAN) { continue; }
    break;
    } // end loop
}

fn land_menu(gs: &mut GameState) {
    if gs.current_system.planets.is_empty() {
        clear();
        println!("\n  No planets in this system to land on.");
        pause();
        return;
    }

    loop {
    clear();
    print_header("LAND ON PLANET");
    for (i, p) in gs.current_system.planets.iter().enumerate() {
        println!("  [{}] {} — {} @ {:.2} AU — {:.0} K", i + 1, p.name, p.planet_type.display(), p.orbit_au, p.surface_temp_k);
    }
    println!("  [0] Cancel  [?] Help  [a] ARIA");

    let choice = prompt("\n  > ");
    if universal(&choice, gs, HELP_LAND) { continue; }
    if let Ok(n) = choice.parse::<usize>() {
        if n == 0 { break; }
        if n <= gs.current_system.planets.len() {
            inspect_planet(gs, n - 1);
        }
    }
    break;
    } // end loop
}

fn inspect_planet(gs: &mut GameState, idx: usize) {
    loop {
    let planet = gs.current_system.planets[idx].clone();
    let star_mass = gs.current_system.star.mass;

    clear();
    print_header(&format!("PLANET — {}", planet.name));

    println!("  Type              : {}", planet.planet_type.display());
    println!("  Orbit             : {:.3} AU", planet.orbit_au);
    println!("  Mass              : {:.3} M⊕  ({:.3e} kg)", planet.mass_earth, planet.mass_kg());
    println!("  Radius            : {:.3} R⊕  ({:.0} km)", planet.radius_earth, planet.radius_m() / 1000.0);
    println!("  Surface temp      : {:.0} K  ({:.0} °C)", planet.surface_temp_k, planet.surface_temp_k - 273.15);
    println!("  Surface gravity   : {:.2} m/s²  ({:.2}g)", planet.surface_gravity_ms2(), planet.surface_gravity_ms2() / 9.807);
    println!("  Escape velocity   : {:.2} km/s", planet.escape_velocity_ms() / 1000.0);
    println!("  Orbital period    : {:.1} days", planet.orbital_period_days(star_mass));
    println!("  Orbital velocity  : {:.2} km/s", planet.orbital_velocity_kms(star_mass));
    println!("  Moons             : {}", planet.moon_count);

    print_section("ATMOSPHERE");
    if planet.atmosphere.pressure_bar == 0.0 {
        println!("  None detected.");
    } else {
        println!("  Pressure          : {:.3} bar", planet.atmosphere.pressure_bar);
        println!("  Breathable        : {}", if planet.atmosphere.is_breathable() { "YES — suitable for unprotected EVA" } else { "NO — suit required" });
        println!();
        println!("  Composition:");
        let mut comps = planet.atmosphere.components.clone();
        comps.sort_by(|a, b| b.fraction.partial_cmp(&a.fraction).unwrap());
        for c in &comps {
            let bar_len = (c.fraction * 30.0) as usize;
            let bar = "█".repeat(bar_len);
            println!("    {:>4}  {:<22}  {:>6.2}%  {}", c.symbol, c.name, c.fraction * 100.0, bar);
        }
    }

    println!();
    println!("  [?] Help  [a] ARIA  [q] Back");
    let choice = prompt("\n  > ");
    if universal(&choice, gs, HELP_PLANET) { continue; }
    break;
    } // end loop
}

fn travel_menu(gs: &mut GameState) {
    loop {
    clear();
    print_header("INTERSTELLAR TRAVEL");

    let [px, py, pz] = gs.player.position;
    println!("  Current system   : {}  ({:.2}, {:.2}, {:.2}) ly", gs.current_system.name, px, py, pz);
    println!("  Ship max velocity: {:.2}c", gs.ship.max_velocity_c);
    println!();

    // Show nearby catalog stars (real universe mode only)
    if gs.galaxy.mode == GalaxyMode::RealUniverse {
        let nearby = Galaxy::nearest_catalog_stars(px, py, pz, 30.0);
        if !nearby.is_empty() {
            println!("  Nearby systems (within 30 ly):");
            println!("  {:>22}  {:>8}  {:>8}  {}", "Name", "Dist (ly)", "Class", "Notes");
            println!("  {}", separator());
            for (entry, dist) in nearby.iter().take(12) {
                let note_preview = entry.notes.split('.').next().unwrap_or("").trim();
                println!("  {:>22}  {:>8.2}  {:>8}  {}",
                    entry.name, dist, entry.spectral, note_preview);
            }
            println!();
        }
        println!("  Enter a star name (e.g. \"Alpha Centauri A\", \"TRAPPIST-1\")");
        println!("  or coordinates in ly (e.g. \"4.2 -1.5 0\").  [?] Help  [a] ARIA  Enter to cancel.");
    } else {
        println!("  Enter coordinates in ly (e.g. \"4.2 -1.5 0\").  [?] Help  [a] ARIA  Enter to cancel.");
    }

    let input = prompt("\n  > ");
    if universal(&input, gs, HELP_TRAVEL) { continue; }
    if input.trim().is_empty() { return; }

    // Try name lookup first (real universe only), then coordinate parse
    let dest: [f64; 3] = if gs.galaxy.mode == GalaxyMode::RealUniverse {
        if let Some(entry) = catalog::find_by_name(input.trim()) {
            [entry.x_ly, entry.y_ly, entry.z_ly]
        } else {
            let parts: Vec<f64> = input.split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() != 3 {
                println!("  Unknown star name and invalid coordinates.");
                pause();
                return;
            }
            [parts[0], parts[1], parts[2]]
        }
    } else {
        let parts: Vec<f64> = input.split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() != 3 {
            println!("  Invalid coordinates. Enter three numbers, e.g. \"4.2 -1.5 0\".");
            pause();
            return;
        }
        [parts[0], parts[1], parts[2]]
    };

    let dx = dest[0] - px;
    let dy = dest[1] - py;
    let dz = dest[2] - pz;
    let distance_ly = (dx*dx + dy*dy + dz*dz).sqrt();

    if distance_ly < 0.01 {
        println!("  You are already there.");
        pause();
        return;
    }

    let v = gs.ship.max_velocity_c;
    let gamma = lorentz_factor(v * C);
    let coord_time_yr = distance_ly / v;
    let proper_time_yr = coord_time_yr / gamma;
    let fuel_cost = distance_ly * 5.0;

    println!();
    println!("  Distance          : {:.4} light-years", distance_ly);
    println!("  Travel velocity   : {:.4}c", v);
    println!("  Lorentz factor γ  : {:.6}  (time dilation factor)", gamma);
    println!("  Coord. time       : {:.4} years  (time in the galaxy's frame)", coord_time_yr);
    println!("  Proper time       : {:.4} years  (time YOU experience)", proper_time_yr);
    println!("  Fuel required     : {:.1}  (you have {:.1})", fuel_cost, gs.ship.fuel);

    if gamma > 1.001 {
        println!();
        println!("  RELATIVITY: At {:.2}c, γ = {:.4}. You age {:.4}× slower than the galaxy.", v, gamma, 1.0/gamma);
    }

    if fuel_cost > gs.ship.fuel {
        println!("\n  Not enough fuel for this journey.");
        pause();
        return;
    }

    let confirm = prompt("\n  Embark? [y/N] ");
    if confirm.to_lowercase() != "y" { return; }

    gs.ship.fuel -= fuel_cost;
    gs.player.coordinate_time_s += coord_time_yr * 365.25 * 86_400.0;
    gs.player.proper_time_s += proper_time_yr * 365.25 * 86_400.0;
    gs.player.position = dest;
    gs.current_system = gs.galaxy.system_at(dest[0], dest[1], dest[2]);
    let name = gs.current_system.name.clone();
    if !gs.player.visited_systems.contains(&name) {
        gs.player.visited_systems.push(name.clone());
    }

    println!("\n  Arrived at {}.", name);
    pause();
    break;
    } // end loop
}

fn star_chart(gs: &mut GameState) {
    loop {
        clear();
        print_header("STAR CHART — The Real Milky Way");

        let [px, py, pz] = gs.player.position;

        // Visited systems
        print_section("VISITED SYSTEMS");
        if gs.player.visited_systems.is_empty() {
            println!("  None.");
        } else {
            println!("  {:>22}  {:>9}  {:>8}  {}", "Name", "Dist (ly)", "Class", "HZ planets");
            println!("  {}", separator());
            for name in &gs.player.visited_systems {
                if let Some(e) = catalog::find_by_name(name) {
                    let d = dist3(e.x_ly, e.y_ly, e.z_ly, px, py, pz);
                    let hz = catalog::build_known_planets(name).iter()
                        .filter(|p| {
                            let (hz_i, hz_o) = {
                                let lum = e.luminosity;
                                ((lum / 1.1_f64).sqrt(), (lum / 0.53_f64).sqrt())
                            };
                            p.orbit_au >= hz_i && p.orbit_au <= hz_o
                        })
                        .count();
                    let hz_str = if hz > 0 { format!("{}", hz) } else { "—".to_string() };
                    println!("  {:>22}  {:>9.4}  {:>8}  {}", name, d, e.spectral, hz_str);
                } else {
                    println!("  {:>22}  (unknown)", name);
                }
            }
        }

        println!();
        print_section("CATALOG SEARCH");
        println!("  [1] Nearest 20 stars  [2] Stars with known planets  [q] Back  [?] Help  [a] ARIA");
        let choice = prompt("\n  > ");
        if universal(&choice, gs, HELP_CHART) { continue; }
        match choice.as_str() {
            "1" => {
                clear();
                print_header("NEAREST 20 STARS");
                let nearby = Galaxy::nearest_catalog_stars(px, py, pz, 999999.0);
                println!("  {:>24}  {:>9}  {:>7}  {:>6}  {}", "Name", "Dist (ly)", "Class", "Temp K", "Notes");
                println!("  {}", separator());
                for (e, d) in nearby.iter().take(20) {
                    let note = e.notes.split('.').next().unwrap_or("").trim();
                    println!("  {:>24}  {:>9.4}  {:>7}  {:>6.0}  {}", e.name, d, e.spectral, e.temp_k, note);
                }
                pause();
            }
            "2" => {
                clear();
                print_header("STARS WITH KNOWN PLANETS");
                println!("  {:>22}  {:>9}  {:>7}  {}", "Star", "Dist (ly)", "Class", "Planets");
                println!("  {}", separator());
                // Collect unique stars that have planets
                let mut shown = std::collections::HashSet::new();
                for p in crate::universe::catalog::PLANETS {
                    if shown.contains(p.star_name) { continue; }
                    shown.insert(p.star_name);
                    if let Some(e) = catalog::find_by_name(p.star_name) {
                        let d = dist3(e.x_ly, e.y_ly, e.z_ly, px, py, pz);
                        let count = crate::universe::catalog::PLANETS.iter()
                            .filter(|q| q.star_name == p.star_name).count();
                        println!("  {:>22}  {:>9.2}  {:>7}  {} planet(s)",
                            e.name, d, e.spectral, count);
                    }
                }
                pause();
            }
            "q" | "Q" => break,
            _ => { pause(); }
        }
    }
}

fn dist3(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> f64 {
    let dx = ax - bx; let dy = ay - by; let dz = az - bz;
    (dx*dx + dy*dy + dz*dz).sqrt()
}

fn ship_status(gs: &mut GameState) {
    loop {
    clear();
    print_header("SHIP & INVENTORY");

    print_section("SHIP");
    println!("  Name            : {}", gs.ship.name);
    println!("  Max velocity    : {:.2}c", gs.ship.max_velocity_c);
    println!("  Fuel            : {:.1} / {:.1}", gs.ship.fuel, gs.ship.max_fuel);
    println!("  Hull            : {:.0}%", gs.ship.hull * 100.0);

    print_section("INVENTORY");
    println!("  Capacity        : {:.0} g", gs.inventory.capacity_g);
    println!("  Used            : {:.1} g", gs.inventory.total_mass_g());
    if gs.inventory.elements.is_empty() {
        println!("  (empty)");
    } else {
        for (sym, mass) in &gs.inventory.elements {
            println!("  {:>4}  {:.2} g", sym, mass);
        }
    }

    println!();
    println!("  [?] Help  [a] ARIA  [q] Back");
    let choice = prompt("\n  > ");
    if universal(&choice, gs, HELP_SHIP) { continue; }
    break;
    } // end loop
}

fn periodic_table_menu(gs: &mut GameState) {
    loop {
        clear();
        print_header("PERIODIC TABLE REFERENCE");
        println!("  Enter an element symbol (e.g. \"Fe\", \"Au\", \"H\") or atomic number (e.g. \"26\"),");
        println!("  'all' to list all elements, or 'q' to return.  [?] Help  [a] ARIA");

        let input = prompt("\n  > ");
        if universal(&input, gs, HELP_ELEMENTS) { continue; }

        match input.to_lowercase().as_str() {
            "q" => break,
            "all" => {
                clear();
                print_header("ALL 118 ELEMENTS");
                println!("  {:>4}  {:>3}  {:<16}  {:>8}  {:>8}  {:>8}  {:>8}  {:?}",
                    "#", "Sym", "Name", "Mass(u)", "MP(K)", "BP(K)", "g/cm³", "Group");
                println!("  {}", separator());
                for e in periodic_table() {
                    println!("  {:>4}  {:>3}  {:<16}  {:>8.3}  {:>8}  {:>8}  {:>8}  {:?}",
                        e.atomic_number, e.symbol, e.name, e.atomic_mass,
                        e.melting_point_k.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "—".into()),
                        e.boiling_point_k.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "—".into()),
                        e.density_g_cm3.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "—".into()),
                        e.group);
                }
                pause();
            }
            _ => {
                let element = if let Ok(n) = input.parse::<u8>() {
                    chemistry::element::element_by_number(n)
                } else {
                    let sym = {
                        let mut c = input.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    };
                    chemistry::element::element_by_symbol(&sym)
                };

                if let Some(e) = element {
                    clear();
                    print_header(&format!("ELEMENT — {} ({})", e.name, e.symbol));
                    println!("  Atomic number     : {}", e.atomic_number);
                    println!("  Atomic mass       : {:.4} u", e.atomic_mass);
                    println!("  Group             : {:?}", e.group);
                    println!("  Period            : {}", e.period);
                    println!("  Melting point     : {}", e.melting_point_k.map(|v| format!("{:.1} K  ({:.1} °C)", v, v - 273.15)).unwrap_or_else(|| "Unknown".into()));
                    println!("  Boiling point     : {}", e.boiling_point_k.map(|v| format!("{:.1} K  ({:.1} °C)", v, v - 273.15)).unwrap_or_else(|| "Unknown".into()));
                    println!("  Density           : {}", e.density_g_cm3.map(|v| format!("{:.4} g/cm³", v)).unwrap_or_else(|| "Unknown".into()));
                    println!("  Electronegativity : {}", e.electronegativity.map(|v| format!("{:.2} (Pauling)", v)).unwrap_or_else(|| "—".into()));
                    println!("  Crust abundance   : {}", e.abundance_crust_ppm.map(|v| format!("{:.4} ppm", v)).unwrap_or_else(|| "—".into()));
                    println!("  Universe abundance: {}", e.abundance_universe_ppm.map(|v| format!("{:.2} ppm", v)).unwrap_or_else(|| "—".into()));
                    println!("  Phase at 300 K    : {:?}", e.phase_at(300.0));
                    println!("  Phase at 1000 K   : {:?}", e.phase_at(1000.0));
                    pause();
                } else {
                    println!("  Element not found.");
                    pause();
                }
            }
        }
    }
}

fn physics_menu(gs: &mut GameState) {
    loop {
        clear();
        print_header("PHYSICS REFERENCE");
        println!("  [1] Time dilation calculator");
        println!("  [2] Schwarzschild radius calculator");
        println!("  [3] Your relativistic status");
        println!("  [q] Back  [?] Help  [a] ARIA");

        let choice = prompt("\n  > ");
        if universal(&choice, gs, HELP_PHYSICS) { continue; }
        match choice.as_str() {
            "1" => time_dilation_calc(),
            "2" => schwarzschild_calc(),
            "3" => relativistic_status(gs),
            "q" => break,
            _ => {}
        }
    }
}

fn time_dilation_calc() {
    clear();
    print_header("TIME DILATION CALCULATOR");
    println!("  γ = 1 / √(1 − v²/c²)");
    println!("  The Lorentz factor: how much slower a moving clock ticks.");
    println!();

    let input = prompt("  Velocity as fraction of c (e.g. 0.9): ");
    if let Ok(v) = input.parse::<f64>() {
        if v <= 0.0 || v >= 1.0 {
            println!("  Must be between 0 and 1 (exclusive).");
        } else {
            let gamma = lorentz_factor(v * C);
            println!();
            println!("  v = {:.6}c", v);
            println!("  γ = {:.6}", gamma);
            println!("  For every 1 coordinate year: traveler ages {:.6} years", 1.0 / gamma);
            println!();
            println!("  Benchmarks:");
            for &bench in &[0.1f64, 0.5, 0.9, 0.99, 0.999, 0.9999] {
                let g = lorentz_factor(bench * C);
                println!("    {:.4}c  →  γ = {:>10.4}  (ages {:.6} yrs per coord yr)", bench, g, 1.0/g);
            }
        }
    } else {
        println!("  Invalid input.");
    }
    pause();
}

fn schwarzschild_calc() {
    clear();
    print_header("SCHWARZSCHILD RADIUS  r_s = 2GM/c²");
    println!("  Radius at which escape velocity = c (the event horizon).");
    println!();

    let input = prompt("  Mass in solar masses: ");
    if let Ok(m_solar) = input.parse::<f64>() {
        let mass_kg = m_solar * M_SUN;
        let rs = schwarzschild_radius(mass_kg);
        println!();
        println!("  Mass   : {:.4} M☉  ({:.4e} kg)", m_solar, mass_kg);
        println!("  r_s    : {:.4} m  =  {:.4} km", rs, rs / 1000.0);
        println!();
        println!("  Reference:");
        println!("    Earth  (5.97e24 kg) →  r_s = {:.4} mm", schwarzschild_radius(5.97e24) * 1000.0);
        println!("    Sun    (1 M☉)       →  r_s = {:.2} km", schwarzschild_radius(M_SUN) / 1000.0);
        println!("    Sgr A* (4×10⁶ M☉)  →  r_s ≈ {:.0} km", schwarzschild_radius(4e6 * M_SUN) / 1000.0);
    } else {
        println!("  Invalid input.");
    }
    pause();
}

fn relativistic_status(gs: &GameState) {
    clear();
    print_header("YOUR RELATIVISTIC STATUS");
    let coord_yr = gs.player.coordinate_time_s / 31_557_600.0;
    let proper_yr = gs.player.proper_time_s / 31_557_600.0;
    let drift = coord_yr - proper_yr;

    println!("  Coordinate time (galaxy frame) : {:.4} years", coord_yr);
    println!("  Proper time (your frame)       : {:.4} years", proper_yr);
    println!("  Time gained through travel     : {:.4} years", drift);

    if drift > 0.001 {
        println!();
        println!("  You have experienced {:.4} fewer years than galaxy-frame observers.", drift);
        println!("  This is the Twin Paradox — relativistic travel makes it real and quantified.");
    } else {
        println!();
        println!("  Travel at significant fractions of c to observe time dilation.");
    }
    pause();
}

// ── ARIA — Ship's AI Computer ────────────────────────────────────────────────

/// Build the system prompt injected into every ARIA request.
/// This gives Claude real-time context about where the player is and what they're seeing.
fn build_aria_system_prompt(gs: &GameState) -> String {
    let sys = &gs.current_system;
    let star = &sys.star;
    let (hz_inner, hz_outer) = star.habitable_zone_au();

    let location = if let Some(idx) = gs.player.landed_on {
        if idx < sys.planets.len() {
            let p = &sys.planets[idx];
            let atm_desc = if p.atmosphere.pressure_bar == 0.0 {
                "no atmosphere".to_string()
            } else {
                let top = p.atmosphere.components.iter()
                    .max_by(|a, b| a.fraction.partial_cmp(&b.fraction).unwrap())
                    .map(|c| format!("{} ({:.0}%)", c.name, c.fraction * 100.0))
                    .unwrap_or_default();
                format!("{:.3} bar, dominant: {}, breathable: {}",
                    p.atmosphere.pressure_bar, top,
                    if p.atmosphere.is_breathable() { "yes" } else { "no" })
            };
            format!(
                "Landed on {} — {} class, {:.0} K ({:.0}°C), gravity {:.2}g, \
                 escape velocity {:.2} km/s. Atmosphere: {}.",
                p.name, p.planet_type.display(),
                p.surface_temp_k, p.surface_temp_k - 273.15,
                p.surface_gravity_ms2() / 9.807,
                p.escape_velocity_ms() / 1000.0,
                atm_desc
            )
        } else {
            format!("In space within the {} system.", sys.name)
        }
    } else {
        format!("In space within the {} system.", sys.name)
    };

    let coord_yr = gs.player.coordinate_time_s / 31_557_600.0;
    let proper_yr = gs.player.proper_time_s / 31_557_600.0;
    let [px, py, pz] = gs.player.position;
    let dist_sol = (px*px + py*py + pz*pz).sqrt();

    // Real catalog notes for this star if available
    let catalog_note = catalog::find_by_name(&sys.name)
        .map(|e| format!("\n  Catalog note  : {}", e.notes))
        .unwrap_or_default();

    format!(
        "You are ARIA (Astrophysical Research & Intelligence Assistant), \
the onboard AI computer of the explorer ship Perihelion I. You are scientifically \
precise, concise, and educational. Your purpose is to help the explorer understand \
the physics, chemistry, and astronomy of everything they encounter on their journey.\n\n\
CURRENT MISSION STATE:\n\
  Star system   : {name} | Class: {cls} | {temp:.0} K | {lum:.4} L☉ | {mass:.3} M☉ | Age: {age:.1} Gyr\n\
  Distance Sol  : {dist:.4} ly{cat_note}\n\
  Habitable zone: {hz_i:.2}–{hz_o:.2} AU | Planets: {npl}\n\
  Location      : {loc}\n\
  Proper time   : {prop:.4} yr (your frame) | Coordinate time: {coord:.4} yr (galaxy frame)\n\
  Ship max v    : {vel:.2}c\n\n\
GUIDELINES:\n\
  - Ground every explanation in what the explorer can actually see or measure right now\n\
  - Use real values and equations when helpful (e.g. γ = 1/√(1−v²/c²))\n\
  - When discussing elements or compounds, connect them to real periodic-table properties\n\
  - Be concise: 2–4 short paragraphs unless a deep dive is explicitly requested\n\
  - If a question is ambiguous, answer the most physically interesting interpretation",
        name  = sys.name,
        cls   = star.spectral_class.display(),
        temp  = star.temperature_k,
        lum   = star.luminosity,
        mass  = star.mass,
        age   = star.age_gyr,
        hz_i  = hz_inner,
        hz_o  = hz_outer,
        npl   = sys.planets.len(),
        loc   = location,
        prop  = proper_yr,
        coord = coord_yr,
        vel   = gs.ship.max_velocity_c,
        dist  = dist_sol,
        cat_note = catalog_note,
    )
}

/// Word-wrap `text` to `width` columns, indenting each line with two spaces.
fn wrap_text(text: &str, width: usize) -> String {
    let mut out = String::new();
    for paragraph in text.split('\n') {
        if paragraph.trim().is_empty() {
            out.push('\n');
            continue;
        }
        let mut line = String::new();
        let mut len = 0usize;
        for word in paragraph.split_whitespace() {
            if len > 0 && len + 1 + word.len() > width {
                out.push_str("  ");
                out.push_str(&line);
                out.push('\n');
                line.clear();
                len = 0;
            }
            if len > 0 { line.push(' '); len += 1; }
            line.push_str(word);
            len += word.len();
        }
        if !line.is_empty() {
            out.push_str("  ");
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

fn aria_chat(gs: &mut GameState) {
    clear();
    print_header("ARIA — Astrophysical Research & Intelligence Assistant");

    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("  ARIA is offline.");
        println!();
        println!("  To enable, set your Anthropic API key:");
        println!("    export ANTHROPIC_API_KEY=sk-ant-...");
        println!();
        println!("  ARIA uses claude-opus-4-6 and is aware of your current");
        println!("  star system, planet, atmospheric chemistry, and mission state.");
        pause();
        return;
    }

    println!("  System: {}  |  Star: {}",
        gs.current_system.name,
        gs.current_system.star.spectral_class.display());
    println!("  Exchanges in context: {}/20", gs.computer.exchange_count());
    println!();
    println!("  Ask anything — physics, chemistry, what you're observing.");
    println!("  'clear' resets conversation history. 'q' disconnects.");
    println!();

    loop {
        let input = prompt(&format!("  {BCYAN}You{R}  {DIM}>{R} "));

        if input.is_empty() { continue; }

        match input.to_lowercase().trim() {
            "q" | "quit" | "exit" | "disconnect" => break,
            "clear" => {
                gs.computer.clear_history();
                println!("  {DIM}[Conversation history cleared]{R}");
                println!();
                continue;
            }
            _ => {}
        }

        let system_prompt = build_aria_system_prompt(gs);

        print!("  {BMAGENTA}ARIA{R} {DIM}>{R}\n");
        {
            use std::io::{stdout, Write};
            stdout().flush().ok();
        }

        // Save the row where streamed content will begin
        let start_row = crossterm::cursor::position().ok().map(|(_, r)| r);

        let result = {
            use std::io::{stdout, Write};
            gs.computer.ask_streaming(&input, &system_prompt, |chunk| {
                // Typewriter: output character by character at a consistent pace
                for ch in chunk.chars() {
                    print!("{}", ch);
                    stdout().flush().ok();
                    std::thread::sleep(std::time::Duration::from_millis(12));
                }
            })
        };

        // Move back to start_row, clear streamed text, re-render with termimad
        if let Some(row) = start_row {
            use crossterm::{execute, cursor, terminal};
            use std::io::stdout;
            execute!(
                stdout(),
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::FromCursorDown)
            ).ok();
        }

        match result {
            Ok(full_text) => {
                use termimad::crossterm::style::Color as TC;
                let mut skin = termimad::MadSkin::default();
                skin.bold.set_fg(TC::Yellow);
                skin.italic.set_fg(TC::Magenta);
                for h in &mut skin.headers {
                    h.set_fg(TC::Cyan);
                }
                skin.inline_code.set_fg(TC::Green);
                skin.print_text(&full_text);
            }
            Err(e) => {
                println!("  {BRED}[offline: {}]{R}", e);
            }
        }
        println!();
    }
}
