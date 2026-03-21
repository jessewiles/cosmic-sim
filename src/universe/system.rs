use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};
use crate::universe::star::Star;
use crate::universe::planet::Planet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarSystem {
    pub name: String,
    pub seed: u64,
    pub star: Star,
    pub planets: Vec<Planet>,
    /// Position in the galaxy (light-years from galactic center)
    pub galactic_x: f64,
    pub galactic_y: f64,
    pub galactic_z: f64,
}

impl StarSystem {
    pub fn generate(name: String, seed: u64, galactic_x: f64, galactic_y: f64, galactic_z: f64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);

        let star = Star::generate(&mut rng, name.clone());
        let planet_count = rng.random_range(0u8..=10u8);
        let (hz_inner, hz_outer) = star.habitable_zone_au();

        let mut planets = Vec::new();
        let mut orbit_au = rng.random_range(0.05f64..0.4f64);

        for i in 0..planet_count {
            let planet_name = format!("{} {}", name, roman(i + 1));
            let planet = Planet::generate(
                &mut rng,
                planet_name,
                orbit_au,
                star.luminosity,
                star.mass,
            );
            planets.push(planet);
            // Titius-Bode-ish spacing
            orbit_au *= rng.random_range(1.4f64..2.2f64);
        }

        StarSystem { name, seed, star, planets, galactic_x, galactic_y, galactic_z }
    }

    pub fn habitable_planets(&self) -> Vec<&Planet> {
        let (hz_inner, hz_outer) = self.star.habitable_zone_au();
        self.planets.iter()
            .filter(|p| p.is_in_habitable_zone(hz_inner, hz_outer) && p.atmosphere.is_breathable())
            .collect()
    }

    pub fn distance_to(&self, other: &StarSystem) -> f64 {
        let dx = self.galactic_x - other.galactic_x;
        let dy = self.galactic_y - other.galactic_y;
        let dz = self.galactic_z - other.galactic_z;
        (dx*dx + dy*dy + dz*dz).sqrt()
    }
}

fn roman(n: u8) -> &'static str {
    match n {
        1 => "I", 2 => "II", 3 => "III", 4 => "IV", 5 => "V",
        6 => "VI", 7 => "VII", 8 => "VIII", 9 => "IX", 10 => "X",
        _ => "?",
    }
}
