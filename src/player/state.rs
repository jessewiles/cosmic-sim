use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub name: String,
    /// Current position in light-years from Sol (x, y, z)
    pub position: [f64; 3],
    /// Ship velocity as fraction of c
    pub velocity_c: f64,
    /// Proper time elapsed for player (accounting for dilation)
    pub proper_time_s: f64,
    /// Coordinate time elapsed in the reference frame of the galaxy
    pub coordinate_time_s: f64,
    /// Current planet index if landed, None if in space
    pub landed_on: Option<usize>,
    /// Names of visited star systems
    pub visited_systems: Vec<String>,
}

impl PlayerState {
    pub fn new(name: String) -> Self {
        PlayerState {
            name,
            position: [0.0, 0.0, 0.0],
            velocity_c: 0.0,
            proper_time_s: 0.0,
            coordinate_time_s: 0.0,
            landed_on: None,
            visited_systems: vec!["Sol".to_string()],
        }
    }
}
