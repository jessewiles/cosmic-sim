/// Real star catalog derived from HYG / Hipparcos / Gliese data.
/// Coordinates are in light-years relative to Sol, computed from
/// equatorial RA/Dec: x = d*cos(dec)*cos(ra), y = d*cos(dec)*sin(ra), z = d*sin(dec)

use crate::universe::planet::{Planet, PlanetType};
use crate::chemistry::atmosphere::{Atmosphere, AtmosphericComponent};

// ── Data types ──────────────────────────────────────────────────────────────

pub struct CatalogStar {
    pub name:        &'static str,
    pub x_ly:        f64,
    pub y_ly:        f64,
    pub z_ly:        f64,
    /// Spectral type string, e.g. "G2V", "M5.5Ve", "DA2"
    pub spectral:    &'static str,
    pub temp_k:      f64,
    /// Luminosity in solar units
    pub luminosity:  f64,
    /// Mass in solar masses
    pub mass:        f64,
    /// Radius in solar radii
    pub radius:      f64,
    pub age_gyr:     f64,
    /// Short fact shown in the UI
    pub notes:       &'static str,
}

impl CatalogStar {
    pub fn dist_ly(&self) -> f64 {
        (self.x_ly * self.x_ly + self.y_ly * self.y_ly + self.z_ly * self.z_ly).sqrt()
    }
}

pub struct CatalogPlanet {
    pub star_name:    &'static str,
    pub name:         &'static str,
    pub orbit_au:     f64,
    pub mass_earth:   f64,
    pub radius_earth: f64,
    pub period_days:  f64,
    pub temp_k:       f64,
    pub planet_type:  PlanetType,
    pub moon_count:   u8,
    pub notes:        &'static str,
}

// ── Lookup helpers ───────────────────────────────────────────────────────────

pub fn find_by_name(name: &str) -> Option<&'static CatalogStar> {
    let lower = name.to_lowercase();
    CATALOG.iter().find(|e| e.name.to_lowercase() == lower)
}

/// Nearest catalog star within `max_ly`.  Returns the entry and exact distance.
pub fn nearest_within(x: f64, y: f64, z: f64, max_ly: f64) -> Option<(&'static CatalogStar, f64)> {
    let mut best: Option<(&'static CatalogStar, f64)> = None;
    for e in CATALOG {
        let d = dist3(e.x_ly, e.y_ly, e.z_ly, x, y, z);
        if d <= max_ly && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((e, d));
        }
    }
    best
}

/// Name of the nearest catalog star within 0.5 ly of the given coords, if any.
pub fn nearest_name(x: f64, y: f64, z: f64) -> Option<String> {
    nearest_within(x, y, z, 0.5).map(|(e, _)| e.name.to_string())
}

/// All catalog stars within `radius` ly, sorted nearest-first.
pub fn stars_within(x: f64, y: f64, z: f64, radius: f64) -> Vec<(&'static CatalogStar, f64)> {
    let mut v: Vec<_> = CATALOG.iter()
        .filter_map(|e| {
            let d = dist3(e.x_ly, e.y_ly, e.z_ly, x, y, z);
            if d <= radius { Some((e, d)) } else { None }
        })
        .collect();
    v.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    v
}

