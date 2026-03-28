use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// resource symbol → amount in grams
    pub elements: HashMap<String, f64>,
    pub capacity_g: f64,
    /// Refined fusion pellets stored in cargo (100g He-3 + 100g D each).
    /// Not counted toward cargo mass — they're dense, compacted objects.
    #[serde(default)]
    pub pellets: u32,
}

impl Inventory {
    pub fn new(capacity_g: f64) -> Self {
        Inventory { elements: HashMap::new(), capacity_g, pellets: 0 }
    }

    pub fn total_mass_g(&self) -> f64 {
        self.elements.values().sum()
    }

    pub fn amount(&self, symbol: &str) -> f64 {
        self.elements.get(symbol).copied().unwrap_or(0.0)
    }

    /// Add up to `grams` of a resource. Returns how much was actually added
    /// (may be less than requested if cargo capacity is tight).
    pub fn add(&mut self, symbol: &str, grams: f64) -> f64 {
        let space = self.capacity_g - self.total_mass_g();
        let added = grams.min(space).max(0.0);
        if added > 0.0 {
            *self.elements.entry(symbol.to_string()).or_insert(0.0) += added;
        }
        added
    }

    /// Remove up to `grams` of a resource. Returns how much was actually removed.
    pub fn remove(&mut self, symbol: &str, grams: f64) -> f64 {
        let have = self.amount(symbol);
        let removed = grams.min(have);
        if removed > 0.0 {
            let entry = self.elements.get_mut(symbol).unwrap();
            *entry -= removed;
            if *entry < 1e-6 {
                self.elements.remove(symbol);
            }
        }
        removed
    }
}
