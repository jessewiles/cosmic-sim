use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::chemistry::atmosphere::Atmosphere;
use crate::physics::orbital::{surface_gravity, escape_velocity, orbital_period, orbital_velocity, au_to_m};
use crate::physics::constants::M_SUN;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanetType {
    /// Rocky, Earth-like
    Terrestrial,
    /// Large rocky, thick atmosphere (Venus-like)
    SuperEarth,
    /// Gas giant (Jupiter/Saturn-like)
    GasGiant,
    /// Ice giant (Uranus/Neptune-like)
    IceGiant,
    /// Small, airless (Mercury/Moon-like)
    Barren,
    /// Ocean world
    OceanWorld,
    /// Hot Jupiter
    HotJupiter,
}

impl PlanetType {
    pub fn display(&self) -> &'static str {
        match self {
            PlanetType::Terrestrial => "Terrestrial",
            PlanetType::SuperEarth  => "Super-Earth",
            PlanetType::GasGiant    => "Gas Giant",
            PlanetType::IceGiant    => "Ice Giant",
            PlanetType::Barren      => "Barren",
            PlanetType::OceanWorld  => "Ocean World",
            PlanetType::HotJupiter  => "Hot Jupiter",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planet {
    pub name: String,
    pub planet_type: PlanetType,
    /// Semi-major axis in AU
    pub orbit_au: f64,
    /// Mass in Earth masses
    pub mass_earth: f64,
    /// Radius in Earth radii
    pub radius_earth: f64,
    /// Surface (or cloud-top) temperature in K
    pub surface_temp_k: f64,
    pub atmosphere: Atmosphere,
    pub has_moons: bool,
    pub moon_count: u8,
}

const EARTH_MASS_KG: f64 = 5.972e24;
const EARTH_RADIUS_M: f64 = 6.371e6;

impl Planet {
    pub fn generate<R: Rng>(
        rng: &mut R,
        name: String,
        orbit_au: f64,
        star_luminosity_solar: f64,
        _star_mass_solar: f64,
    ) -> Self {
        // Equilibrium temperature (Bond albedo ~0.3)
        let albedo = rng.random_range(0.1f64..0.7f64);
        let temp_k = 278.0 * (star_luminosity_solar * (1.0 - albedo)).powf(0.25) / orbit_au.sqrt();

        let planet_type = Self::pick_type(rng, orbit_au, temp_k, star_luminosity_solar);

        let (mass_earth, radius_earth) = match &planet_type {
            PlanetType::Barren      => (rng.random_range(0.01..0.5),  rng.random_range(0.2..0.7)),
            PlanetType::Terrestrial => (rng.random_range(0.3..2.0),   rng.random_range(0.7..1.3)),
            PlanetType::SuperEarth  => (rng.random_range(2.0..10.0),  rng.random_range(1.3..2.5)),
            PlanetType::OceanWorld  => (rng.random_range(0.5..5.0),   rng.random_range(0.9..1.8)),
            PlanetType::IceGiant    => (rng.random_range(10.0..50.0), rng.random_range(2.5..5.0)),
            PlanetType::GasGiant    => (rng.random_range(50.0..500.0),rng.random_range(5.0..12.0)),
            PlanetType::HotJupiter  => (rng.random_range(100.0..2000.0), rng.random_range(8.0..15.0)),
        };

        let atmosphere = Atmosphere::generate(rng, &planet_type, temp_k, mass_earth);
        let has_moons = rng.random::<f64>() > 0.4 || matches!(planet_type, PlanetType::GasGiant | PlanetType::IceGiant);
        let moon_count = if has_moons {
            match &planet_type {
                PlanetType::GasGiant | PlanetType::IceGiant => rng.random_range(1u8..80u8),
                _ => rng.random_range(1u8..4u8),
            }
        } else { 0 };

        Planet {
            name, planet_type, orbit_au, mass_earth, radius_earth,
            surface_temp_k: temp_k, atmosphere, has_moons, moon_count,
        }
    }

    fn pick_type<R: Rng>(rng: &mut R, orbit_au: f64, _temp_k: f64, lum: f64) -> PlanetType {
        // Hot Jupiters hug their star
        if orbit_au < 0.1 && rng.random::<f64>() < 0.3 {
            return PlanetType::HotJupiter;
        }
        // Frost line ~2.7 AU (scaled by sqrt of luminosity)
        let frost_line = 2.7 * lum.sqrt();
        if orbit_au > frost_line {
            let roll = rng.random::<f64>();
            if roll < 0.4 { PlanetType::GasGiant }
            else if roll < 0.7 { PlanetType::IceGiant }
            else { PlanetType::Barren }
        } else {
            let roll = rng.random::<f64>();
            if roll < 0.35      { PlanetType::Terrestrial }
            else if roll < 0.55 { PlanetType::Barren }
            else if roll < 0.70 { PlanetType::SuperEarth }
            else if roll < 0.85 { PlanetType::OceanWorld }
            else                { PlanetType::GasGiant }
        }
    }

    pub fn mass_kg(&self) -> f64 { self.mass_earth * EARTH_MASS_KG }
    pub fn radius_m(&self) -> f64 { self.radius_earth * EARTH_RADIUS_M }

    pub fn surface_gravity_ms2(&self) -> f64 {
        surface_gravity(self.mass_kg(), self.radius_m())
    }

    pub fn escape_velocity_ms(&self) -> f64 {
        escape_velocity(self.mass_kg(), self.radius_m())
    }

    pub fn orbital_period_days(&self, star_mass_solar: f64) -> f64 {
        let star_kg = star_mass_solar * M_SUN;
        orbital_period(au_to_m(self.orbit_au), star_kg) / 86_400.0
    }

    pub fn orbital_velocity_kms(&self, star_mass_solar: f64) -> f64 {
        let star_kg = star_mass_solar * M_SUN;
        orbital_velocity(au_to_m(self.orbit_au), star_kg) / 1000.0
    }

    pub fn is_in_habitable_zone(&self, hz_inner: f64, hz_outer: f64) -> bool {
        self.orbit_au >= hz_inner && self.orbit_au <= hz_outer
    }

    /// Infrastructure risk for digital consciousnesses operating on the surface.
    pub fn infrastructure_risk(&self) -> InfraRisk {
        let t = self.surface_temp_k;
        let p = self.atmosphere.pressure_bar;

        // Gas giants / hot Jupiters: crushing pressure, no solid surface
        match self.planet_type {
            PlanetType::GasGiant | PlanetType::HotJupiter => return InfraRisk::Extreme,
            PlanetType::IceGiant                          => return InfraRisk::High,
            _ => {}
        }

        // Temperature extremes
        if t > 700.0 || t < 50.0 { return InfraRisk::Extreme; }
        if t > 500.0 || t < 80.0 { return InfraRisk::High; }

        // Pressure extremes
        if p > 100.0 { return InfraRisk::Extreme; }
        if p > 20.0  { return InfraRisk::High; }

        // Corrosive atmosphere
        let has_sulfur = self.atmosphere.components.iter().any(|c| c.symbol == "SO₂");
        if has_sulfur { return InfraRisk::High; }

        // Vacuum — no convective cooling, unshielded radiation
        if p == 0.0 { return InfraRisk::Moderate; }

        // Elevated temperature or pressure
        if t > 350.0 || t < 150.0 { return InfraRisk::Moderate; }
        if p > 5.0                 { return InfraRisk::Moderate; }

        // Benign band
        if t >= 200.0 && t <= 320.0 && p >= 0.5 && p <= 3.0 {
            return InfraRisk::Minimal;
        }

        InfraRisk::Low
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InfraRisk {
    Minimal,
    Low,
    Moderate,
    High,
    Extreme,
}

impl InfraRisk {
    pub fn label(&self) -> &'static str {
        match self {
            InfraRisk::Minimal  => "MINIMAL",
            InfraRisk::Low      => "LOW",
            InfraRisk::Moderate => "MODERATE",
            InfraRisk::High     => "HIGH",
            InfraRisk::Extreme  => "EXTREME",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            InfraRisk::Minimal  => "benign — full sensor deployment possible",
            InfraRisk::Low      => "standard shielding sufficient",
            InfraRisk::Moderate => "elevated shielding recommended",
            InfraRisk::High     => "reinforced housing required",
            InfraRisk::Extreme  => "hostile to digital substrate",
        }
    }

    pub fn is_low(&self) -> bool {
        matches!(self, InfraRisk::Minimal | InfraRisk::Low)
    }
}
