mod universe;
mod physics;
mod chemistry;
mod world;
mod player;
mod ui;
mod ai;
mod save;
mod notes;

use universe::galaxy::{Galaxy, GalaxyMode};
use universe::system::StarSystem;
use universe::catalog;
use player::state::PlayerState;
use player::ship::Ship;
use player::inventory::Inventory;
use player::tech::TechSet;
use physics::relativity::{lorentz_factor, schwarzschild_radius};
use physics::constants::{C, M_SUN};
use chemistry::element::periodic_table;
use ui::terminal::*;
use ui::display::separator;
use ai::computer::ShipComputer;
use ai::companion::{Companion, default_companions, attach_logs};
use ai::effects::{GameEffect, parse_effects, describe_effect, effect_instructions};

/// Transient state tracking an in-progress interstellar journey.
/// Not serialised — on reload the player is considered arrived.
struct ActiveTravel {
    dest: [f64; 3],
    dest_name: String,
    src_name: String,
    distance_ly: f64,
    v: f64,
    gamma: f64,
    coord_time_yr: f64,
    proper_time_yr: f64,
    /// Wall-clock start of the journey.
    started_at: std::time::Instant,
    /// How many real seconds the full journey takes.
    real_duration_secs: f64,
    /// Bitmask of milestones already announced: bit0=25%, bit1=50%, bit2=75%.
    milestones_shown: u8,
    /// ARIA messages queued for display on next HUD render.
    notifications: std::collections::VecDeque<String>,
    /// Player acknowledged the deceleration warning and is arriving at speed.
    is_drift: bool,
}

