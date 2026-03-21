use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::universe::planet::PlanetType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmosphericComponent {
    pub symbol: String,
    pub name: String,
    pub fraction: f64,
}

fn comp(symbol: &str, name: &str, fraction: f64) -> AtmosphericComponent {
    AtmosphericComponent { symbol: symbol.to_string(), name: name.to_string(), fraction }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atmosphere {
    /// Surface pressure in bar (0 = none)
    pub pressure_bar: f64,
    pub components: Vec<AtmosphericComponent>,
}

impl Atmosphere {
    pub fn none() -> Self {
        Atmosphere { pressure_bar: 0.0, components: vec![] }
    }

    pub fn generate<R: Rng>(rng: &mut R, planet_type: &PlanetType, temp_k: f64, _mass_earth: f64) -> Self {
        match planet_type {
            PlanetType::Barren => {
                if rng.random::<f64>() < 0.3 {
                    Atmosphere {
                        pressure_bar: rng.random_range(0.001..0.01),
                        components: vec![
                            comp("CO₂", "Carbon dioxide", 0.95),
                            comp("N₂",  "Nitrogen",        0.03),
                            comp("Ar",  "Argon",            0.02),
                        ],
                    }
                } else {
                    Atmosphere::none()
                }
            }

            PlanetType::Terrestrial => {
                let pressure = rng.random_range(0.1..3.0);
                if temp_k > 700.0 {
                    Atmosphere {
                        pressure_bar: rng.random_range(50.0..100.0),
                        components: vec![
                            comp("CO₂", "Carbon dioxide", 0.965),
                            comp("N₂",  "Nitrogen",        0.035),
                        ],
                    }
                } else if temp_k > 200.0 && temp_k < 350.0 {
                    let n2  = rng.random_range(0.6..0.85);
                    let o2  = rng.random_range(0.05..0.25);
                    let ar  = rng.random_range(0.005..0.02);
                    let co2 = f64::max(1.0 - n2 - o2 - ar, 0.0);
                    Atmosphere {
                        pressure_bar: pressure,
                        components: vec![
                            comp("N₂",  "Nitrogen",       n2),
                            comp("O₂",  "Oxygen",          o2),
                            comp("Ar",  "Argon",            ar),
                            comp("CO₂", "Carbon dioxide",  co2),
                        ],
                    }
                } else {
                    Atmosphere {
                        pressure_bar: pressure,
                        components: vec![
                            comp("CO₂", "Carbon dioxide", 0.80),
                            comp("N₂",  "Nitrogen",        0.15),
                            comp("CH₄", "Methane",          0.05),
                        ],
                    }
                }
            }

            PlanetType::SuperEarth => {
                let pressure = rng.random_range(1.0..20.0);
                let components = if temp_k > 500.0 {
                    vec![
                        comp("CO₂", "Carbon dioxide", 0.90),
                        comp("SO₂", "Sulfur dioxide",  0.07),
                        comp("N₂",  "Nitrogen",         0.03),
                    ]
                } else {
                    vec![
                        comp("N₂",  "Nitrogen",       0.75),
                        comp("H₂O", "Water vapor",    0.15),
                        comp("CO₂", "Carbon dioxide", 0.10),
                    ]
                };
                Atmosphere { pressure_bar: pressure, components }
            }

            PlanetType::OceanWorld => {
                Atmosphere {
                    pressure_bar: rng.random_range(0.5..5.0),
                    components: vec![
                        comp("N₂",  "Nitrogen",       0.70),
                        comp("H₂O", "Water vapor",    0.20),
                        comp("O₂",  "Oxygen",          0.08),
                        comp("CO₂", "Carbon dioxide", 0.02),
                    ],
                }
            }

            PlanetType::GasGiant | PlanetType::HotJupiter => {
                let h2  = rng.random_range(0.75..0.92);
                let he  = rng.random_range(0.05..0.24);
                let ch4 = f64::max(1.0 - h2 - he, 0.0);
                Atmosphere {
                    pressure_bar: 1000.0,
                    components: vec![
                        comp("H₂",  "Molecular hydrogen", h2),
                        comp("He",  "Helium",              he),
                        comp("CH₄", "Methane",             ch4),
                    ],
                }
            }

            PlanetType::IceGiant => {
                Atmosphere {
                    pressure_bar: 100.0,
                    components: vec![
                        comp("H₂",  "Molecular hydrogen", 0.83),
                        comp("He",  "Helium",              0.15),
                        comp("CH₄", "Methane",             0.02),
                    ],
                }
            }
        }
    }

    pub fn is_breathable(&self) -> bool {
        if self.pressure_bar < 0.5 || self.pressure_bar > 5.0 {
            return false;
        }
        let o2_frac = self.components.iter()
            .find(|c| c.symbol == "O₂")
            .map(|c| c.fraction)
            .unwrap_or(0.0);
        let co2_frac = self.components.iter()
            .find(|c| c.symbol == "CO₂")
            .map(|c| c.fraction)
            .unwrap_or(0.0);
        o2_frac > 0.15 && o2_frac < 0.35 && co2_frac < 0.01
    }
}
