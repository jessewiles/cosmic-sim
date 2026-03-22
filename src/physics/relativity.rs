use super::constants::C;

/// Lorentz factor γ = 1 / sqrt(1 - v²/c²)
pub fn lorentz_factor(velocity: f64) -> f64 {
    let beta = velocity / C;
    1.0 / (1.0 - beta * beta).sqrt()
}

/// Time dilation: proper time elapsed for traveler given coordinate time and velocity
pub fn time_dilation(coordinate_time_s: f64, velocity: f64) -> f64 {
    coordinate_time_s / lorentz_factor(velocity)
}

/// Relativistic kinetic energy in joules
pub fn relativistic_kinetic_energy(mass_kg: f64, velocity: f64) -> f64 {
    let gamma = lorentz_factor(velocity);
    (gamma - 1.0) * mass_kg * C * C
}

/// Schwarzschild radius (event horizon) for a given mass in meters
pub fn schwarzschild_radius(mass_kg: f64) -> f64 {
    use super::constants::G;
    2.0 * G * mass_kg / (C * C)
}

/// Gravitational time dilation factor near a massive body.
/// Returns the rate at which a clock at `distance_m` from center ticks
/// relative to a clock at infinity. Factor < 1 means slower.
pub fn gravitational_time_dilation(mass_kg: f64, distance_m: f64) -> f64 {
    let rs = schwarzschild_radius(mass_kg);
    (1.0 - rs / distance_m).sqrt()
}
