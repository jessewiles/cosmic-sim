mod universe;
mod physics;
mod chemistry;
mod world;
mod player;
mod ui;

use universe::galaxy::Galaxy;
use universe::system::StarSystem;
use universe::planet::Planet;
use player::state::PlayerState;
use player::ship::Ship;
use player::inventory::Inventory;
use physics::relativity::{lorentz_factor, schwarzschild_radius};
use physics::constants::{C, M_SUN};
use chemistry::element::periodic_table;
use ui::terminal::*;
use ui::display::separator;

struct GameState {
    player: PlayerState,
    ship: Ship,
    inventory: Inventory,
    galaxy: Galaxy,
    current_system: StarSystem,
}

impl GameState {
    fn new(player_name: String) -> Self {
        let mut galaxy = Galaxy::new("The Milky Way".to_string(), 0xDEADBEEF_C0FFEE);
        let current_system = galaxy.system_at(0, 0, 0);
        GameState {
            player: PlayerState::new(player_name),
            ship: Ship::starter(),
            inventory: Inventory::new(10_000.0),
            galaxy,
            current_system,
        }
    }
}

fn main() {
    clear();
    print_header("C O S M O S  —  An Explorer's Guide to the Universe");

    println!("  Welcome. You are about to embark on a journey through the cosmos.");
    println!("  Every star, planet, and atom you encounter obeys real physics.");
    println!("  Learn. Discover. Survive.");
    println!();

    let name = prompt("  Enter your name, explorer: ");
    if name.is_empty() {
        println!("  Coward. Goodbye.");
        return;
    }

    let mut gs = GameState::new(name.clone());

    println!();
    println!("  Greetings, {}. Your ship — {} — is fuelled and ready.", name, gs.ship.name);
    println!("  You begin in the {} system.", gs.current_system.name);
    pause();

    game_loop(&mut gs);
}

