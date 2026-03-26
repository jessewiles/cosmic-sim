#![allow(dead_code)]

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TechSet(HashSet<String>);

impl TechSet {
    pub fn has(&self, id: &str) -> bool {
        self.0.contains(id)
    }

    pub fn unlock(&mut self, id: &str) {
        self.0.insert(id.to_string());
    }

    pub fn all(&self) -> &HashSet<String> {
        &self.0
    }
}

pub struct Invention {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub unlocks: &'static str,
    pub cost_he3_g: f64,
    pub cost_d_g: f64,
    /// Tech ID that must be unlocked first.
    pub requires: Option<&'static str>,
}

pub fn all_inventions() -> Vec<Invention> {
    vec![
        Invention {
            id: "hd_separation",
            name: "HD Centrifuge Array",
            description: "Gas-phase isotopic separation via mass-differential centrifugation.\
 HD is ~1% heavier than H₂ — enough for cascade separation across a column.",
            unlocks: "Deuterium extraction from gas giant and hot Jupiter atmospheres",
            cost_he3_g: 300.0,
            cost_d_g: 200.0,
            requires: None,
        },
        Invention {
            id: "cryo_drill",
            name: "Subsurface Cryo-Drill",
            description: "Thermal-lance drilling to subsurface ice on ultra-cold bodies.\
 Expands mining on frigid barren worlds (below 80 K) that surface scrapers can't reach.",
            unlocks: "Mining on barren worlds below 80 K",
            cost_he3_g: 0.0,
            cost_d_g: 400.0,
            requires: None,
        },
        Invention {
            id: "electrolysis_boost",
            name: "High-Yield Electrolytic Cell",
            description: "Improved electrode geometry and catalyst loading.\
 Doubles the D₂O extraction rate from liquid water — more deuterium per tonne of ocean processed.",
            unlocks: "2× deuterium yield from ocean worlds",
            cost_he3_g: 0.0,
            cost_d_g: 600.0,
            requires: None,
        },
        Invention {
            id: "ism_scoop",
            name: "Interstellar Medium Scoop",
            description: "Magnetohydrodynamic ram-scoop tuned to ISM hydrogen density.\
 Collects trace He-3 and D during transit — slowly, but without landing.",
            unlocks: "Passive resource collection added to each interstellar journey",
            cost_he3_g: 2000.0,
            cost_d_g: 500.0,
            requires: Some("hd_separation"),
        },
    ]
}
