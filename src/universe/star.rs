use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::physics::constants::{M_SUN, R_SUN, L_SUN};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpectralClass {
    O, B, A, F, G, K, M,
    /// Neutron star
    NS,
    /// White dwarf
    WD,
    /// Black hole
    BH,
}

impl SpectralClass {
    /// Parse the leading character(s) of a spectral type string.
    pub fn from_spectral_str(s: &str) -> Self {
        let s = s.trim();
        if s == "BH" { return SpectralClass::BH; }
        match s.chars().next() {
            Some('O') => SpectralClass::O,
            Some('B') => SpectralClass::B,
            Some('A') => SpectralClass::A,
            Some('F') => SpectralClass::F,
            Some('G') => SpectralClass::G,
            Some('K') => SpectralClass::K,
            Some('M') => SpectralClass::M,
            Some('D') => SpectralClass::WD, // DA, DQ, DZ white dwarfs
            Some('N') | Some('P') => SpectralClass::NS,
            _ => SpectralClass::M,
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            SpectralClass::O  => "O (blue, ~30 000 K)",
            SpectralClass::B  => "B (blue-white, ~10 000 K)",
            SpectralClass::A  => "A (white, ~7 500 K)",
            SpectralClass::F  => "F (yellow-white, ~6 000 K)",
            SpectralClass::G  => "G (yellow, ~5 500 K) — like the Sun",
            SpectralClass::K  => "K (orange, ~4 000 K)",
            SpectralClass::M  => "M (red dwarf, ~3 000 K)",
            SpectralClass::NS => "Neutron Star",
            SpectralClass::WD => "White Dwarf",
            SpectralClass::BH => "Black Hole",
        }
    }

    /// Rough probability weight in a realistic galaxy sample
    fn weight(&self) -> f64 {
        match self {
            SpectralClass::M  => 76.0,
            SpectralClass::K  => 12.0,
            SpectralClass::G  => 7.0,
            SpectralClass::F  => 3.0,
            SpectralClass::A  => 1.0,
            SpectralClass::B  => 0.12,
            SpectralClass::O  => 0.00003,
            SpectralClass::NS => 0.4,
            SpectralClass::WD => 0.5,
            SpectralClass::BH => 0.01,
        }
    }

    pub fn random<R: Rng>(rng: &mut R) -> Self {
        let classes = [
            SpectralClass::M, SpectralClass::K, SpectralClass::G,
            SpectralClass::F, SpectralClass::A, SpectralClass::B,
            SpectralClass::O, SpectralClass::NS, SpectralClass::WD,
            SpectralClass::BH,
        ];
        let total: f64 = classes.iter().map(|c| c.weight()).sum();
        let mut roll = rng.random::<f64>() * total;
        for c in &classes {
            roll -= c.weight();
            if roll <= 0.0 {
                return c.clone();
            }
        }
        SpectralClass::M
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Star {
    pub name: String,
    pub spectral_class: SpectralClass,
    /// In solar masses
    pub mass: f64,
    /// Surface temperature in Kelvin
    pub temperature_k: f64,
    /// In solar radii
    pub radius: f64,
    /// In solar luminosities
    pub luminosity: f64,
    /// Age in billions of years
    pub age_gyr: f64,
}

impl Star {
    pub fn generate<R: Rng>(rng: &mut R, name: String) -> Self {
        let spectral_class = SpectralClass::random(rng);
        let (mass, temp, radius, luminosity) = match &spectral_class {
            SpectralClass::O  => (
                rng.random_range(16.0..150.0),
                rng.random_range(30_000.0..50_000.0),
                rng.random_range(6.6..100.0),
                rng.random_range(30_000.0..1_000_000.0),
            ),
            SpectralClass::B  => (
                rng.random_range(2.1..16.0),
                rng.random_range(10_000.0..30_000.0),
                rng.random_range(1.8..6.6),
                rng.random_range(25.0..30_000.0),
            ),
            SpectralClass::A  => (
                rng.random_range(1.4..2.1),
                rng.random_range(7_500.0..10_000.0),
                rng.random_range(1.4..1.8),
                rng.random_range(5.0..25.0),
            ),
            SpectralClass::F  => (
                rng.random_range(1.04..1.4),
                rng.random_range(6_000.0..7_500.0),
                rng.random_range(1.15..1.4),
                rng.random_range(1.5..5.0),
            ),
            SpectralClass::G  => (
                rng.random_range(0.8..1.04),
                rng.random_range(5_200.0..6_000.0),
                rng.random_range(0.96..1.15),
                rng.random_range(0.6..1.5),
            ),
            SpectralClass::K  => (
                rng.random_range(0.45..0.8),
                rng.random_range(3_700.0..5_200.0),
                rng.random_range(0.7..0.96),
                rng.random_range(0.08..0.6),
            ),
            SpectralClass::M  => (
                rng.random_range(0.08..0.45),
                rng.random_range(2_400.0..3_700.0),
                rng.random_range(0.1..0.7),
                rng.random_range(0.0001..0.08),
            ),
            SpectralClass::WD => (
                rng.random_range(0.5..1.4),
                rng.random_range(8_000.0..150_000.0),
                rng.random_range(0.008..0.02),
                rng.random_range(0.0001..0.01),
            ),
            SpectralClass::NS => (1.4, 1_000_000.0, 0.000014, 0.00001),
            SpectralClass::BH => (
                rng.random_range(3.0..50.0),
                0.0,
                0.0,
                0.0,
            ),
        };
        let age_gyr = rng.random_range(0.1..13.0);
        Star { name, spectral_class, mass, temperature_k: temp, radius, luminosity, age_gyr }
    }

    pub fn mass_kg(&self) -> f64 { self.mass * M_SUN }
    pub fn radius_m(&self) -> f64 { self.radius * R_SUN }
    pub fn luminosity_w(&self) -> f64 { self.luminosity * L_SUN }

    /// Habitable zone inner/outer edge in AU (simple luminosity scaling)
    pub fn habitable_zone_au(&self) -> (f64, f64) {
        let inner = (self.luminosity / 1.1).sqrt();
        let outer = (self.luminosity / 0.53).sqrt();
        (inner, outer)
    }

    /// Build a Star from a real catalog entry.
    pub fn from_catalog(entry: &crate::universe::catalog::CatalogStar) -> Self {
        Star {
            name:           entry.name.to_string(),
            spectral_class: SpectralClass::from_spectral_str(entry.spectral),
            mass:           entry.mass,
            temperature_k:  entry.temp_k,
            radius:         entry.radius,
            luminosity:     entry.luminosity,
            age_gyr:        entry.age_gyr,
        }
    }
}