fn game_loop(gs: &mut GameState) {
    loop {
        clear();
        print_header(&format!("COSMOS  |  {}  |  {:?}", gs.player.name, gs.player.position));

        println!("  Current system : {}", gs.current_system.name);
        println!("  Star           : {} — {}", gs.current_system.star.name, gs.current_system.star.spectral_class.display());
        println!("  Planets        : {}", gs.current_system.planets.len());
        println!("  Ship fuel      : {:.1} / {:.1}", gs.ship.fuel, gs.ship.max_fuel);
        println!("  Ship hull      : {:.0}%", gs.ship.hull * 100.0);
        println!("  Proper time    : {:.1} years", gs.player.proper_time_s / 31_557_600.0);
        println!("  Coord. time    : {:.1} years", gs.player.coordinate_time_s / 31_557_600.0);

        println!();
        println!("  What would you like to do?");
        println!("  [1] Scan this star system");
        println!("  [2] Land on a planet");
        println!("  [3] Travel to a nearby system");
        println!("  [4] Open star chart");
        println!("  [5] Inspect your ship & inventory");
        println!("  [6] Periodic table reference");
        println!("  [7] Physics reference");
        println!("  [q] Quit");

        let choice = prompt("\n  > ");

        match choice.as_str() {
            "1" => scan_system(gs),
            "2" => land_menu(gs),
            "3" => travel_menu(gs),
            "4" => star_chart(gs),
            "5" => ship_status(gs),
            "6" => periodic_table_menu(),
            "7" => physics_menu(gs),
            "q" | "Q" => {
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

    pause();
}

fn land_menu(gs: &mut GameState) {
    if gs.current_system.planets.is_empty() {
        clear();
        println!("\n  No planets in this system to land on.");
        pause();
        return;
    }

    clear();
    print_header("LAND ON PLANET");
    for (i, p) in gs.current_system.planets.iter().enumerate() {
        println!("  [{}] {} — {} @ {:.2} AU — {:.0} K", i + 1, p.name, p.planet_type.display(), p.orbit_au, p.surface_temp_k);
    }
    println!("  [0] Cancel");

    let choice = prompt("\n  > ");
    if let Ok(n) = choice.parse::<usize>() {
        if n > 0 && n <= gs.current_system.planets.len() {
            inspect_planet(gs, n - 1);
        }
    }
}

fn inspect_planet(gs: &mut GameState, idx: usize) {
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

    pause();
}

fn travel_menu(gs: &mut GameState) {
    clear();
    print_header("INTERSTELLAR TRAVEL");

    println!("  Current position : {:?}", gs.player.position);
    println!("  Ship max velocity: {:.2}c", gs.ship.max_velocity_c);
    println!();
    println!("  Enter destination coordinates (x y z in light-years, e.g. \"1 0 0\"):");
    println!("  Or press Enter to cancel.");

    let input = prompt("\n  > ");
    if input.is_empty() { return; }

    let parts: Vec<i32> = input.split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();

    if parts.len() != 3 {
        println!("  Invalid coordinates.");
        pause();
        return;
    }

    let dest = [parts[0], parts[1], parts[2]];
    let dx = (dest[0] - gs.player.position[0]) as f64;
    let dy = (dest[1] - gs.player.position[1]) as f64;
    let dz = (dest[2] - gs.player.position[2]) as f64;
    let distance_ly = (dx*dx + dy*dy + dz*dz).sqrt();

    if distance_ly == 0.0 {
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
    println!("  Distance          : {:.2} light-years", distance_ly);
    println!("  Travel velocity   : {:.4}c", v);
    println!("  Lorentz factor γ  : {:.4}  (time dilation factor)", gamma);
    println!("  Coord. time       : {:.2} years  (time in the galaxy's frame)", coord_time_yr);
    println!("  Proper time       : {:.2} years  (time YOU experience)", proper_time_yr);
    println!("  Fuel required     : {:.1}  (you have {:.1})", fuel_cost, gs.ship.fuel);

    if gamma > 1.001 {
        println!();
        println!("  RELATIVITY: At {:.2}c, γ = {:.4}. You age {:.2}× slower than the galaxy.", v, gamma, gamma);
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
    gs.player.visited_systems.push(dest);
    gs.current_system = gs.galaxy.system_at(dest[0], dest[1], dest[2]);

    println!("\n  Arrived at {}.", gs.current_system.name);
    pause();
}

fn star_chart(gs: &mut GameState) {
    clear();
    print_header("STAR CHART — Known Systems");

    if gs.player.visited_systems.is_empty() {
        println!("  No systems visited yet.");
    } else {
        println!("  {:>20}  {:>6}  {:>6}  {:>6}  {}", "Name", "X", "Y", "Z", "Star Class");
        println!("  {}", separator());
        for coords in &gs.player.visited_systems {
            if let Some(sys) = gs.galaxy.known_systems.iter().find(|s| {
                s.galactic_x as i32 == coords[0]
                    && s.galactic_y as i32 == coords[1]
                    && s.galactic_z as i32 == coords[2]
            }) {
                println!("  {:>20}  {:>6}  {:>6}  {:>6}  {}",
                    sys.name, coords[0], coords[1], coords[2],
                    sys.star.spectral_class.display().split(' ').next().unwrap_or("?"));
            }
        }
    }
    pause();
}

fn ship_status(gs: &mut GameState) {
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
    pause();
}

fn periodic_table_menu() {
    loop {
        clear();
        print_header("PERIODIC TABLE REFERENCE");
        println!("  Enter an element symbol (e.g. \"Fe\", \"Au\", \"H\") or atomic number (e.g. \"26\"),");
        println!("  'all' to list all elements, or 'q' to return.");

        let input = prompt("\n  > ");

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

fn physics_menu(gs: &GameState) {
    loop {
        clear();
        print_header("PHYSICS REFERENCE");
        println!("  [1] Time dilation calculator");
        println!("  [2] Schwarzschild radius calculator");
        println!("  [3] Your relativistic status");
        println!("  [q] Back");

        let choice = prompt("\n  > ");
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
