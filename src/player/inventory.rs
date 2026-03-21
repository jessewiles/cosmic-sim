use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// element symbol → amount in grams
    pub elements: HashMap<String, f64>,
    pub capacity_g: f64,
}

impl Inventory {
    pub fn new(capacity_g: f64) -> Self {
        Inventory { elements: HashMap::new(), capacity_g }
    }

    pub fn total_mass_g(&self) -> f64 {
        self.elements.values().sum()
    }

    pub fn add(&mut self, symbol: &str, grams: f64) -> bool {
        let current = self.total_mass_g();
        if current + grams > self.capacity_g {
            return false;
        }
        *self.elements.entry(symbol.to_string()).or_insert(0.0) += grams;
        true
    }
}
