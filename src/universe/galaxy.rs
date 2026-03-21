use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use crate::universe::system::StarSystem;
use crate::universe::catalog;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GalaxyMode {
    /// Real star catalog + known exoplanets, procedural fallback for deep space.
    RealUniverse,
    /// Fully deterministic procedural generation — no catalog data.
    Procedural,
}

pub struct Galaxy {
    pub name: String,
    pub seed: u64,
    pub mode: GalaxyMode,
    /// Cache of discovered systems (catalog + procedural)
    pub known_systems: Vec<StarSystem>,
}

impl Galaxy {
    pub fn new(name: String, seed: u64, mode: GalaxyMode) -> Self {
        Galaxy { name, seed, mode, known_systems: vec![] }
    }

    /// Return the star system at the given position (light-years from Sol).
    /// In RealUniverse mode, checks the real star catalog first (within 0.4 ly),
    /// then falls back to deterministic procedural generation.
    pub fn system_at(&mut self, x: f64, y: f64, z: f64) -> StarSystem {
        // Check cache by name proximity
        // (float coords can drift, so match on nearest cached within 0.1 ly)
        for s in &self.known_systems {
            let dx = s.galactic_x - x;
            let dy = s.galactic_y - y;
            let dz = s.galactic_z - z;
            if (dx*dx + dy*dy + dz*dz).sqrt() < 0.1 {
                return s.clone();
            }
        }

        let system = match self.mode {
            GalaxyMode::RealUniverse => {
                if let Some((entry, _)) = catalog::nearest_within(x, y, z, 0.4) {
                    StarSystem::from_catalog(entry)
                } else {
                    self.procedural_system(x, y, z)
                }
            }
            GalaxyMode::Procedural => self.procedural_system(x, y, z),
        };

        self.known_systems.push(system.clone());
        system
    }

    /// Look up a system by exact catalog name (case-insensitive).
    /// Returns None in Procedural mode (no catalog).
    pub fn system_by_name(&mut self, name: &str) -> Option<StarSystem> {
        if self.mode == GalaxyMode::Procedural { return None; }
        let entry = catalog::find_by_name(name)?;
        let sys = self.system_at(entry.x_ly, entry.y_ly, entry.z_ly);
        Some(sys)
    }

    /// List up to `n` nearest catalog stars to the given position.
    pub fn nearest_catalog_stars(x: f64, y: f64, z: f64, radius: f64)
        -> Vec<(&'static catalog::CatalogStar, f64)>
    {
        catalog::stars_within(x, y, z, radius)
    }

    // ── Procedural fallback ──────────────────────────────────────────────────

    fn procedural_system(&self, x: f64, y: f64, z: f64) -> StarSystem {
        // Round coords to nearest 0.5 ly grid for stable seeds
        let gx = (x * 2.0).round() / 2.0;
        let gy = (y * 2.0).round() / 2.0;
        let gz = (z * 2.0).round() / 2.0;

        let coord_seed = self.seed
            .wrapping_add(gx.to_bits().wrapping_mul(0x9e3779b97f4a7c15))
            .wrapping_add(gy.to_bits().wrapping_mul(0x6c62272e07bb0142))
            .wrapping_add(gz.to_bits().wrapping_mul(0xd2a98b26625eee7b));

        let mut rng = SmallRng::seed_from_u64(coord_seed);
        let name = Self::random_name(&mut rng);
        StarSystem::generate(name, coord_seed, gx, gy, gz)
    }

    fn random_name<R: Rng>(rng: &mut R) -> String {
        let prefixes = ["Kep", "Ari", "Tau", "Vel", "Cen", "Lyr", "Cyg",
                        "Her", "Aqu", "Per", "Ori", "Gem", "Leo", "Vir", "Sco"];
        let suffixes = ["ara", "ion", "eon", "ius", "an", "is", "os", "us",
                        "ix", "ax", "id", "ux", "el", "al", "on", "en"];
        let p = prefixes[rng.random_range(0..prefixes.len())];
        let s = suffixes[rng.random_range(0..suffixes.len())];
        format!("{}{}", p, s)
    }
}
