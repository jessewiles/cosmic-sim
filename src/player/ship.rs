use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ship {
    pub name: String,
    /// Max velocity as fraction of c
    pub max_velocity_c: f64,
    /// Fuel (arbitrary units)
    pub fuel: f64,
    pub max_fuel: f64,
    /// Hull integrity 0.0–1.0
    pub hull: f64,
}

impl Ship {
    pub fn starter() -> Self {
        Ship {
            name: "Perihelion I".to_string(),
            max_velocity_c: 0.1,
            fuel: 100.0,
            max_fuel: 100.0,
            hull: 1.0,
        }
    }
}