fn dist3(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> f64 {
    let dx = ax - bx; let dy = ay - by; let dz = az - bz;
    (dx*dx + dy*dy + dz*dz).sqrt()
}

// ── Planet builders ──────────────────────────────────────────────────────────

fn comp(symbol: &str, name: &str, fraction: f64) -> AtmosphericComponent {
    AtmosphericComponent { symbol: symbol.to_string(), name: name.to_string(), fraction }
}

fn atm(pressure_bar: f64, components: Vec<AtmosphericComponent>) -> Atmosphere {
    Atmosphere { pressure_bar, components }
}

/// Build Planet objects for a named star from the PLANETS catalog.
/// Returns empty Vec if no known planets.
pub fn build_known_planets(star_name: &str) -> Vec<Planet> {
    PLANETS.iter()
        .filter(|p| p.star_name == star_name)
        .map(|cp| Planet {
            name:          cp.name.to_string(),
            planet_type:   cp.planet_type.clone(),
            orbit_au:      cp.orbit_au,
            mass_earth:    cp.mass_earth,
            radius_earth:  cp.radius_earth,
            surface_temp_k: cp.temp_k,
            atmosphere:    catalog_atmosphere(cp),
            has_moons:     cp.moon_count > 0,
            moon_count:    cp.moon_count,
        })
        .collect()
}

fn catalog_atmosphere(cp: &CatalogPlanet) -> Atmosphere {
    match cp.name {
        "Mercury" => Atmosphere::none(),
        "Venus"   => atm(92.0, vec![comp("CO₂","Carbon dioxide",0.965), comp("N₂","Nitrogen",0.035)]),
        "Earth"   => atm(1.013, vec![
            comp("N₂",  "Nitrogen",       0.7808),
            comp("O₂",  "Oxygen",          0.2095),
            comp("Ar",  "Argon",            0.0093),
            comp("CO₂", "Carbon dioxide",  0.0004),
        ]),
        "Mars"    => atm(0.006, vec![
            comp("CO₂","Carbon dioxide",0.9532),
            comp("N₂", "Nitrogen",      0.0270),
            comp("Ar", "Argon",         0.0160),
        ]),
        "Jupiter" => atm(1000.0, vec![
            comp("H₂",  "Molecular hydrogen", 0.890),
            comp("He",  "Helium",              0.102),
            comp("CH₄", "Methane",             0.003),
        ]),
        "Saturn"  => atm(1000.0, vec![
            comp("H₂",  "Molecular hydrogen", 0.963),
            comp("He",  "Helium",              0.033),
            comp("CH₄", "Methane",             0.004),
        ]),
        "Uranus"  => atm(100.0, vec![
            comp("H₂",  "Molecular hydrogen", 0.830),
            comp("He",  "Helium",              0.150),
            comp("CH₄", "Methane",             0.022),
        ]),
        "Neptune" => atm(100.0, vec![
            comp("H₂",  "Molecular hydrogen", 0.800),
            comp("He",  "Helium",              0.190),
            comp("CH₄", "Methane",             0.015),
        ]),
        _ => match cp.planet_type {
            PlanetType::GasGiant | PlanetType::HotJupiter => atm(1000.0, vec![
                comp("H₂",  "Molecular hydrogen", 0.89),
                comp("He",  "Helium",              0.10),
                comp("CH₄", "Methane",             0.01),
            ]),
            PlanetType::IceGiant => atm(100.0, vec![
                comp("H₂",  "Molecular hydrogen", 0.83),
                comp("He",  "Helium",              0.15),
                comp("CH₄", "Methane",             0.02),
            ]),
            PlanetType::Terrestrial | PlanetType::SuperEarth | PlanetType::OceanWorld => {
                if cp.temp_k > 200.0 && cp.temp_k < 350.0 {
                    atm(1.0, vec![
                        comp("N₂",  "Nitrogen",       0.77),
                        comp("O₂",  "Oxygen",          0.21),
                        comp("CO₂", "Carbon dioxide",  0.01),
                        comp("Ar",  "Argon",            0.01),
                    ])
                } else if cp.temp_k >= 350.0 {
                    atm(10.0, vec![comp("CO₂","Carbon dioxide",0.96), comp("N₂","Nitrogen",0.04)])
                } else {
                    atm(0.3, vec![comp("CO₂","Carbon dioxide",0.95), comp("N₂","Nitrogen",0.05)])
                }
            }
            PlanetType::Barren => Atmosphere::none(),
        },
    }
}

// ── The real-star catalog ────────────────────────────────────────────────────
// Positions (x,y,z) in light-years from Sol.
// Formula: x = d·cos(dec)·cos(ra), y = d·cos(dec)·sin(ra), z = d·sin(dec)

pub static CATALOG: &[CatalogStar] = &[
    // ── Nearest stars ────────────────────────────────────────────────────────
    CatalogStar { name: "Sol",
        x_ly: 0.0, y_ly: 0.0, z_ly: 0.0,
        spectral: "G2V", temp_k: 5778.0, luminosity: 1.0, mass: 1.0, radius: 1.0, age_gyr: 4.60,
        notes: "Home star. 8 planets, 1 confirmed habitable." },

    CatalogStar { name: "Proxima Centauri",
        x_ly: -1.532, y_ly: -1.171, z_ly: -3.771,
        spectral: "M5.5Ve", temp_k: 3042.0, luminosity: 0.00155, mass: 0.1221, radius: 0.1542, age_gyr: 4.85,
        notes: "Closest star. Has Proxima b — a rocky planet in the habitable zone." },

    CatalogStar { name: "Alpha Centauri A",
        x_ly: -1.630, y_ly: -1.363, z_ly: -3.812,
        spectral: "G2V", temp_k: 5790.0, luminosity: 1.519, mass: 1.100, radius: 1.227, age_gyr: 5.30,
        notes: "Sun-like, part of triple system with Proxima. Best candidate for Earth-like planets nearby." },

    CatalogStar { name: "Alpha Centauri B",
        x_ly: -1.631, y_ly: -1.364, z_ly: -3.814,
        spectral: "K1V", temp_k: 5260.0, luminosity: 0.500, mass: 0.907, radius: 0.865, age_gyr: 5.30,
        notes: "Orange companion to Alpha Cen A. Orbits at ~23 AU from A." },

    CatalogStar { name: "Barnard's Star",
        x_ly: -0.057, y_ly: -5.942, z_ly: 0.488,
        spectral: "M4Ve", temp_k: 3134.0, luminosity: 0.00350, mass: 0.144, radius: 0.196, age_gyr: 10.00,
        notes: "Fastest-moving star in the sky. Among the oldest in the galaxy." },

    CatalogStar { name: "Wolf 359",
        x_ly: -7.426, y_ly: 2.128, z_ly: 0.950,
        spectral: "M6Ve", temp_k: 2800.0, luminosity: 0.000090, mass: 0.109, radius: 0.144, age_gyr: 0.35,
        notes: "Faint red dwarf. One of the lowest-luminosity stars known." },

    CatalogStar { name: "Lalande 21185",
        x_ly: -6.521, y_ly: 1.627, z_ly: 4.878,
        spectral: "M2V", temp_k: 3360.0, luminosity: 0.00522, mass: 0.386, radius: 0.393, age_gyr: 7.50,
        notes: "4th nearest star system. May host a super-Earth." },

    CatalogStar { name: "Sirius A",
        x_ly: -1.613, y_ly: 8.079, z_ly: -2.475,
        spectral: "A1V", temp_k: 9940.0, luminosity: 25.40, mass: 2.063, radius: 1.711, age_gyr: 0.242,
        notes: "Brightest star in Earth's night sky. Part of binary with Sirius B (white dwarf)." },

    CatalogStar { name: "Sirius B",
        x_ly: -1.615, y_ly: 8.081, z_ly: -2.477,
        spectral: "DA2", temp_k: 25200.0, luminosity: 0.0256, mass: 1.018, radius: 0.0084, age_gyr: 0.242,
        notes: "White dwarf companion of Sirius A. Earth-sized, but with the mass of the Sun." },

    CatalogStar { name: "Luyten 726-8 A",
        x_ly: 7.537, y_ly: 3.481, z_ly: -2.691,
        spectral: "M5.5Ve", temp_k: 2670.0, luminosity: 0.000060, mass: 0.102, radius: 0.140, age_gyr: 0.20,
        notes: "Flare star in UV Ceti system. Dramatic X-ray bursts." },

    CatalogStar { name: "Ross 154",
        x_ly: 1.914, y_ly: -8.637, z_ly: -3.911,
        spectral: "M3.5Ve", temp_k: 3300.0, luminosity: 0.00384, mass: 0.178, radius: 0.240, age_gyr: 1.00,
        notes: "Active flare star. Young by stellar standards." },

    CatalogStar { name: "Ross 248",
        x_ly: 7.353, y_ly: -0.650, z_ly: 7.181,
        spectral: "M5.5Ve", temp_k: 2799.0, luminosity: 0.000181, mass: 0.136, radius: 0.160, age_gyr: 0.00,
        notes: "Voyager 2 will pass within 1.7 ly of this star in ~40,000 years." },

    CatalogStar { name: "Epsilon Eridani",
        x_ly: 6.186, y_ly: 8.273, z_ly: -1.722,
        spectral: "K2V", temp_k: 5084.0, luminosity: 0.340, mass: 0.830, radius: 0.735, age_gyr: 0.70,
        notes: "Young, active K-dwarf. Has a confirmed super-Jupiter (Epsilon Eridani b) and debris disk." },

    CatalogStar { name: "Lacaille 9352",
        x_ly: 8.477, y_ly: -2.009, z_ly: -6.287,
        spectral: "M1.5Ve", temp_k: 3626.0, luminosity: 0.0284, mass: 0.503, radius: 0.476, age_gyr: 6.20,
        notes: "Third brightest red dwarf in the sky." },

    CatalogStar { name: "Ross 128",
        x_ly: -10.874, y_ly: 0.571, z_ly: 0.156,
        spectral: "M4Vn", temp_k: 3192.0, luminosity: 0.00362, mass: 0.168, radius: 0.197, age_gyr: 9.40,
        notes: "Has Ross 128 b — a temperate rocky planet likely in the habitable zone." },

    CatalogStar { name: "Procyon A",
        x_ly: -4.840, y_ly: 10.336, z_ly: 1.043,
        spectral: "F5IV-V", temp_k: 6530.0, luminosity: 6.930, mass: 1.499, radius: 2.048, age_gyr: 1.87,
        notes: "7th brightest star. Its white dwarf companion (Procyon B) was the first predicted before discovery." },

    CatalogStar { name: "Procyon B",
        x_ly: -4.842, y_ly: 10.338, z_ly: 1.044,
        spectral: "DQZ", temp_k: 7740.0, luminosity: 0.000549, mass: 0.602, radius: 0.0096, age_gyr: 1.87,
        notes: "White dwarf companion to Procyon A. Predicted by Bessel in 1844 from proper motion anomaly." },

    CatalogStar { name: "61 Cygni A",
        x_ly: 6.502, y_ly: -6.082, z_ly: 7.134,
        spectral: "K5V", temp_k: 4526.0, luminosity: 0.0853, mass: 0.708, radius: 0.665, age_gyr: 6.10,
        notes: "First star to have its parallax measured (Bessel, 1838). Binary with 61 Cygni B." },

    CatalogStar { name: "61 Cygni B",
        x_ly: 6.501, y_ly: -6.083, z_ly: 7.133,
        spectral: "K7V", temp_k: 4077.0, luminosity: 0.0410, mass: 0.663, radius: 0.628, age_gyr: 6.10,
        notes: "K-dwarf companion to 61 Cygni A. Historical star in measuring stellar parallax." },

    CatalogStar { name: "Tau Ceti",
        x_ly: 10.293, y_ly: 5.021, z_ly: -3.267,
        spectral: "G8.5V", temp_k: 5344.0, luminosity: 0.488, mass: 0.783, radius: 0.793, age_gyr: 5.80,
        notes: "Has 4+ confirmed planets. Tau Ceti e and f may be in or near the habitable zone." },

    CatalogStar { name: "Luyten's Star",
        x_ly: -4.597, y_ly: 11.434, z_ly: 1.126,
        spectral: "M3.5Vh", temp_k: 3382.0, luminosity: 0.00293, mass: 0.290, radius: 0.294, age_gyr: 3.50,
        notes: "Has Luyten b — a super-Earth with mass 2.89 ME in the habitable zone." },

    CatalogStar { name: "Teegarden's Star",
        x_ly: 8.696, y_ly: 8.224, z_ly: 3.629,
        spectral: "M7V", temp_k: 2904.0, luminosity: 0.000730, mass: 0.089, radius: 0.107, age_gyr: 8.00,
        notes: "Has Teegarden b and c — two Earth-mass planets in the habitable zone." },

    CatalogStar { name: "Kapteyn's Star",
        x_ly: 1.890, y_ly: 8.815, z_ly: -9.025,
        spectral: "M1.5V", temp_k: 3570.0, luminosity: 0.00384, mass: 0.281, radius: 0.291, age_gyr: 11.50,
        notes: "Halo star, extremely old. Has Kapteyn b in the habitable zone." },

    CatalogStar { name: "GJ 1061",
        x_ly: 5.029, y_ly: 6.926, z_ly: -8.411,
        spectral: "M5.5V", temp_k: 2953.0, luminosity: 0.00180, mass: 0.113, radius: 0.156, age_gyr: 10.00,
        notes: "Has three rocky planets, including GJ 1061 c and d in or near the habitable zone." },

    // ── 15–30 ly ─────────────────────────────────────────────────────────────
    CatalogStar { name: "Epsilon Indi A",
        x_ly: 5.677, y_ly: -3.141, z_ly: -9.937,
        spectral: "K5Ve", temp_k: 4280.0, luminosity: 0.150, mass: 0.762, radius: 0.732, age_gyr: 1.30,
        notes: "Has a brown dwarf binary companion. One of nearest Sun-like stars to the south celestial pole." },

    CatalogStar { name: "Groombridge 34 A",
        x_ly: 8.334, y_ly: 0.671, z_ly: 8.073,
        spectral: "M1.5V", temp_k: 3701.0, luminosity: 0.0218, mass: 0.404, radius: 0.388, age_gyr: 5.70,
        notes: "Close M-dwarf binary. GX And & GQ And. Possibly hosts a super-Earth." },

    CatalogStar { name: "Vega",
        x_ly: 3.051, y_ly: -19.271, z_ly: 15.701,
        spectral: "A0Va", temp_k: 9602.0, luminosity: 40.12, mass: 2.135, radius: 2.362, age_gyr: 0.455,
        notes: "Northern pole star ~14,000 years ago. Has a circumstellar disk suggesting planet formation." },

    CatalogStar { name: "Fomalhaut",
        x_ly: 21.092, y_ly: -5.834, z_ly: -12.421,
        spectral: "A3V", temp_k: 8590.0, luminosity: 16.63, mass: 1.920, radius: 1.842, age_gyr: 0.44,
        notes: "Has a famous bright debris ring imaged by Hubble. A disputed planet candidate (Fomalhaut b)." },

    CatalogStar { name: "55 Cancri A",
        x_ly: -24.76, y_ly: 26.09, z_ly: 19.42,
        spectral: "G8V", temp_k: 5196.0, luminosity: 0.582, mass: 0.905, radius: 0.943, age_gyr: 10.20,
        notes: "Has 5 confirmed planets including 55 Cnc e — a super-Earth with a likely lava ocean surface." },

    CatalogStar { name: "TRAPPIST-1",
        x_ly: 39.430, y_ly: -9.178, z_ly: -3.566,
        spectral: "M8V", temp_k: 2566.0, luminosity: 0.000553, mass: 0.0898, radius: 0.1192, age_gyr: 7.60,
        notes: "7 Earth-size planets, 3 in the habitable zone (TRAPPIST-1 d, e, f). Landmark discovery." },

    // ── Notable bright / giant stars ─────────────────────────────────────────
    CatalogStar { name: "Pollux",
        x_ly: -13.165, y_ly: 26.745, z_ly: 15.858,
        spectral: "K0IIIb", temp_k: 4666.0, luminosity: 32.70, mass: 1.86, radius: 8.80, age_gyr: 7.23,
        notes: "Nearest giant star. Has a confirmed gas-giant planet — Pollux b (Thestias), 590 day orbit." },

    CatalogStar { name: "Arcturus",
        x_ly: -28.830, y_ly: -19.323, z_ly: 12.054,
        spectral: "K1.5IIIFe-0.5", temp_k: 4286.0, luminosity: 170.0, mass: 1.08, radius: 25.40, age_gyr: 7.10,
        notes: "Brightest star in the northern hemisphere. Red giant moving fast through the Milky Way disk." },

    CatalogStar { name: "Capella A",
        x_ly: 5.583, y_ly: 29.272, z_ly: 30.877,
        spectral: "G8III", temp_k: 4943.0, luminosity: 78.70, mass: 2.57, radius: 11.98, age_gyr: 1.70,
        notes: "Binary giant system. Capella A and B are two giant stars orbiting each other at ~1 AU." },

    CatalogStar { name: "Aldebaran",
        x_ly: 23.006, y_ly: 59.655, z_ly: 18.925,
        spectral: "K5+", temp_k: 3910.0, luminosity: 518.0, mass: 1.13, radius: 44.20, age_gyr: 6.40,
        notes: "Red giant eye of Taurus. Its radius is so large that if it replaced the Sun, it would engulf Mercury." },

    CatalogStar { name: "Regulus",
        x_ly: -68.200, y_ly: 36.270, z_ly: 16.380,
        spectral: "B7V", temp_k: 11209.0, luminosity: 363.0, mass: 3.80, radius: 3.09, age_gyr: 0.075,
        notes: "Brightest star in Leo. Spins so fast it bulges at the equator — rotationally oblate." },

    CatalogStar { name: "Polaris",
        x_ly: 4.410, y_ly: 3.443, z_ly: 432.970,
        spectral: "F7Ib", temp_k: 6015.0, luminosity: 2500.0, mass: 5.40, radius: 37.50, age_gyr: 0.07,
        notes: "Current north pole star. A Cepheid variable — pulses in brightness with ~4 day period." },

    CatalogStar { name: "Spica",
        x_ly: -229.16, y_ly: -87.50, z_ly: -48.37,
        spectral: "B1III", temp_k: 22400.0, luminosity: 20500.0, mass: 10.25, radius: 7.47, age_gyr: 0.012,
        notes: "Brightest in Virgo. A binary where tidal distortion makes both stars pear-shaped." },

    CatalogStar { name: "Antares",
        x_ly: -208.42, y_ly: -499.38, z_ly: -268.90,
        spectral: "M1.5Iab", temp_k: 3400.0, luminosity: 75000.0, mass: 12.40, radius: 700.0, age_gyr: 0.012,
        notes: "Supergiant in Scorpius. If placed at the Sun, its surface would extend past Mars. Will go supernova." },

    CatalogStar { name: "Rigel",
        x_ly: 166.5, y_ly: 835.5, z_ly: -122.6,
        spectral: "B8Ia", temp_k: 12100.0, luminosity: 120000.0, mass: 21.0, radius: 78.9, age_gyr: 0.008,
        notes: "Blue supergiant — the brightest star in Orion. One of the most luminous stars in the galaxy." },

    CatalogStar { name: "Betelgeuse",
        x_ly: 14.49, y_ly: 694.8, z_ly: 90.3,
        spectral: "M1-M2Ia", temp_k: 3600.0, luminosity: 126000.0, mass: 11.43, radius: 764.0, age_gyr: 0.008,
        notes: "Red supergiant in Orion. Will explode as a supernova within ~100,000 years. Dimmed dramatically in 2019." },

    CatalogStar { name: "Deneb",
        x_ly: 1182.0, y_ly: -1393.7, z_ly: 1849.1,
        spectral: "A2Ia", temp_k: 8525.0, luminosity: 196000.0, mass: 19.0, radius: 203.0, age_gyr: 0.01,
        notes: "Distance uncertain (~2,600 ly). If at Sirius's distance, it would cast shadows at night." },

    // ── Famous exoplanet hosts ────────────────────────────────────────────────
    CatalogStar { name: "Gliese 667C",
        x_ly: -3.413, y_ly: -19.047, z_ly: -13.548,
        spectral: "M1.5V", temp_k: 3700.0, luminosity: 0.0137, mass: 0.327, radius: 0.337, age_gyr: 2.00,
        notes: "Has up to 3 potentially habitable planets (GJ 667C c, e, f). Part of a triple-star system." },

    CatalogStar { name: "Kepler-186",
        x_ly: 199.7, y_ly: -366.7, z_ly: 402.9,
        spectral: "M1V", temp_k: 3788.0, luminosity: 0.0404, mass: 0.478, radius: 0.472, age_gyr: 4.00,
        notes: "Has Kepler-186f — the first Earth-sized planet confirmed in a habitable zone of another star." },

    CatalogStar { name: "Kepler-442",
        x_ly: 240.9, y_ly: -905.1, z_ly: 759.1,
        spectral: "K5V", temp_k: 4402.0, luminosity: 0.112, mass: 0.610, radius: 0.598, age_gyr: 2.90,
        notes: "Has Kepler-442b — a super-Earth with one of the highest Earth Similarity Index scores ever measured." },

    // ── Galactic landmarks ───────────────────────────────────────────────────
    CatalogStar { name: "Sagittarius A*",
        x_ly: -25677.0, y_ly: -5000.0, z_ly: -27.0,
        spectral: "BH", temp_k: 0.0, luminosity: 0.0, mass: 4_100_000.0, radius: 0.0, age_gyr: 13.6,
        notes: "Supermassive black hole at the galactic centre. 4.1 million solar masses." },
];

// ── Known planet data ────────────────────────────────────────────────────────

pub static PLANETS: &[CatalogPlanet] = &[
    // ── Solar System ─────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Sol", name: "Mercury",
        orbit_au: 0.387, mass_earth: 0.0553, radius_earth: 0.3829,
        period_days: 87.97, temp_k: 440.0, planet_type: PlanetType::Barren, moon_count: 0,
        notes: "Extreme temperature swings. No atmosphere." },

    CatalogPlanet { star_name: "Sol", name: "Venus",
        orbit_au: 0.723, mass_earth: 0.815, radius_earth: 0.9499,
        period_days: 224.70, temp_k: 737.0, planet_type: PlanetType::Barren, moon_count: 0,
        notes: "Runaway greenhouse effect. 92 bar CO₂ atmosphere." },

    CatalogPlanet { star_name: "Sol", name: "Earth",
        orbit_au: 1.000, mass_earth: 1.000, radius_earth: 1.000,
        period_days: 365.25, temp_k: 288.0, planet_type: PlanetType::OceanWorld, moon_count: 1,
        notes: "Only known inhabited planet. N₂/O₂ atmosphere." },

    CatalogPlanet { star_name: "Sol", name: "Mars",
        orbit_au: 1.524, mass_earth: 0.1074, radius_earth: 0.5320,
        period_days: 686.97, temp_k: 231.0, planet_type: PlanetType::OceanWorld, moon_count: 2,
        notes: "Shallow northern seas. Thin but breathable atmosphere. Home." },

    CatalogPlanet { star_name: "Sol", name: "Jupiter",
        orbit_au: 5.203, mass_earth: 317.8, radius_earth: 11.21,
        period_days: 4332.59, temp_k: 165.0, planet_type: PlanetType::GasGiant, moon_count: 95,
        notes: "Largest planet. Shields inner planets from comets." },

    CatalogPlanet { star_name: "Sol", name: "Saturn",
        orbit_au: 9.537, mass_earth: 95.16, radius_earth: 9.45,
        period_days: 10759.22, temp_k: 134.0, planet_type: PlanetType::GasGiant, moon_count: 146,
        notes: "Ring system extends to 282,000 km. Less dense than water." },

    CatalogPlanet { star_name: "Sol", name: "Uranus",
        orbit_au: 19.191, mass_earth: 14.54, radius_earth: 4.007,
        period_days: 30688.50, temp_k: 76.0, planet_type: PlanetType::IceGiant, moon_count: 28,
        notes: "Rotates on its side (98° axial tilt). Ice giant." },

    CatalogPlanet { star_name: "Sol", name: "Neptune",
        orbit_au: 30.069, mass_earth: 17.15, radius_earth: 3.883,
        period_days: 60182.00, temp_k: 72.0, planet_type: PlanetType::IceGiant, moon_count: 16,
        notes: "Supersonic winds. Its moon Triton orbits retrograde — likely a captured Kuiper Belt object." },

    // ── Proxima Centauri ──────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Proxima Centauri", name: "Proxima Centauri b",
        orbit_au: 0.0485, mass_earth: 1.27, radius_earth: 1.10,
        period_days: 11.186, temp_k: 234.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "Nearest known exoplanet. In the habitable zone. Likely tidally locked." },

    // ── Ross 128 ──────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Ross 128", name: "Ross 128 b",
        orbit_au: 0.0496, mass_earth: 1.35, radius_earth: 1.13,
        period_days: 9.860, temp_k: 294.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "Temperate rocky planet. Among best candidates for habitability within 15 ly." },

    // ── Tau Ceti ──────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Tau Ceti", name: "Tau Ceti e",
        orbit_au: 0.538, mass_earth: 2.78, radius_earth: 1.44,
        period_days: 162.87, temp_k: 288.0, planet_type: PlanetType::SuperEarth, moon_count: 0,
        notes: "Super-Earth near inner edge of habitable zone. Dense debris disk may mean frequent impacts." },

    CatalogPlanet { star_name: "Tau Ceti", name: "Tau Ceti f",
        orbit_au: 0.879, mass_earth: 1.75, radius_earth: 1.21,
        period_days: 636.13, temp_k: 225.0, planet_type: PlanetType::SuperEarth, moon_count: 0,
        notes: "In the conservative habitable zone. Just 1.75 Earth masses — potentially rocky." },

    // ── TRAPPIST-1 ────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "TRAPPIST-1", name: "TRAPPIST-1 b",
        orbit_au: 0.01154, mass_earth: 1.017, radius_earth: 1.116,
        period_days: 1.511, temp_k: 400.0, planet_type: PlanetType::Barren, moon_count: 0,
        notes: "Innermost TRAPPIST planet. Too hot for liquid water." },

    CatalogPlanet { star_name: "TRAPPIST-1", name: "TRAPPIST-1 c",
        orbit_au: 0.01580, mass_earth: 1.156, radius_earth: 1.097,
        period_days: 2.422, temp_k: 340.0, planet_type: PlanetType::Barren, moon_count: 0,
        notes: "Venus-like. May have a thick CO₂ atmosphere." },

    CatalogPlanet { star_name: "TRAPPIST-1", name: "TRAPPIST-1 d",
        orbit_au: 0.02227, mass_earth: 0.297, radius_earth: 0.788,
        period_days: 4.050, temp_k: 288.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "Near inner HZ edge. Receives similar flux to Earth. Lightest TRAPPIST planet." },

    CatalogPlanet { star_name: "TRAPPIST-1", name: "TRAPPIST-1 e",
        orbit_au: 0.02925, mass_earth: 0.772, radius_earth: 0.920,
        period_days: 6.101, temp_k: 251.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "Best habitable zone candidate in the TRAPPIST system. Similar size and density to Earth." },

    CatalogPlanet { star_name: "TRAPPIST-1", name: "TRAPPIST-1 f",
        orbit_au: 0.03849, mass_earth: 0.934, radius_earth: 1.045,
        period_days: 9.207, temp_k: 219.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "In the habitable zone. Could host liquid water with a substantial greenhouse atmosphere." },

    CatalogPlanet { star_name: "TRAPPIST-1", name: "TRAPPIST-1 g",
        orbit_au: 0.04683, mass_earth: 1.148, radius_earth: 1.129,
        period_days: 12.354, temp_k: 198.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "Outer HZ. Needs a strong greenhouse effect to maintain surface liquid water." },

    CatalogPlanet { star_name: "TRAPPIST-1", name: "TRAPPIST-1 h",
        orbit_au: 0.06189, mass_earth: 0.331, radius_earth: 0.755,
        period_days: 18.767, temp_k: 173.0, planet_type: PlanetType::Barren, moon_count: 0,
        notes: "Outermost TRAPPIST planet. Likely too cold for liquid water without thick atmosphere." },

    // ── Teegarden ─────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Teegarden's Star", name: "Teegarden b",
        orbit_au: 0.0252, mass_earth: 1.05, radius_earth: 1.02,
        period_days: 4.910, temp_k: 259.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "Earth-mass. Highest Earth Similarity Index of any confirmed exoplanet at time of discovery." },

    CatalogPlanet { star_name: "Teegarden's Star", name: "Teegarden c",
        orbit_au: 0.0443, mass_earth: 1.11, radius_earth: 1.04,
        period_days: 11.416, temp_k: 182.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "Mars-like temperatures. May still have liquid water with a thick enough atmosphere." },

    // ── Luyten's Star ─────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Luyten's Star", name: "Luyten b",
        orbit_au: 0.0911, mass_earth: 2.89, radius_earth: 1.40,
        period_days: 18.650, temp_k: 259.0, planet_type: PlanetType::SuperEarth, moon_count: 0,
        notes: "Super-Earth in the habitable zone. One of the best nearby targets for biosignature searches." },

    // ── Kapteyn's Star ────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Kapteyn's Star", name: "Kapteyn b",
        orbit_au: 0.168, mass_earth: 4.80, radius_earth: 1.60,
        period_days: 48.616, temp_k: 209.0, planet_type: PlanetType::SuperEarth, moon_count: 0,
        notes: "Super-Earth in the HZ of one of the oldest stars known. If habitable, life had billions of extra years." },

    // ── Gliese 667C ───────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Gliese 667C", name: "GJ 667C c",
        orbit_au: 0.1251, mass_earth: 3.81, radius_earth: 1.54,
        period_days: 28.143, temp_k: 277.0, planet_type: PlanetType::SuperEarth, moon_count: 0,
        notes: "In the habitable zone. One of the highest-priority targets for atmospheric characterization." },

    // ── Epsilon Eridani ───────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Epsilon Eridani", name: "Epsilon Eridani b",
        orbit_au: 3.39, mass_earth: 534.7, radius_earth: 11.0,
        period_days: 2502.0, temp_k: 133.0, planet_type: PlanetType::GasGiant, moon_count: 0,
        notes: "Gas giant beyond the habitable zone. Has a thick debris disk system suggesting active planetesimals." },

    // ── Kepler-186 ────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Kepler-186", name: "Kepler-186 f",
        orbit_au: 0.432, mass_earth: 1.11, radius_earth: 1.17,
        period_days: 129.945, temp_k: 188.0, planet_type: PlanetType::Terrestrial, moon_count: 0,
        notes: "First Earth-sized planet confirmed in the habitable zone of another star (2014). Historic discovery." },

    // ── Kepler-442 ────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Kepler-442", name: "Kepler-442 b",
        orbit_au: 0.409, mass_earth: 2.30, radius_earth: 1.34,
        period_days: 112.305, temp_k: 233.0, planet_type: PlanetType::SuperEarth, moon_count: 0,
        notes: "ESI score of 0.84 — among the highest of any known exoplanet. Cool but potentially habitable." },

    // ── Pollux ────────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "Pollux", name: "Pollux b",
        orbit_au: 1.64, mass_earth: 534.7, radius_earth: 11.0,
        period_days: 589.64, temp_k: 165.0, planet_type: PlanetType::GasGiant, moon_count: 0,
        notes: "First planet confirmed around a giant star. Officially named Thestias." },

    // ── 55 Cancri ─────────────────────────────────────────────────────────────
    CatalogPlanet { star_name: "55 Cancri A", name: "55 Cnc e",
        orbit_au: 0.01544, mass_earth: 8.09, radius_earth: 1.875,
        period_days: 0.737, temp_k: 2673.0, planet_type: PlanetType::Barren, moon_count: 0,
        notes: "Ultra-hot super-Earth likely with a global lava ocean. Year is 17.5 hours long." },
];
