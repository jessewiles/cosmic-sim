use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use crate::universe::system::StarSystem;

pub struct Galaxy {
    pub name: String,
    pub seed: u64,
    /// Discovered/visited systems
    pub known_systems: Vec<StarSystem>,
}

impl Galaxy {
    pub fn new(name: String, seed: u64) -> Self {
        Galaxy { name, seed, known_systems: vec![] }
    }

    /// Generate or retrieve a system at grid coordinates (x, y, z) in light-years.
    /// The seed is derived deterministically so the same coordinates always
    /// produce the same system.
    pub fn system_at(&mut self, x: i32, y: i32, z: i32) -> StarSystem {
        // Check cache
        if let Some(s) = self.known_systems.iter().find(|s| {
            s.galactic_x as i32 == x && s.galactic_y as i32 == y && s.galactic_z as i32 == z
        }) {
            return s.clone();
        }

        let coord_seed = self.seed
            .wrapping_add((x as u64).wrapping_mul(0x9e3779b97f4a7c15))
            .wrapping_add((y as u64).wrapping_mul(0x6c62272e07bb0142))
            .wrapping_add((z as u64).wrapping_mul(0xd2a98b26625eee7b));

        let mut rng = SmallRng::seed_from_u64(coord_seed);
        let system_name = Self::random_name(&mut rng);
        let system = StarSystem::generate(
            system_name,
            coord_seed,
            x as f64, y as f64, z as f64,
        );
        self.known_systems.push(system.clone());
        system
    }

    fn random_name<R: Rng>(rng: &mut R) -> String {
        let prefixes = ["Kep", "Sol", "Ari", "Tau", "Vel", "Cen", "Lyr", "Cyg",
                        "Her", "Aqu", "Per", "Ori", "Gem", "Leo", "Vir", "Sco"];
        let suffixes = ["ara", "ion", "eon", "ius", "an", "is", "os", "us",
                        "ix", "ax", "id", "ux", "el", "al", "on", "en"];
        let p = prefixes[rng.random_range(0..prefixes.len())];
        let s = suffixes[rng.random_range(0..suffixes.len())];
        format!("{}{}", p, s)
    }
}
