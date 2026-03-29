use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use crate::player::state::PlayerState;
use crate::player::ship::Ship;
use crate::player::inventory::Inventory;
use crate::player::tech::TechSet;
use crate::universe::galaxy::{Galaxy, GalaxyMode};
use crate::universe::system::StarSystem;

// ── Paths ────────────────────────────────────────────────────────────────────

pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cosmic-sim")
}

fn saves_dir() -> PathBuf {
    data_dir().join("saves")
}

pub fn api_key_path() -> PathBuf {
    data_dir().join("api_key")
}

fn ensure_dirs() -> io::Result<()> {
    fs::create_dir_all(saves_dir())
}

// ── SavedGame ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedGame {
    /// Slot name — typically the player's name or a custom label.
    pub slot: String,
    /// Unix timestamp of when this save was written.
    pub saved_at: u64,
    pub player: PlayerState,
    pub ship: Ship,
    pub inventory: Inventory,
    pub galaxy: Galaxy,
    pub current_system: StarSystem,
    #[serde(default)]
    pub tech: TechSet,
    #[serde(default)]
    pub inbox: Vec<crate::CompanionMessage>,
    #[serde(default)]
    pub triggers_fired: std::collections::HashSet<String>,
    #[serde(default)]
    pub objective: Option<String>,
}

impl SavedGame {
    pub fn new(
        slot: String,
        player: PlayerState,
        ship: Ship,
        inventory: Inventory,
        galaxy: Galaxy,
        current_system: StarSystem,
        tech: TechSet,
        inbox: Vec<crate::CompanionMessage>,
        triggers_fired: std::collections::HashSet<String>,
        objective: Option<String>,
    ) -> Self {
        let saved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        SavedGame { slot, saved_at, player, ship, inventory, galaxy, current_system, tech, inbox, triggers_fired, objective }
    }

    /// Human-readable timestamp, e.g. "2024-03-21 14:05".
    pub fn timestamp_display(&self) -> String {
        let secs = self.saved_at;
        // Manual UTC decomposition (no chrono dep)
        let s = secs % 60;
        let m = (secs / 60) % 60;
        let h = (secs / 3600) % 24;
        let days = secs / 86400;
        // Days since 1970-01-01 → approximate calendar date
        let (y, mo, d) = days_to_ymd(days);
        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, mo, d, h, m, s)
    }
}

/// Rough but correct Gregorian calendar conversion for UTC dates.
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut d = days + 719468; // shift to era starting 0000-03-01
    let era = d / 146097;
    d %= 146097;
    let yoe = (d - d/1460 + d/36524 - d/146096) / 365;
    let y = yoe + era * 400;
    let doy = d - (365*yoe + yoe/4 - yoe/100);
    let mp = (5*doy + 2) / 153;
    let dom = doy - (153*mp + 2)/5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, dom)
}

// ── Save / Load ───────────────────────────────────────────────────────────────

pub fn save(game: &SavedGame) -> io::Result<()> {
    ensure_dirs()?;
    let path = saves_dir().join(format!("{}.json", sanitize_slot(&game.slot)));
    let json = serde_json::to_string_pretty(game)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(path, json)
}

#[allow(dead_code)]
pub fn load(slot: &str) -> io::Result<SavedGame> {
    let path = saves_dir().join(format!("{}.json", sanitize_slot(slot)));
    let json = fs::read_to_string(&path)?;
    serde_json::from_str(&json)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// List all save files, sorted newest first.
pub fn list_saves() -> Vec<SavedGame> {
    let dir = saves_dir();
    if !dir.exists() { return vec![]; }

    let mut saves: Vec<SavedGame> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| {
            let json = fs::read_to_string(e.path()).ok()?;
            serde_json::from_str(&json).ok()
        })
        .collect();

    saves.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
    saves
}

fn sanitize_slot(slot: &str) -> String {
    slot.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

// ── API key persistence ───────────────────────────────────────────────────────

/// Load stored API key from ~/.cosmos/api_key (trimmed).
pub fn load_api_key() -> Option<String> {
    fs::read_to_string(api_key_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Write an API key to ~/.cosmos/api_key.
pub fn store_api_key(key: &str) -> io::Result<()> {
    ensure_dirs()?;
    // Restrict permissions to owner-only on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true).mode(0o600);
        use io::Write;
        opts.open(api_key_path())?.write_all(key.trim().as_bytes())?;
        return Ok(());
    }
    #[cfg(not(unix))]
    fs::write(api_key_path(), key.trim().as_bytes())
}

/// Rebuild a Galaxy from a SavedGame (re-attaches methods; known_systems cache preserved).
pub fn galaxy_from_save(saved: &SavedGame) -> Galaxy {
    Galaxy {
        name: saved.galaxy.name.clone(),
        seed: saved.galaxy.seed,
        mode: saved.galaxy.mode,
        known_systems: saved.galaxy.known_systems.clone(),
    }
}

/// Build a GalaxyMode label for display.
pub fn mode_label(mode: GalaxyMode) -> &'static str {
    match mode {
        GalaxyMode::RealUniverse => "Real Universe",
        GalaxyMode::Procedural   => "Procedural",
    }
}