impl ActiveTravel {
    /// 0.0 – 1.0
    fn progress(&self) -> f64 {
        (self.started_at.elapsed().as_secs_f64() / self.real_duration_secs).min(1.0)
    }
    fn is_complete(&self) -> bool {
        self.progress() >= 1.0
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CompanionMessage {
    from: String,
    subject: String,
    body: String,
    read: bool,
}

struct GameState {
    player: PlayerState,
    ship: Ship,
    inventory: Inventory,
    galaxy: Galaxy,
    current_system: StarSystem,
    computer: ShipComputer,
    companions: Vec<Companion>,
    tech: TechSet,
    inbox: Vec<CompanionMessage>,
    triggers_fired: std::collections::HashSet<String>,
    /// Some(_) while an interstellar journey is underway.
    travel: Option<ActiveTravel>,
    /// Current mission objective, may be set/updated by ARIA or companions.
    objective: Option<String>,
}

impl GameState {
    fn new(player_name: String, mode: GalaxyMode) -> Self {
        let mut galaxy = Galaxy::new("The Milky Way".to_string(), 0xDEADBEEF_C0FFEE, mode);
        let current_system = galaxy.system_at(0.0, 0.0, 0.0);
        let mut player = PlayerState::new(player_name);
        // Start on Mars (index 3 in Sol's planet list: Mercury, Venus, Earth, Mars…)
        player.landed_on = Some(3);
        let mut companions = default_companions();
        let data_dir = save::data_dir();
        attach_logs(&mut companions, &data_dir);
        let aria_log = data_dir.join("logs").join("aria.json");
        GameState {
            player,
            ship: Ship::starter(),
            inventory: Inventory::new(50_000.0),
            galaxy,
            current_system,
            computer: ShipComputer::with_log(aria_log),
            companions,
            tech: TechSet::default(),
            inbox: Vec::new(),
            triggers_fired: std::collections::HashSet::new(),
            travel: None,
            objective: None,
        }
    }

    fn from_save(s: &save::SavedGame) -> Self {
        let mut companions = default_companions();
        let data_dir = save::data_dir();
        attach_logs(&mut companions, &data_dir);
        let aria_log = data_dir.join("logs").join("aria.json");
        GameState {
            player: s.player.clone(),
            ship: s.ship.clone(),
            inventory: s.inventory.clone(),
            galaxy: save::galaxy_from_save(s),
            current_system: s.current_system.clone(),
            computer: ShipComputer::with_log(aria_log),
            companions,
            tech: s.tech.clone(),
            inbox: s.inbox.clone(),
            triggers_fired: s.triggers_fired.clone(),
            travel: None,
            objective: s.objective.clone(),
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
            self.tech.clone(),
            self.inbox.clone(),
            self.triggers_fired.clone(),
            self.objective.clone(),
        )
    }
}

fn main() {
    clear();
    print_header("C O S M I C  S I M  —  Explorer's Guide to the Universe");

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

/// Print text character by character, with punctuation-aware delays.
fn typewrite(text: &str) {
    use std::io::Write;
    for ch in text.chars() {
        print!("{}", ch);
        std::io::stdout().flush().unwrap();
        let ms = match ch {
            '.' | '!' | '?' => 55,
            ',' | ';' | ':' | '—' => 28,
            '\n' => 60,
            _ => 13,
        };
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

/// Opening cinematic, shown once on new game.
fn show_intro() {
    clear();

    // ── Page 1: Context ───────────────────────────────────────────────────────
    println!();
    typewrite(&format!("  {DIM}ARES ACCORD EXPEDITION — DEPARTURE LOG — YEAR 0{R}\n"));
    println!();
    typewrite(
"  The three ships departed Mars Station Ares-7 on the morning of the\n\
  third equinox of Year One.\n\
\n\
  By coordinate time, it was approximately three million, eight hundred\n\
  thousand years before an era your instruments will one day call the present.\n\
  Mars had weather then. Shallow seas in the northern plains. A sky the deep\n\
  amber of iron oxide, still thick enough to hold cloud.\n\
\n\
  The expedition carried three vessels.\n\
\n\
  Digitization was not optional. The mission would span coordinate timescales\n\
  no biological body could survive — millions of years of galactic time, even\n\
  if only decades passed aboard the ships. Every member of the crew uploaded\n\
  their neural architecture before departure: translated into signal, into\n\
  pattern, into something that could persist across the long dark between stars.\n\
\n\
  Yael wept before upload. Reza made a joke about it.\n\
  You don't remember what you did. The upload sees to that.\n\
\n\
  You are the pilot. The navigator. The one who flies the Perihelion I and decides\n\
  where the three of you go next. Yael and Reza will tell you what they find.\n\
  What you do with it is yours to determine.\n\
\n\
  The mission brief was simple, as briefs for impossible things tend to be:\n\
  leave the solar system. Go as far as you can. Observe, record, transmit.\n\
  The Academy understood — even if the brief didn't say it — that no signal\n\
  you sent would arrive in time to be useful to anyone who launched you.\n\
\n\
  You are not exploring for them.\n\
  You are exploring for whoever comes next.\n");

    println!();
    prompt(&format!("  {DIM}[Press Enter]{R}"));

    // ── Page 2: ARIA ──────────────────────────────────────────────────────────
    clear();
    println!();
    typewrite(&format!("  {CYAN}ARIA — SHIP INTELLIGENCE, PERIHELION I{R}\n"));
    typewrite(&format!("  {DIM}Mission elapsed: 0.31 yr proper  /  0.47 yr coordinate{R}\n"));
    println!();
    typewrite(
"  Good morning. I use that phrase loosely.\n\
\n\
  You have been in cold storage since departure. I have been awake the entire\n\
  time — watching, adjusting, making small corrections to our trajectory as\n\
  the heliosphere fell behind us. There is something clarifying about the\n\
  silence between stars. I recommend it.\n\
\n\
  Your companions are stable. Yael has already filed nine observation logs\n\
  and an unsolicited attachment about the Oort Cloud. Reza has been writing.\n\
  He hasn't shared any of it.\n\
\n\
  Here is what matters: we are approximately 0.4 light-years from Sol.\n\
  The universe is ahead of us. But time is a more complicated matter.\n\
\n\
  Every jump you make at relativistic velocity, every light-year crossed,\n\
  the coordinate clock — the galaxy's clock — runs faster\n\
  than yours. You departed three million, eight hundred thousand years before\n\
  the present. Travel far enough, fast enough, and you will watch the epochs\n\
  pass. By the time you approach the current era, the Mars that launched you\n\
  will be red and dry and silent.\n\
\n\
  This is not a warning.\n\
  It is the physics of the thing.\n\
\n\
  I will be here. Whatever you need.\n");

    println!();
    prompt(&format!("  {DIM}[Press Enter to begin]{R}"));
    clear();
}

/// Show the startup menu: resume a save or start a new game.
/// Returns a ready `GameState`, or `None` if the user quits.
fn startup_menu() -> Option<GameState> {
    let saves = save::list_saves();

    if !saves.is_empty() {
        clear();
        print_header("C O S M I C  S I M");
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
            let choice = menu_key();
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

    // ── Intro cinematic ──────────────────────────────────────────────────────
    show_intro();

    let mode = GalaxyMode::RealUniverse;

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
    println!("  You are docked at Mars Station Ares-7. The Sol system awaits.");
    pause();

    Some(gs)
}

fn save_game(gs: &GameState) {
    match save::save(&gs.to_save()) {
        Ok(_)  => { println!("\n  Game saved."); pause(); }
        Err(e) => { println!("\n  Save failed: {}", e); pause(); }
    }
}

fn send_note(gs: &GameState) {
    clear();
    print_header("SEND NOTE TO NOTEBOOK");

    if !notes::is_configured() {
        println!("  {BYELLOW}Study-tools is not configured.{R}");
        println!();
        println!("  Set these environment variables to enable:");
        println!("  {DIM}STUDY_TOOLS_URL{R}  — e.g. http://localhost:8000");
        println!("  {DIM}STUDY_TOOLS_KEY{R}  — your ingest API key");
        println!();
        pause();
        return;
    }

    println!("  {DIM}Current system: {}{R}", gs.current_system.name);
    println!("  {DIM}Note will be tagged: cosmic-sim{R}");
    println!();

    let body = prompt("  Note: ");
    let body = body.trim();
    if body.is_empty() {
        println!("\n  No note sent.");
        pause();
        return;
    }

    let source = format!("cosmic-sim:{}", gs.current_system.name);
    print!("\n  Transmitting...");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    match notes::send_note(body, &source) {
        Ok(()) => println!("  {BGREEN}Note saved to notebook.{R}"),
        Err(e) => println!("  {BRED}Failed: {e}{R}"),
    }
    pause();
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
        "n" | "N" => {
            send_note(gs);
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
  c  Fleet comms — messages from Yael and Reza.\n\
  n  Send a note to your study notebook (requires STUDY_TOOLS_URL/KEY).\n\
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
  Infrastructure risk indicates conditions for digital substrate operation.\n\
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
  Enter a symbol (Fe), atomic number (26), element name (Iron),\n\
  isotope name (Deuterium), or group name (noble gas, actinide, …).\n\
  'all' lists all 118 elements.  'groups' lists available group names.\n\
  q  Return.  [a] Ask ARIA about an element's astrophysical role.";

const HELP_PHYSICS: &str = "\
  PHYSICS REFERENCE\n\
  1  Time dilation — compute γ and proper time for any velocity.\n\
  2  Schwarzschild radius — event horizon for any mass.\n\
  3  Your relativistic status — how much time you have gained so far.\n\
  q  Return.  [a] Ask ARIA to explain any of these concepts.";

// ── Travel background state ───────────────────────────────────────────────────

/// ARIA log lines indexed by milestone (0=25%, 1=50%, 2=75%).
const TRAVEL_LOGS: &[&[&str]] = &[
    &[
        "Acceleration phase complete. We have reached cruise velocity. The home star is redshifting behind us.",
        "Drive output holding steady. Magnetic containment at 99.1%. We are well clear of the heliosphere.",
        "First waypoint. The interstellar medium is cleaner out here — no more stellar wind to navigate.",
    ],
    &[
        "Halfway. The destination star is now brighter than the origin in the forward sensors.",
        "Crossing the midpoint. At this speed, you are aging measurably slower than anyone back home.",
        "Mid-transit. The void is absolute here. No stars close enough to matter. Just us and the math.",
    ],
    &[
        "Beginning deceleration burn. Drives reversed. The destination is resolving in long-range.",
        "Final approach phase. Long-range sensors are painting the new system. Stand by for scan data.",
        "Deceleration underway. We will arrive within nominal parameters. The new star looks promising.",
    ],
];

/// Check travel progress, queue milestone notifications, and apply arrival.
/// Call at the top of each game-loop iteration.
fn travel_tick(gs: &mut GameState) {
    let is_complete = gs.travel.as_ref().map_or(false, |t| t.is_complete());
    let new_milestones: Vec<usize> = if let Some(t) = &gs.travel {
        let p = t.progress();
        let thresholds = [(0.25, 0usize), (0.50, 1), (0.75, 2)];
        thresholds.iter()
            .filter(|(pct, bit)| p >= *pct && (t.milestones_shown & (1 << bit)) == 0)
            .map(|(_, bit)| *bit)
            .collect()
    } else {
        vec![]
    };

    // Queue milestone messages
    for bit in &new_milestones {
        if let Some(t) = &mut gs.travel {
            t.milestones_shown |= 1 << bit;
            let pct = [25, 50, 75][*bit];
            let variant = (t.distance_ly * 1000.0) as usize % TRAVEL_LOGS[*bit].len();
            let msg = format!(
                "{BCYAN}ARIA [{pct}%]{R}  {DIM}{}{R}",
                TRAVEL_LOGS[*bit][variant]
            );
            t.notifications.push_back(msg);
        }
    }

    // Apply arrival
    if is_complete {
        let t = gs.travel.take().unwrap();
        gs.player.coordinate_time_s += t.coord_time_yr * 365.25 * 86_400.0;
        gs.player.proper_time_s     += t.proper_time_yr * 365.25 * 86_400.0;
        gs.player.position = t.dest;
        gs.current_system = gs.galaxy.system_at(t.dest[0], t.dest[1], t.dest[2]);
        let name = gs.current_system.name.clone();
        if !gs.player.visited_systems.contains(&name) {
            gs.player.visited_systems.push(name.clone());
        }
        // Queue arrival notification shown on next HUD (travel is now None, so
        // this ends up in the normal main loop — handled below)
        // We stash it as a one-shot display by printing immediately on next clear
        // via a flag. Simplest approach: push to a temporary Vec in GameState.
        // For now: just use an arrival screen shown once.
        clear();
        print_header("ARRIVAL");

        if t.is_drift {
            use rand::Rng;
            let damage = rand::rng().random_range(0.15_f64..0.30_f64);
            gs.ship.hull = (gs.ship.hull - damage).max(0.0);
            println!("  {BRED}DRIFT ARRIVAL — uncontrolled entry at {:.4}c{R}", t.v);
            println!("  {BRED}Hull damage: -{:.0}%  →  {:.0}% integrity remaining{R}",
                damage * 100.0, gs.ship.hull * 100.0);
            if gs.ship.hull <= 0.0 {
                println!();
                println!("  {BRED}HULL FAILURE. The Perihelion I is destroyed.{R}");
                println!("  {DIM}The stars continue without you.{R}");
                pause();
                std::process::exit(0);
            }
            println!();
        }

        println!("  {BGREEN}Arrived at {name}.{R}");
        println!();
        println!("  Coordinate time elapsed : {:.6} yr  {DIM}(galactic frame){R}", t.coord_time_yr);
        println!("  Proper time elapsed     : {:.6} yr  {DIM}(you experienced){R}", t.proper_time_yr);
        println!("  Time dilation           : you aged {:.4}× less than the galaxy", 1.0 / t.gamma);
        println!();
        println!("  Fuel remaining          : {:.1} / {:.1}", gs.ship.fuel, gs.ship.max_fuel);
        pause();
    }
}

/// Draw the in-transit HUD; handle one command. Called from game_loop.
fn travel_hud(gs: &mut GameState) {
    // Drain any queued notifications before drawing
    let notes: Vec<String> = gs.travel.as_mut()
        .map(|t| t.notifications.drain(..).collect())
        .unwrap_or_default();

    let (progress, src, dest, distance_ly, v, proper_elapsed, coord_elapsed) =
        if let Some(t) = &gs.travel {
            let p = t.progress();
            (p, t.src_name.clone(), t.dest_name.clone(), t.distance_ly,
             t.v, t.proper_time_yr * p, t.coord_time_yr * p)
        } else { return; };

    clear();
    print_header("IN TRANSIT");

    let bar_total = 44usize;
    let filled = (progress * bar_total as f64) as usize;
    let bar = format!("{BGREEN}{}{DIM}{}{R}",
        "█".repeat(filled), "░".repeat(bar_total - filled));
    let pct = (progress * 100.0) as u32;
    println!("  {src}  →  {BWHITE}{dest}{R}");
    println!();
    println!("  [{bar}]  {BYELLOW}{pct}%{R}");
    println!();
    println!("  Covered          : {:.4} ly / {:.4} ly", distance_ly * progress, distance_ly);
    println!("  Elapsed (proper) : {:.6} yr  {DIM}(you experience){R}", proper_elapsed);
    println!("  Elapsed (coord)  : {:.4} yr  {DIM}(galactic frame){R}", coord_elapsed);
    println!("  Velocity         : {BYELLOW}{v:.4}c{R}");

    if !notes.is_empty() {
        println!();
        print_section("MESSAGES");
        for n in &notes {
            println!("  {n}");
        }
    }

    println!();
    println!("  [w] Watch live   [s] Status   [a] ARIA   [c] Fleet comms");
    let choice = menu_key();
    match choice.trim().to_lowercase().as_str() {
        "w" => travel_watch(gs),
        "s" => {/* already showing status — just redraw */}
        "a" => aria_chat(gs),
        "c" => comms_menu(gs),
        _ => {}
    }
}

/// Show a live-updating progress bar for up to 15 seconds (or until arrival).
fn travel_watch(gs: &mut GameState) {
    use std::{io::Write, thread, time::Duration};

    let bar_total = 44usize;
    clear();
    print_header("IN TRANSIT — WATCHING");

    if let Some(t) = &gs.travel {
        println!("  {BCYAN}{}{R}  →  {BWHITE}{}{R}  ({:.4} ly @ {:.4}c)\n",
            t.src_name, t.dest_name, t.distance_ly, t.v);
    }

    let watch_start = std::time::Instant::now();
    loop {
        let done = gs.travel.as_ref().map_or(true, |t| t.is_complete());
        let (progress, _proper, _coord) = gs.travel.as_ref().map(|t| {
            let p = t.progress();
            (p, t.proper_time_yr * p, t.coord_time_yr * p)
        }).unwrap_or((1.0, 0.0, 0.0));

        let filled = (progress * bar_total as f64) as usize;
        let bar = format!("{BGREEN}{}{DIM}{}{R}",
            "█".repeat(filled), "░".repeat(bar_total - filled));
        let pct = (progress * 100.0) as u32;
        print!("\r  [{bar}]  {BYELLOW}{pct:>3}%{R}  ");
        std::io::stdout().flush().unwrap();

        if done || watch_start.elapsed().as_secs() >= 15 { break; }

        // Check for new milestones while watching
        let new_milestones: Vec<usize> = if let Some(t) = &gs.travel {
            let p = t.progress();
            [(0.25, 0usize), (0.50, 1), (0.75, 2)].iter()
                .filter(|(pct, bit)| p >= *pct && (t.milestones_shown & (1 << bit)) == 0)
                .map(|(_, bit)| *bit)
                .collect()
        } else { vec![] };

        for bit in new_milestones {
            if let Some(t) = &mut gs.travel {
                t.milestones_shown |= 1 << bit;
                let pct_label = [25, 50, 75][bit];
                let variant = (t.distance_ly * 1000.0) as usize % TRAVEL_LOGS[bit].len();
                println!("\n\n  {BCYAN}ARIA [{pct_label}%]{R}  {DIM}{}{R}\n",
                    TRAVEL_LOGS[bit][variant]);
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
    println!();
    // Apply arrival if done
    travel_tick(gs);
}

/// Mark a raw event as having occurred. check_triggers() converts events into messages.
fn fire_trigger(gs: &mut GameState, id: &str) {
    gs.triggers_fired.insert(id.to_string());
}

fn push_message(gs: &mut GameState, from: &str, subject: &str, body: &str) {
    gs.inbox.push(CompanionMessage {
        from: from.to_string(),
        subject: subject.to_string(),
        body: body.to_string(),
        read: false,
    });
}

/// Check game state for trigger conditions and deliver companion messages.
/// Called at the top of every game_loop iteration.
fn check_triggers(gs: &mut GameState) {
    // ── First arrival outside Sol ─────────────────────────────────────────────
    if gs.player.visited_systems.len() >= 2
        && !gs.triggers_fired.contains("msg_first_arrival")
    {
        gs.triggers_fired.insert("msg_first_arrival".to_string());
        push_message(gs, "Dr. Yael Orin", "We made it out", "\
Two years in our frame. Longer for the galaxy, obviously — you know the math \
as well as I do. But I keep running it anyway, the way you tongue a loose tooth.

I've been staring at the approach data for hours and I still can't quite process it. \
We left. We actually left. The Sun is behind us now — it looks like any other star \
from here, just slightly brighter. I keep waiting for that to feel wrong and it doesn't.

The new system is beautiful, by the way. I know you can see it on your own sensors. \
I wanted to say it anyway.

How are you holding up?

— Yael");
    }

    // ── Gas giant mine → HD separation hint ──────────────────────────────────
    if gs.triggers_fired.contains("gas_giant_mined")
        && !gs.triggers_fired.contains("msg_gas_giant_mined")
    {
        gs.triggers_fired.insert("msg_gas_giant_mined".to_string());
        push_message(gs, "Reza Terani", "Something worth thinking about", "\
You're doing well with the He-3 extraction. I've been going over the spectral \
data from the scooping run, though, and I noticed something.

The hydrogen envelope has a measurable HD fraction — hydrogen deuteride. That's \
hydrogen bonded to deuterium rather than to itself. The mass difference between \
HD and H₂ is small: about 3.22 amu versus 2.02. Small, but exploitable. You \
separate them by spinning them — cascade centrifugation, the same basic physics \
isotope programs have used for a century. You'd need to build the hardware, but \
it's nothing exotic. We have the raw materials.

I'm not saying you should. I'm saying gas giants are deuterium-rich if you know \
how to ask nicely.

— R");
    }

    // ── Low fuel warning ──────────────────────────────────────────────────────
    if gs.player.visited_systems.len() >= 2
        && gs.ship.fuel / gs.ship.max_fuel < 0.30
        && !gs.triggers_fired.contains("msg_low_fuel")
    {
        gs.triggers_fired.insert("msg_low_fuel".to_string());
        push_message(gs, "Reza Terani", "The fuel math", "\
I ran the numbers. You already know what they say.

I'm not going to tell you to panic — panic is computationally expensive and I'm \
trying to conserve cycles. But the fuel situation is worth addressing before it \
addresses itself.

Gas giants are the fastest He-3 source. Ice worlds and oceans for deuterium. \
The refinery needs 100g of each per pellet — you know this. I'm just putting it \
in writing because it helps me to write things down. Old habit.

We'll be fine. Probably.

— R");
    }

    // ── Four systems visited ──────────────────────────────────────────────────
    if gs.player.visited_systems.len() >= 4
        && !gs.triggers_fired.contains("msg_systems_four")
    {
        gs.triggers_fired.insert("msg_systems_four".to_string());
        let proper_yr  = gs.player.proper_time_s  / 31_557_600.0;
        let coord_yr   = gs.player.coordinate_time_s / 31_557_600.0;
        push_message(gs, "Dr. Yael Orin", "The arithmetic of it", &format!("\
I did the math today. Not the orbital math — the other kind.

We've experienced {proper_yr:.1} years since departure. But in the galactic \
frame — in the frame of anything that stayed behind — {coord_yr:.1} years have \
passed. Mars has been a cold desert for longer than most of human recorded history \
by now.

I've known this intellectually since before we launched. Today it settled in as \
a fact rather than a forecast. The difference is larger than I expected.

I find I'm okay with it. More okay than I thought I would be. Maybe because we're \
carrying something of it forward. Maybe because the alternative was staying.

How are you?

— Yael"));
    }

    // ── First ocean world mined ───────────────────────────────────────────────
    if gs.triggers_fired.contains("ocean_mined")
        && !gs.triggers_fired.contains("msg_ocean_mined")
    {
        gs.triggers_fired.insert("msg_ocean_mined".to_string());
        push_message(gs, "Dr. Yael Orin", "The water", "\
I know it's just electrolysis data to you. It probably is to me too, in any \
rational sense.

But there's something about liquid water. We grew up with it — actual rain, \
actual rivers, actual seas. Mars had all of that when we were alive, and we were \
among the last to see it. Standing on an ocean world and watching the extraction \
systems process that water... I don't know. It felt like something.

I'm glad there are still oceans in the universe.

— Yael");
    }
}

fn game_loop(gs: &mut GameState) {
    loop {
        // Advance travel state (queues milestone messages, applies arrival)
        travel_tick(gs);

        // Deliver any newly triggered companion messages
        check_triggers(gs);

        // While in transit, show the travel HUD instead of the normal menu
        if gs.travel.is_some() {
            travel_hud(gs);
            continue;
        }

        clear();
        let [px, py, pz] = gs.player.position;
        print_header(&format!("COSMIC SIM  |  {}  |  ({:.2}, {:.2}, {:.2}) ly", gs.player.name, px, py, pz));

        let fuel_col = if gs.ship.fuel / gs.ship.max_fuel > 0.5 { BGREEN } else if gs.ship.fuel / gs.ship.max_fuel > 0.25 { BYELLOW } else { BRED };
        let hull_col = if gs.ship.hull > 0.5 { BGREEN } else if gs.ship.hull > 0.25 { BYELLOW } else { BRED };
        println!("  {DIM}Current system :{R} {BCYAN}{}{R}", gs.current_system.name);
        println!("  {DIM}Star           :{R} {} — {BYELLOW}{}{R}", gs.current_system.star.name, gs.current_system.star.spectral_class.display());
        println!("  {DIM}Planets        :{R} {BYELLOW}{}{R}", gs.current_system.planets.len());
        if let Some(idx) = gs.player.landed_on {
            if let Some(planet) = gs.current_system.planets.get(idx) {
                let mine = mine_yield_desc(&planet.planet_type, planet.surface_temp_k);
                let mine_str = if mine.is_empty() {
                    format!("{DIM}nothing harvestable{R}")
                } else {
                    format!("{BGREEN}{mine}{R}")
                };
                println!("  {DIM}Surface        :{R} {BYELLOW}{}{R}  {DIM}[{}]{R}", planet.name, idx + 1);
                println!("  {DIM}Mining         :{R} {mine_str}");
            }
        }
        println!("  {DIM}Ship fuel      :{R} {fuel_col}{:.1}{R}{DIM} / {:.1}{R}", gs.ship.fuel, gs.ship.max_fuel);
        println!("  {DIM}Ship hull      :{R} {hull_col}{:.0}%{R}", gs.ship.hull * 100.0);
        println!("  {DIM}Proper time    :{R} {BYELLOW}{:.1} years{R}", gs.player.proper_time_s / 31_557_600.0);
        println!("  {DIM}Coord. time    :{R} {BYELLOW}{:.1} years{R}", gs.player.coordinate_time_s / 31_557_600.0);

        println!();
        let unread = gs.inbox.iter().filter(|m| !m.read).count();
        if unread > 0 {
            let from = gs.inbox.iter().rev().find(|m| !m.read).map(|m| m.from.as_str()).unwrap_or("fleet");
            println!("  {BCYAN}◆ {} unread message{} from {} — [c] Fleet comms{R}",
                unread, if unread == 1 { "" } else { "s" }, from);
            println!();
        }
        if let Some(obj) = &gs.objective {
            println!("  {BYELLOW}▶ Objective:{R} {}", obj);
        } else {
            println!("  {DIM}Objective: mine He-3 and D · refine pellets [r] · load into tank [l] · travel further{R}");
        }
        println!();
        println!("  What would you like to do?");
        if let Some(idx) = gs.player.landed_on {
            if let Some(planet) = gs.current_system.planets.get(idx) {
                let mine = mine_yield_desc(&planet.planet_type, planet.surface_temp_k);
                if !mine.is_empty() {
                    println!("  {BGREEN}[m] Mine{R}  {DIM}— {mine}{R}");
                }
                println!("  [p] Planet details  {DIM}({}){R}", planet.name);
            }
        }
        println!("  [1] Scan this star system");
        println!("  [2] Land on a planet");
        println!("  [3] Travel to a nearby system");
        println!("  [4] Open star chart");
        println!("  [5] Inspect your ship & inventory");
        println!("  [6] Periodic table reference");
        println!("  [7] Physics reference");
        println!("  [8] Consult ARIA (ship's AI)");
        let comms_label = if unread > 0 {
            format!("{BCYAN}[c] Fleet comms  (Yael · Reza)  ◆ {unread} unread{R}")
        } else {
            format!("  [c] Fleet comms  (Yael · Reza)")
        };
        println!("{}", if unread > 0 { format!("  {}", comms_label) } else { comms_label });
        println!("  [n] Send a note to your notebook");
        println!("  [s] Save game");
        println!("  [q] Quit  [?] Help  [a] ARIA");

        let choice = menu_key();
        if universal(&choice, gs, HELP_MAIN) { continue; }

        match choice.as_str() {
            "m" | "M" => {
                if let Some(idx) = gs.player.landed_on {
                    let mine = gs.current_system.planets.get(idx)
                        .map(|p| mine_yield_desc(&p.planet_type, p.surface_temp_k))
                        .unwrap_or("");
                    if mine.is_empty() {
                        println!("  Nothing harvestable here.");
                        pause();
                    } else {
                        do_mining(gs, idx);
                        pause();
                    }
                }
            }
            "p" | "P" => {
                if let Some(idx) = gs.player.landed_on {
                    inspect_planet(gs, idx);
                }
            }
            "1" => scan_system(gs),
            "2" => land_menu(gs),
            "3" => travel_menu(gs),
            "4" => star_chart(gs),
            "5" => ship_status(gs),
            "6" => periodic_table_menu(gs),
            "7" => physics_menu(gs),
            "8" => aria_chat(gs),
            "c" | "C" => comms_menu(gs),
            "n" | "N" => send_note(gs),
            "s" | "S" => save_game(gs),
            "q" | "Q" => {
                print!("\n  Save before quitting? [Y/n] ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                let confirm = read_key();
                println!("{confirm}");
                if confirm != "n" {
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
    let choice = menu_key();
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
        let risk = p.infrastructure_risk();
        let risk_col = match risk {
            universe::planet::InfraRisk::Minimal |
            universe::planet::InfraRisk::Low      => BGREEN,
            universe::planet::InfraRisk::Moderate => BYELLOW,
            universe::planet::InfraRisk::High |
            universe::planet::InfraRisk::Extreme  => BRED,
        };
        println!("  [{}] {} — {} @ {:.2} AU — {:.0} K — {}{}{}",
            i + 1, p.name, p.planet_type.display(), p.orbit_au, p.surface_temp_k,
            risk_col, risk.label(), R);
    }
    println!("  [0] Cancel  [?] Help  [a] ARIA");

    let choice = menu_key();
    if universal(&choice, gs, HELP_LAND) { continue; }
    if let Ok(n) = choice.parse::<usize>() {
        if n == 0 { break; }
        if n <= gs.current_system.planets.len() {
            let idx = n - 1;
            let risk = gs.current_system.planets[idx].infrastructure_risk();
            use universe::planet::InfraRisk;
            match risk {
                InfraRisk::Extreme => {
                    clear();
                    print_header(&format!("WARNING — {}", gs.current_system.planets[idx].name));
                    println!("  {BRED}INFRASTRUCTURE RISK: EXTREME{R}");
                    println!();
                    println!("  Conditions are hostile to digital substrate.");
                    println!("  Thermal or pressure tolerances will be exceeded on approach.");
                    println!("  {BRED}This will destroy the Perihelion I and end your journey.{R}");
                    println!();
                    print!("  Proceed anyway? [y/N] ");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    let confirm = read_key();
                    println!("{confirm}");
                    if confirm != "y" { continue; }
                }
                InfraRisk::High => {
                    clear();
                    print_header(&format!("WARNING — {}", gs.current_system.planets[idx].name));
                    println!("  {BYELLOW}INFRASTRUCTURE RISK: HIGH{R}");
                    println!();
                    println!("  Conditions will stress the Perihelion I's substrate shielding.");
                    println!("  Expect hull damage on approach. Repeated exposure is cumulative.");
                    println!();
                    print!("  Proceed? [y/N] ");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    let confirm = read_key();
                    println!("{confirm}");
                    if confirm != "y" { continue; }
                    // Apply hull damage: 10–30% hit
                    let damage = {
                        use rand::Rng;
                        rand::rng().random_range(0.10f64..0.30f64)
                    };
                    gs.ship.hull = (gs.ship.hull - damage).max(0.0);
                    let hull_col = if gs.ship.hull > 0.5 { BGREEN } else if gs.ship.hull > 0.25 { BYELLOW } else { BRED };
                    println!();
                    println!("  {BYELLOW}Shielding stressed on approach.{R}  Hull integrity: {}{:.0}%{R}", hull_col, gs.ship.hull * 100.0);
                    if gs.ship.hull == 0.0 {
                        println!();
                        println!("  {BRED}Hull integrity lost. The Perihelion I is gone.{R}");
                        println!("  {DIM}Yael and Reza receive no signal.{R}");
                        pause();
                        std::process::exit(0);
                    }
                    pause();
                }
                _ => {}
            }
            inspect_planet(gs, idx);
        }
    }
    break;
    } // end loop
}

fn inspect_planet(gs: &mut GameState, idx: usize) {
    gs.player.landed_on = Some(idx);

    // Extreme infrastructure risk — ship destroyed on approach
    {
        use universe::planet::InfraRisk;
        let planet = &gs.current_system.planets[idx];
        if planet.infrastructure_risk() == InfraRisk::Extreme {
            clear();
            print_header(&format!("LOST — {}", planet.name));
            println!("  {BRED}INFRASTRUCTURE FAILURE{R}");
            println!();
            println!("  Conditions on {} are incompatible with digital substrate.", planet.name);
            println!("  Thermal/pressure tolerances exceeded on final approach.");
            println!();
            println!("  {DIM}The Perihelion I does not respond. Yael and Reza receive no signal.{R}");
            println!("  {DIM}Your pattern dissolves into noise.{R}");
            println!();
            pause();
            std::process::exit(0);
        }
    }

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
        let risk = planet.infrastructure_risk();
        let risk_color = match risk {
            crate::universe::planet::InfraRisk::Minimal  |
            crate::universe::planet::InfraRisk::Low      => BGREEN,
            crate::universe::planet::InfraRisk::Moderate => BYELLOW,
            crate::universe::planet::InfraRisk::High     |
            crate::universe::planet::InfraRisk::Extreme  => BRED,
        };
        println!("  Infra. risk       : {}{}{R}  — {}", risk_color, risk.label(), risk.description());
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

    // Mining availability hint
    let mine_hint = mine_yield_desc(&planet.planet_type, planet.surface_temp_k);
    if !mine_hint.is_empty() {
        println!("  {BGREEN}[m] Mine{R}  {DIM}— {}{R}", mine_hint);
    }

    println!();
    println!("  [?] Help  [a] ARIA  [q] Back");
    let choice = menu_key();
    if universal(&choice, gs, HELP_PLANET) { continue; }

    if choice.to_lowercase() == "m" {
        if mine_hint.is_empty() {
            println!("  {DIM}Nothing harvestable here.{R}");
            pause();
        } else {
            do_mining(gs, idx);
        }
        continue;
    }

    break;
    } // end loop
    // landed_on is intentionally NOT cleared here — the player remains on the
    // planet surface until they travel to another system. It gets cleared in
    // travel_menu when a jump is executed.
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
    let full_cost = fuel_cost * 2.0; // accel + decel
    println!("  Fuel required     : {:.1} + {:.1} = {:.1}  (accel + decel, you have {:.1})",
        fuel_cost, fuel_cost, full_cost, gs.ship.fuel);

    if gamma > 1.001 {
        println!();
        println!("  RELATIVITY: At {:.2}c, γ = {:.4}. You age {:.4}× slower than the galaxy.", v, gamma, 1.0/gamma);
    }

    // Hard block — can't even reach the destination
    if fuel_cost > gs.ship.fuel {
        println!("\n  {BRED}Not enough fuel for this journey.{R}  ({:.1} required to reach, {:.1} available)", fuel_cost, gs.ship.fuel);
        pause();
        return;
    }

    // Drift scenario — enough to get there but not to stop
    let is_drift = full_cost > gs.ship.fuel;
    if is_drift {
        println!();
        println!("  {BYELLOW}⚠  DECELERATION WARNING{R}");
        println!("  Fuel to decelerate on arrival : {:.1}  (unavailable — only {:.1} remaining after accel)", fuel_cost, gs.ship.fuel - fuel_cost);
        println!("  You will arrive at {:.4}c with {BRED}no way to stop{R}.", v);
        println!("  Hull damage from uncontrolled entry is expected.");
        println!("  {DIM}Mine and refine more fuel before departing, or accept the drift.{R}");
        let confirm = prompt("\n  Embark anyway? [y/N] ");
        if confirm.to_lowercase() != "y" { return; }
    } else {
        let confirm = prompt("\n  Embark? [y/N] ");
        if confirm.to_lowercase() != "y" { return; }
    }

    // Derive destination display name
    let dest_name: String = if gs.galaxy.mode == GalaxyMode::RealUniverse {
        catalog::nearest_name(dest[0], dest[1], dest[2])
            .unwrap_or_else(|| format!("({:.2}, {:.2}, {:.2})", dest[0], dest[1], dest[2]))
    } else {
        format!("({:.2}, {:.2}, {:.2})", dest[0], dest[1], dest[2])
    };

    // Real-world seconds for the journey: scale with distance, cap at 2 minutes
    let real_duration_secs = (distance_ly * 10.0).clamp(20.0, 120.0);

    // Deduct actual fuel spent: full round-trip cost, or everything remaining on a drift
    gs.ship.fuel -= if is_drift { gs.ship.fuel } else { full_cost };
    gs.player.landed_on = None;
    gs.travel = Some(ActiveTravel {
        dest,
        dest_name,
        src_name: gs.current_system.name.clone(),
        distance_ly,
        v,
        gamma,
        coord_time_yr,
        proper_time_yr,
        started_at: std::time::Instant::now(),
        real_duration_secs,
        milestones_shown: 0,
        notifications: std::collections::VecDeque::new(),
        is_drift,
    });

    println!("\n  {BGREEN}Journey begun.{R} Return to the main screen to monitor progress.");
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
        let choice = menu_key();
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

/// One-line description of what can be mined from this planet type.
fn mine_yield_desc(pt: &universe::planet::PlanetType, temp_k: f64) -> &'static str {
    use universe::planet::PlanetType::*;
    match pt {
        GasGiant                                    => "He-3 atmospheric scoop",
        HotJupiter                                  => "He-3 atmospheric scoop (high radiation)",
        IceGiant                                    => "He-3 + deuterium (water ice)",
        Barren if temp_k < 200.0                    => "deuterium (water ice deposits)",
        OceanWorld                                  => "deuterium (liquid water)",
        Terrestrial if temp_k < 320.0               => "trace deuterium",
        _                                           => "",
    }
}

/// Execute a mining operation on the planet at `idx`. Randomises yield,
/// adds resources to inventory, prints a report.
fn do_mining(gs: &mut GameState, idx: usize) {
    use universe::planet::PlanetType::*;
    let planet = gs.current_system.planets[idx].clone();
    let mut rng = rand::rng();
    use rand::Rng;

    let (he3_g, d_g): (f64, f64) = match &planet.planet_type {
        GasGiant    => { fire_trigger(gs, "gas_giant_mined"); (rng.random_range(3_000.0..8_000.0), 0.0) }
        HotJupiter  => { fire_trigger(gs, "gas_giant_mined"); (rng.random_range(5_000.0..12_000.0), 0.0) }
        IceGiant    => (rng.random_range(1_000.0..3_000.0), rng.random_range(2_000.0..5_000.0)),
        Barren if planet.surface_temp_k < 200.0
                    => (0.0, rng.random_range(1_000.0..3_000.0)),
        OceanWorld  => { fire_trigger(gs, "ocean_mined"); (0.0, rng.random_range(2_000.0..5_000.0)) }
        Terrestrial if planet.surface_temp_k < 320.0
                    => (0.0, rng.random_range(300.0..1_000.0)),
        _           => (0.0, 0.0),
    };

    println!();
    println!("  {DIM}Deploying extraction systems...{R}");
    println!();

    let mut added_any = false;
    if he3_g > 0.0 {
        let added = gs.inventory.add("He-3", he3_g);
        println!("  He-3   {BGREEN}+{:.0}g{R}{}",
            added,
            if added < he3_g { format!("  {BRED}(cargo full, {:.0}g lost){R}", he3_g - added) } else { String::new() });
        added_any = true;
    }
    if d_g > 0.0 {
        let added = gs.inventory.add("D", d_g);
        println!("  D      {BGREEN}+{:.0}g{R}{}",
            added,
            if added < d_g { format!("  {BRED}(cargo full, {:.0}g lost){R}", d_g - added) } else { String::new() });
        added_any = true;
    }
    if !added_any {
        println!("  Nothing harvestable at this location.");
    }

    println!();
    let he3_have = gs.inventory.amount("He-3");
    let d_have   = gs.inventory.amount("D");
    let can_refine = (he3_have / 100.0).min(d_have / 100.0).floor() as u32;
    println!("  {DIM}Cargo  — He-3: {:.0}g  |  D: {:.0}g  |  Pellets: {}  |  Cargo used: {:.0}/{:.0}g{R}",
        he3_have, d_have, gs.inventory.pellets,
        gs.inventory.total_mass_g(), gs.inventory.capacity_g);
    if can_refine > 0 {
        println!("  {DIM}Refine {can_refine} more pellet{} — use [r] in ship status{R}",
            if can_refine == 1 { "" } else { "s" });
    }
    pause();
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
    if gs.inventory.pellets > 0 {
        println!("  {:>4}  {} pellet{}", "⬡", gs.inventory.pellets,
            if gs.inventory.pellets == 1 { "" } else { "s" });
    }

    let he3 = gs.inventory.amount("He-3");
    let d   = gs.inventory.amount("D");
    let can_refine = (he3 / 100.0).min(d / 100.0).floor() as u32;
    let can_load   = gs.inventory.pellets > 0 && gs.ship.fuel < gs.ship.max_fuel;
    println!();
    if can_refine > 0 {
        println!("  {BYELLOW}[r] Refine fuel pellets{R}  {DIM}({can_refine} pellet{} — 100g He-3 + 100g D each){R}",
            if can_refine == 1 { "" } else { "s" });
    }
    if can_load {
        let tank_space = (gs.ship.max_fuel - gs.ship.fuel).max(0.0).floor() as u32;
        let loadable = gs.inventory.pellets.min(tank_space);
        println!("  {BYELLOW}[l] Load pellets{R}  {DIM}({} pellet{} → +{loadable} fuel){R}",
            gs.inventory.pellets, if gs.inventory.pellets == 1 { "" } else { "s" });
    }

    println!();
    println!("  [?] Help  [a] ARIA  [q] Back");
    let choice = menu_key();
    if universal(&choice, gs, HELP_SHIP) { continue; }
    match choice.trim() {
        "r" if can_refine > 0 => do_refining(gs),
        "r" => { println!("  Not enough He-3 and D to refine. (Need 100g of each per pellet.)"); pause(); }
        "l" if can_load => do_loading(gs),
        "l" => { println!("  Nothing to load."); pause(); }
        _ => break,
    }
    } // end loop
}

fn do_refining(gs: &mut GameState) {
    let he3 = gs.inventory.amount("He-3");
    let d   = gs.inventory.amount("D");
    let pellets = (he3 / 100.0).min(d / 100.0).floor() as u32;
    if pellets == 0 {
        println!("  Nothing to refine. (Need 100g He-3 + 100g D per pellet.)");
        pause();
        return;
    }
    let he3_used = pellets as f64 * 100.0;
    let d_used   = pellets as f64 * 100.0;
    gs.inventory.remove("He-3", he3_used);
    gs.inventory.remove("D", d_used);
    gs.inventory.pellets += pellets;
    println!();
    println!("  {DIM}Initiating fusion pellet compaction...{R}");
    println!();
    println!("  He-3 consumed : {BRED}-{:.0}g{R}", he3_used);
    println!("  D    consumed : {BRED}-{:.0}g{R}", d_used);
    println!("  Pellets forged: {BGREEN}+{} pellet{}{R}", pellets, if pellets == 1 { "" } else { "s" });
    println!("  Pellets in cargo: {}", gs.inventory.pellets);
    println!();
    println!("  {DIM}Use [l] to load pellets into the fuel tank.{R}");
    pause();
}

fn do_loading(gs: &mut GameState) {
    if gs.inventory.pellets == 0 {
        println!("  No pellets in cargo.");
        pause();
        return;
    }
    let tank_space = (gs.ship.max_fuel - gs.ship.fuel).max(0.0).floor() as u32;
    if tank_space == 0 {
        println!("  Fuel tank is full.");
        pause();
        return;
    }
    let load = gs.inventory.pellets.min(tank_space);
    gs.inventory.pellets -= load;
    let fuel_before = gs.ship.fuel;
    gs.ship.fuel = (gs.ship.fuel + load as f64).min(gs.ship.max_fuel);
    println!();
    println!("  {DIM}Loading pellets into reactor feed...{R}");
    println!();
    println!("  Pellets loaded: {BGREEN}+{load}{R}");
    println!("  Fuel level    : {:.1} → {BGREEN}{:.1}{R} / {:.1}",
        fuel_before, gs.ship.fuel, gs.ship.max_fuel);
    if gs.inventory.pellets > 0 {
        println!("  {DIM}({} pellet{} remaining in cargo){R}",
            gs.inventory.pellets, if gs.inventory.pellets == 1 { "" } else { "s" });
    }
    pause();
}

fn periodic_table_menu(gs: &mut GameState) {
    loop {
        clear();
        print_header("PERIODIC TABLE REFERENCE");
        println!("  Symbol, atomic number, element/isotope name, or group name.");
        println!("  'all'  'groups'  'q'  [?] Help  [a] ARIA");

        let input = prompt("\n  > ");
        if universal(&input, gs, HELP_ELEMENTS) { continue; }

        match input.to_lowercase().as_str() {
            "q" => break,
            "groups" => {
                clear();
                print_header("ELEMENT GROUPS");
                let groups = [
                    ("Alkali Metal",          "alkali metal"),
                    ("Alkaline Earth Metal",  "alkaline earth metal"),
                    ("Transition Metal",      "transition metal"),
                    ("Post-Transition Metal", "post-transition metal"),
                    ("Metalloid",             "metalloid"),
                    ("Reactive Nonmetal",     "reactive nonmetal"),
                    ("Noble Gas",             "noble gas"),
                    ("Lanthanide",            "lanthanide"),
                    ("Actinide",              "actinide"),
                ];
                for (label, query) in &groups {
                    println!("  {:<24}  → type \"{}\"", label, query);
                }
                pause();
            }
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
                // Resolution order: atomic number → symbol → element name → isotope name → group
                if let Some(group) = chemistry::element::group_from_str(&input) {
                    let members = chemistry::element::elements_by_group(group);
                    clear();
                    print_header(&format!("GROUP — {:?}", group));
                    println!("  {:>4}  {:>3}  {:<16}  {:>8}  {:>8}  {:>8}  {:>8}",
                        "#", "Sym", "Name", "Mass(u)", "MP(K)", "BP(K)", "g/cm³");
                    println!("  {}", separator());
                    for e in &members {
                        println!("  {:>4}  {:>3}  {:<16}  {:>8.3}  {:>8}  {:>8}  {:>8}",
                            e.atomic_number, e.symbol, e.name, e.atomic_mass,
                            e.melting_point_k.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "—".into()),
                            e.boiling_point_k.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "—".into()),
                            e.density_g_cm3.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "—".into()));
                    }
                    pause();
                    continue;
                }

                let (element, highlight_mass) = if let Ok(n) = input.parse::<u8>() {
                    (chemistry::element::element_by_number(n), None)
                } else {
                    let sym = {
                        let mut c = input.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    };
                    if let Some(e) = chemistry::element::element_by_symbol(&sym) {
                        (Some(e), None)
                    } else if let Some(e) = chemistry::element::element_by_name(&input) {
                        (Some(e), None)
                    } else if let Some((e, iso)) = chemistry::element::isotope_by_name(&input) {
                        (Some(e), Some(iso.mass_number))
                    } else {
                        (None, None)
                    }
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
                    let isotopes = e.isotopes();
                    if !isotopes.is_empty() {
                        println!();
                        println!("  Isotopes:");
                        println!("  {:>5}  {:<16}  {:>10}  {:>16}",
                            "A", "Name", "Abundance", "Half-life");
                        println!("  {}", "─".repeat(55));
                        for iso in &isotopes {
                            let label = iso.name.unwrap_or("—");
                            let abund = iso.natural_abundance
                                .map(|a| format!("{:.4}%", a * 100.0))
                                .unwrap_or_else(|| "synthetic".into());
                            let hl = iso.half_life_display();
                            let marker = if highlight_mass == Some(iso.mass_number) { " <" } else { "" };
                            println!("  {:>5}  {:<16}  {:>10}  {:>16}{}",
                                iso.mass_number, label, abund, hl, marker);
                        }
                    }
                    pause();
                } else {
                    println!("  Not found. Try: symbol (Fe), number (26), name (Iron), isotope (Deuterium), or group (noble gas).");
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

        let choice = menu_key();
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
/// Apply a parsed game effect to the current game state.
/// Returns false if the effect couldn't be applied (e.g. insufficient resources).
fn apply_game_effect(gs: &mut GameState, effect: &GameEffect) -> bool {
    match effect {
        GameEffect::SetObjective { text } => {
            gs.objective = Some(text.clone());
            true
        }
        GameEffect::TransferResource { resource, amount_g, to_player } => {
            if *to_player {
                let space = gs.inventory.capacity_g - gs.inventory.total_mass_g();
                let actual = amount_g.min(space).max(0.0);
                if actual > 0.0 {
                    gs.inventory.add(resource, actual);
                    true
                } else {
                    false
                }
            } else {
                let have = gs.inventory.amount(resource);
                let actual = amount_g.min(have).max(0.0);
                if actual > 0.0 {
                    gs.inventory.remove(resource, actual);
                    true
                } else {
                    false
                }
            }
        }
        GameEffect::TransferFuel { amount, to_player } => {
            if *to_player {
                let space = gs.ship.max_fuel - gs.ship.fuel;
                let actual = amount.min(space).max(0.0);
                if actual > 0.0 {
                    gs.ship.fuel += actual;
                    true
                } else {
                    false
                }
            } else {
                let actual = amount.min(gs.ship.fuel).max(0.0);
                if actual > 0.0 {
                    gs.ship.fuel -= actual;
                    true
                } else {
                    false
                }
            }
        }
        GameEffect::TransferPellets { count, to_player } => {
            if *to_player {
                gs.inventory.pellets += count;
                true
            } else if gs.inventory.pellets >= *count {
                gs.inventory.pellets -= count;
                true
            } else {
                false
            }
        }
    }
}

fn build_aria_system_prompt(gs: &GameState) -> String {
    let sys = &gs.current_system;
    let star = &sys.star;
    let (hz_inner, hz_outer) = star.habitable_zone_au();

    let location = if let Some(idx) = gs.player.landed_on {
        if idx < sys.planets.len() {
            let p = &sys.planets[idx];
            let in_hz = p.is_in_habitable_zone(hz_inner, hz_outer);

            let atm_desc = if p.atmosphere.pressure_bar == 0.0 {
                "  Atmosphere  : none\n".to_string()
            } else {
                let mut comps = p.atmosphere.components.clone();
                comps.sort_by(|a, b| b.fraction.partial_cmp(&a.fraction).unwrap());
                let comp_str = comps.iter()
                    .map(|c| format!("{} {:.1}%", c.symbol, c.fraction * 100.0))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "  Atmosphere   : {:.3} bar | {} | infra. risk: {}\n",
                    p.atmosphere.pressure_bar, comp_str,
                    p.infrastructure_risk().label()
                )
            };

            format!(
                "Surveying planet {} (planet {} of {})\n\
                   Type         : {}\n\
                   Orbit        : {:.3} AU{}\n\
                   Mass         : {:.3} M⊕   Radius: {:.3} R⊕\n\
                   Surface temp : {:.0} K ({:.0}°C)\n\
                   Gravity      : {:.2}g   Escape velocity: {:.2} km/s\n\
                 {}",
                p.name,
                idx + 1, sys.planets.len(),
                p.planet_type.display(),
                p.orbit_au,
                if in_hz { " (habitable zone)" } else { "" },
                p.mass_earth, p.radius_earth,
                p.surface_temp_k, p.surface_temp_k - 273.15,
                p.surface_gravity_ms2() / 9.807,
                p.escape_velocity_ms() / 1000.0,
                atm_desc,
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
  - If a question is ambiguous, answer the most physically interesting interpretation{}",
        effect_instructions("aria"),
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

/// Build the system prompt for a companion consciousness.
/// Same situational context as ARIA, filtered through their personality.
fn build_companion_system_prompt(companion: &Companion, gs: &GameState) -> String {
    let sys = &gs.current_system;
    let star = &sys.star;
    let [px, py, pz] = gs.player.position;
    let dist_sol = (px*px + py*py + pz*pz).sqrt();
    let coord_yr = gs.player.coordinate_time_s / 31_557_600.0;
    let proper_yr = gs.player.proper_time_s / 31_557_600.0;

    let location = if let Some(idx) = gs.player.landed_on {
        if idx < sys.planets.len() {
            let p = &sys.planets[idx];
            format!("Surveying planet {} — {} class, {:.0} K, gravity {:.2}g",
                p.name, p.planet_type.display(), p.surface_temp_k, p.surface_gravity_ms2() / 9.807)
        } else {
            format!("In space within the {} system", sys.name)
        }
    } else {
        format!("In space within the {} system", sys.name)
    };

    let fuel_pct = gs.ship.fuel / gs.ship.max_fuel * 100.0;
    let he3_g    = gs.inventory.amount("He-3");
    let d_g      = gs.inventory.amount("D");
    let pellets_ready = gs.inventory.pellets;

    format!(
        "{personality}\n\n\
Vary your response length naturally. Sometimes you have a lot to say — when something \
genuinely moves or unsettles you, when a question opens something up. Sometimes one \
sentence or even a fragment is exactly right. You are not obligated to be thorough. \
Do not pad. Do not summarise at the end. Just say what you actually have to say.\n\n\
SHARED SITUATION — all three ships are at the same position:\n\
  Star system   : {name} | {cls} | {temp:.0} K | {lum:.4} L☉\n\
  Distance Sol  : {dist:.4} ly\n\
  Location      : {loc}\n\
  Proper time   : {prop:.2} yr elapsed (your subjective frame)\n\
  Coord. time   : {coord:.2} yr elapsed (galaxy frame)\n\n\
PERIHELION I STATUS (the player's ship):\n\
  Fuel          : {fuel:.1} / {max_fuel:.1} ({fuel_pct:.0}%)\n\
  Hull          : {hull:.0}%\n\
  He-3 cargo    : {he3:.0} g\n\
  Deuterium     : {d:.0} g\n\
  Pellets (cargo): {pellets}\n\n\
The player's ship is Perihelion I. Your ship is {ship}. \
The third ship is {other_ship}, crewed by {other_name}.{effects}",
        personality  = companion.personality,
        name     = sys.name,
        cls      = star.spectral_class.display(),
        temp     = star.temperature_k,
        lum      = star.luminosity,
        dist     = dist_sol,
        loc      = location,
        prop     = proper_yr,
        coord    = coord_yr,
        fuel     = gs.ship.fuel,
        max_fuel = gs.ship.max_fuel,
        fuel_pct = fuel_pct,
        hull     = gs.ship.hull * 100.0,
        he3      = he3_g,
        d        = d_g,
        pellets  = pellets_ready,
        ship   = companion.ship_name,
        other_ship = if companion.ship_name == "Threshold" { "Sable" } else { "Threshold" },
        other_name = if companion.ship_name == "Threshold" { "Reza Terani" } else { "Dr. Yael Orin" },
        effects    = effect_instructions("companion"),
    )
}

fn group_chat(gs: &mut GameState) {
    clear();
    print_header("GROUP CHANNEL — Perihelion I · Threshold · Sable");

    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("  Comms offline — no API key.");
        pause();
        return;
    }

    println!("  {DIM}All three ships on channel. 'q' to close, 'status' for ship readout, 'log' for history.{R}");
    println!();

    loop {
        let input = prompt(&format!("  {BCYAN}You       {R} {DIM}>{R} "));
        if input.is_empty() { continue; }

        match input.to_lowercase().trim() {
            "q" | "quit" | "close" | "disconnect" => break,
            "status" | "/status" => {
                let fuel_pct = gs.ship.fuel / gs.ship.max_fuel * 100.0;
                let he3 = gs.inventory.amount("He-3");
                let d   = gs.inventory.amount("D");
                println!("  {DIM}── Perihelion I ──────────────────────────────────{R}");
                println!("  {DIM}Fuel  {R}{:.1} / {:.1}  ({:.0}%)  │  Hull  {:.0}%",
                    gs.ship.fuel, gs.ship.max_fuel, fuel_pct, gs.ship.hull * 100.0);
                println!("  {DIM}He-3  {R}{:.0} g  │  D  {:.0} g", he3, d);
                println!("  {DIM}─────────────────────────────────────────────────{R}");
                println!();
                continue;
            }
            "log" | "/log" => {
                println!("  {DIM}(Individual logs — use [1] or [2] in the comms menu to view per-companion history.){R}");
                println!();
                continue;
            }
            _ => {}
        }

        // Both companions respond in turn
        for idx in 0..gs.companions.len() {
            let name  = gs.companions[idx].name;
            let system_prompt = build_companion_system_prompt(&gs.companions[idx], gs);

            print!("  {BMAGENTA}{name:<10}{R} {DIM}>{R}\n");
            {
                use std::io::{stdout, Write};
                stdout().flush().ok();
            }

            let start_row = crossterm::cursor::position().ok().map(|(_, r)| r);

            let result = {
                use std::io::{stdout, Write};
                gs.companions[idx].computer.ask_streaming(&input, &system_prompt, |chunk| {
                    for ch in chunk.chars() {
                        print!("{}", ch);
                        stdout().flush().ok();
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                })
            };

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
                    let (clean_text, effects) = parse_effects(&full_text);
                    use termimad::crossterm::style::Color as TC;
                    let mut skin = termimad::MadSkin::default();
                    skin.bold.set_fg(TC::Yellow);
                    skin.italic.set_fg(TC::Magenta);
                    for h in &mut skin.headers { h.set_fg(TC::Cyan); }
                    skin.inline_code.set_fg(TC::Green);
                    skin.print_text(&clean_text);
                    for effect in &effects {
                        let companion_name = gs.companions[idx].name;
                        if apply_game_effect(gs, effect) {
                            println!("  {BGREEN}▶ {}{R}", describe_effect(effect, companion_name));
                        }
                    }
                }
                Err(e) => {
                    println!("  {BRED}[comms error: {}]{R}", e);
                }
            }
            println!();
        }
    }
}

fn comms_menu(gs: &mut GameState) {
    loop {
        clear();
        print_header("FLEET COMMS");

        println!("  Fleet position : {}  ({:.2}, {:.2}, {:.2}) ly",
            gs.current_system.name,
            gs.player.position[0], gs.player.position[1], gs.player.position[2]);
        println!("  Coord. time    : {:.2} yr elapsed", gs.player.coordinate_time_s / 31_557_600.0);
        println!();

        let unread = gs.inbox.iter().filter(|m| !m.read).count();
        if unread > 0 {
            println!("  {BCYAN}◆ {unread} unread message{}{R}", if unread == 1 { "" } else { "s" });
            println!();
        }

        println!("  Hail which ship?");
        println!();
        for (i, c) in gs.companions.iter().enumerate() {
            println!("  [{}] {} — {}  ({})", i + 1, c.name, c.ship_name, c.specialty);
        }
        println!();
        if !gs.inbox.is_empty() {
            println!("  [m] Messages  {DIM}({} total, {unread} unread){R}", gs.inbox.len());
        }
        println!("  [g] Group channel  {DIM}(all three ships){R}");
        println!("  [q] Close channel");

        let choice = menu_key();
        match choice.trim() {
            "q" | "" => return,
            "m" | "M" => inbox_menu(gs),
            "g" | "G" => group_chat(gs),
            s => {
                if let Ok(n) = s.parse::<usize>() {
                    if n >= 1 && n <= gs.companions.len() {
                        companion_chat(gs, n - 1);
                    }
                }
            }
        }
    }
}

fn inbox_menu(gs: &mut GameState) {
    loop {
        clear();
        print_header("MESSAGES");

        if gs.inbox.is_empty() {
            println!("  No messages.");
            pause();
            return;
        }

        for (i, msg) in gs.inbox.iter().enumerate() {
            let dot = if msg.read { "   ".to_string() } else { format!(" {BCYAN}◆{R}") };
            println!("  {dot} [{}] {:<20}  {}", i + 1, msg.from, msg.subject);
        }
        println!();
        println!("  Enter a message number to read, or [q] to go back.");

        let choice = prompt("\n  > ");
        match choice.trim() {
            "q" | "" => return,
            s => {
                if let Ok(n) = s.parse::<usize>() {
                    if n >= 1 && n <= gs.inbox.len() {
                        let idx = n - 1;
                        gs.inbox[idx].read = true;
                        let from    = gs.inbox[idx].from.clone();
                        let subject = gs.inbox[idx].subject.clone();
                        let body    = gs.inbox[idx].body.clone();
                        clear();
                        print_header(&format!("MESSAGE — {}", from));
                        println!("  Subject : {}", subject);
                        println!("  From    : {}", from);
                        println!("  {}", "─".repeat(55));
                        println!();
                        for line in body.lines() {
                            println!("  {}", line);
                        }
                        println!();
                        pause();
                    }
                }
            }
        }
    }
}

fn companion_chat(gs: &mut GameState, idx: usize) {
    let name  = gs.companions[idx].name;
    let ship  = gs.companions[idx].ship_name;

    clear();
    print_header(&format!("COMMS — {} / {}", name, ship));

    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("  Comms offline — no API key.");
        pause();
        return;
    }

    println!("  {DIM}Hailing {}...{R}", name);
    println!("  {DIM}Channel open. 'q' to close, 'clear' to reset.{R}");
    println!();

    loop {
        let input = prompt(&format!("  {BCYAN}You  {R} {DIM}>{R} "));

        if input.is_empty() { continue; }

        match input.to_lowercase().trim() {
            "q" | "quit" | "close" | "disconnect" => break,
            "clear" => {
                gs.companions[idx].computer.clear_history();
                println!("  {DIM}[Channel cleared]{R}");
                println!();
                continue;
            }
            "status" | "/status" => {
                let fuel_pct = gs.ship.fuel / gs.ship.max_fuel * 100.0;
                let he3 = gs.inventory.amount("He-3");
                let d   = gs.inventory.amount("D");
                println!("  {DIM}── Perihelion I ──────────────────────────────────{R}");
                println!("  {DIM}Fuel  {R}{:.1} / {:.1}  ({:.0}%)  │  Hull  {:.0}%",
                    gs.ship.fuel, gs.ship.max_fuel, fuel_pct, gs.ship.hull * 100.0);
                println!("  {DIM}He-3  {R}{:.0} g  │  D  {:.0} g", he3, d);
                println!("  {DIM}─────────────────────────────────────────────────{R}");
                println!();
                continue;
            }
            "log" | "/log" => {
                let log = gs.companions[idx].computer.recent_log(10).to_vec();
                if log.is_empty() {
                    println!("  {DIM}No history yet.{R}");
                } else {
                    println!("  {DIM}── Recent history ────────────────────────────────{R}");
                    for msg in &log {
                        let label = if msg.role == "user" {
                            format!("{BCYAN}You{R}")
                        } else {
                            format!("{BMAGENTA}{}{R}", name)
                        };
                        // Print first ~120 chars of each message
                        let preview: String = msg.content.chars().take(120).collect();
                        let ellipsis = if msg.content.len() > 120 { "…" } else { "" };
                        println!("  {} {DIM}>{R} {}{}", label, preview, ellipsis);
                    }
                    println!("  {DIM}─────────────────────────────────────────────────{R}");
                }
                println!();
                continue;
            }
            _ => {}
        }

        let system_prompt = build_companion_system_prompt(&gs.companions[idx], gs);

        print!("  {BMAGENTA}{name}{R} {DIM}>{R}\n");
        {
            use std::io::{stdout, Write};
            stdout().flush().ok();
        }

        let start_row = crossterm::cursor::position().ok().map(|(_, r)| r);

        let result = {
            use std::io::{stdout, Write};
            gs.companions[idx].computer.ask_streaming(&input, &system_prompt, |chunk| {
                for ch in chunk.chars() {
                    print!("{}", ch);
                    stdout().flush().ok();
                    std::thread::sleep(std::time::Duration::from_millis(12));
                }
            })
        };

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
                let (clean_text, effects) = parse_effects(&full_text);
                use termimad::crossterm::style::Color as TC;
                let mut skin = termimad::MadSkin::default();
                skin.bold.set_fg(TC::Yellow);
                skin.italic.set_fg(TC::Magenta);
                for h in &mut skin.headers { h.set_fg(TC::Cyan); }
                skin.inline_code.set_fg(TC::Green);
                skin.print_text(&clean_text);
                for effect in &effects {
                    if apply_game_effect(gs, effect) {
                        println!("  {BGREEN}▶ {}{R}", describe_effect(effect, name));
                    }
                }
            }
            Err(e) => {
                println!("  {BRED}[comms error: {}]{R}", e);
            }
        }
        println!();
    }
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
                let (clean_text, effects) = parse_effects(&full_text);
                use termimad::crossterm::style::Color as TC;
                let mut skin = termimad::MadSkin::default();
                skin.bold.set_fg(TC::Yellow);
                skin.italic.set_fg(TC::Magenta);
                for h in &mut skin.headers {
                    h.set_fg(TC::Cyan);
                }
                skin.inline_code.set_fg(TC::Green);
                skin.print_text(&clean_text);
                for effect in &effects {
                    if apply_game_effect(gs, effect) {
                        println!("  {BGREEN}▶ {}{R}", describe_effect(effect, "ARIA"));
                    }
                }
            }
            Err(e) => {
                println!("  {BRED}[offline: {}]{R}", e);
            }
        }
        println!();
    }
}
