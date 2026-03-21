use super::constants::{G, AU};

/// Orbital period in seconds via Kepler's third law
pub fn orbital_period(semi_major_axis_m: f64, central_mass_kg: f64) -> f64 {
    use std::f64::consts::PI;
    2.0 * PI * (semi_major_axis_m.powi(3) / (G * central_mass_kg)).sqrt()
}

/// Orbital velocity at a given distance from the central body
pub fn orbital_velocity(distance_m: f64, central_mass_kg: f64) -> f64 {
    (G * central_mass_kg / distance_m).sqrt()
}

/// Surface gravity in m/s²
pub fn surface_gravity(mass_kg: f64, radius_m: f64) -> f64 {
    G * mass_kg / (radius_m * radius_m)
}

/// Escape velocity in m/s
pub fn escape_velocity(mass_kg: f64, radius_m: f64) -> f64 {
    (2.0 * G * mass_kg / radius_m).sqrt()
}

/// Hill sphere radius — how far a planet's gravity dominates over the star
pub fn hill_sphere(semi_major_axis_m: f64, planet_mass_kg: f64, star_mass_kg: f64) -> f64 {
    semi_major_axis_m * (planet_mass_kg / (3.0 * star_mass_kg)).powf(1.0 / 3.0)
}

pub fn au_to_m(au: f64) -> f64 { au * AU }
pub fn m_to_au(m: f64) -> f64 { m / AU }
