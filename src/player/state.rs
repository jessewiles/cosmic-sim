use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub name: String,
    /// Current galactic position in light-years
    pub position: [i32; 3],
    /// Ship velocity as fraction of c (0.0 to 1.0)
    pub velocity_c: f64,
    /// Proper time elapsed for player (accounting for dilation)
    pub proper_time_s: f64,
    /// Coordinate time elapsed in the reference frame of the galaxy
    pub coordinate_time_s: f64,
    /// Current planet index if landed, None if in space
    pub landed_on: Option<usize>,
    pub visited_systems: Vec<[i32; 3]>,
}

impl PlayerState {
    pub fn new(name: String) -> Self {
        PlayerState {
            name,
            position: [0, 0, 0],
            velocity_c: 0.0,
            proper_time_s: 0.0,
            coordinate_time_s: 0.0,
            landed_on: None,
            visited_systems: vec![[0, 0, 0]],
        }
    }
}
