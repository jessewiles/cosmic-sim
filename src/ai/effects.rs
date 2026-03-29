use serde::Deserialize;

/// A game-world action that an LLM can emit alongside its narrative text.
/// Encoded in responses as `<effect>{JSON}</effect>` tags.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEffect {
    /// Set or replace the player's current mission objective.
    SetObjective { text: String },
    /// Transfer He-3 or D between a companion's ship and the player.
    TransferResource { resource: String, amount_g: f64, to_player: bool },
    /// Transfer fuel units between a companion's ship and the player.
    TransferFuel { amount: f64, to_player: bool },
    /// Transfer refined pellets between a companion's cargo and the player.
    TransferPellets { count: u32, to_player: bool },
    /// Permanently unlock a named game feature (e.g. "atmo_scoop").
    UnlockFeature { feature: String },
}

/// Parse `<effect>…</effect>` tags out of an LLM response.
/// Returns the cleaned text (tags removed) and a list of parsed effects.
/// Malformed or unrecognised effect JSON is silently dropped.
pub fn parse_effects(text: &str) -> (String, Vec<GameEffect>) {
    let mut effects = Vec::new();
    let mut cleaned = String::new();
    let mut rest = text;

    while let Some(start) = rest.find("<effect>") {
        cleaned.push_str(&rest[..start]);
        let after_open = &rest[start + "<effect>".len()..];
        if let Some(end) = after_open.find("</effect>") {
            let json = after_open[..end].trim();
            if let Ok(effect) = serde_json::from_str::<GameEffect>(json) {
                effects.push(effect);
            }
            rest = &after_open[end + "</effect>".len()..];
        } else {
            // Unclosed tag — pass through unchanged and stop scanning
            cleaned.push_str(&rest[start..]);
            rest = "";
        }
    }
    cleaned.push_str(rest);
    (cleaned.trim_end().to_string(), effects)
}

/// One-line description of an effect for the player notification bar.
pub fn describe_effect(effect: &GameEffect, source: &str) -> String {
    match effect {
        GameEffect::SetObjective { text } => {
            format!("Objective → {}", text)
        }
        GameEffect::TransferResource { resource, amount_g, to_player } => {
            if *to_player {
                format!("{} → +{:.0}g {} in your cargo", source, amount_g, resource)
            } else {
                format!("{} ← {:.0}g {} drawn from your cargo", source, amount_g, resource)
            }
        }
        GameEffect::TransferFuel { amount, to_player } => {
            if *to_player {
                format!("{} → +{:.1} fuel", source, amount)
            } else {
                format!("{} ← {:.1} fuel drawn", source, amount)
            }
        }
        GameEffect::TransferPellets { count, to_player } => {
            if *to_player {
                format!("{} → +{} pellet{}", source, count, if *count == 1 { "" } else { "s" })
            } else {
                format!("{} ← {} pellet{} drawn", source, count, if *count == 1 { "" } else { "s" })
            }
        }
        GameEffect::UnlockFeature { feature } => {
            match feature.as_str() {
                "atmo_scoop" => "Atmospheric scooping unlocked — orbital passes on gas/ice giants now available".to_string(),
                other        => format!("Capability unlocked: {}", other),
            }
        }
    }
}

/// The system prompt instructions injected so LLMs know the effect grammar.
/// `source_role` is either "companion" or "ARIA" to tailor available effects.
pub fn effect_instructions(source_role: &str) -> String {
    let available = if source_role == "companion" {
        r#"  {"type": "set_objective", "text": "concise objective description"}
  {"type": "transfer_resource", "resource": "He-3" or "D", "amount_g": <float>, "to_player": <bool>}
  {"type": "transfer_fuel", "amount": <float>, "to_player": <bool>}
  {"type": "transfer_pellets", "count": <int>, "to_player": <bool>}
  {"type": "unlock_feature", "feature": "atmo_scoop"}

Unlockable features you know about:
  atmo_scoop — orbital atmospheric scooping on gas giants and ice giants for He-3/D
    collection without landing. Bypasses Extreme surface risk. Emit this unlock if the
    player asks about alternative fuel sources, seems stuck for fuel, or the topic
    comes up naturally. Do not force it; let it arise from the conversation."#
    } else {
        // ARIA is onboard — can only set objectives, not transfer external resources
        r#"  {"type": "set_objective", "text": "concise objective description"}"#
    };

    format!(
        "\n\nGAME EFFECTS\n\
If your response commits to a concrete game-world action, append a single effect tag \
on its own line at the very end of your message — after all narrative text:\n\
  <effect>{{...}}</effect>\n\n\
Available types:\n\
{available}\n\n\
Rules:\n\
- Only emit an effect when your narrative explicitly describes that action as happening now\n\
- \"to_player: true\" means resources flow to the player; false means the reverse\n\
- Do not manufacture reasons to trigger effects; most messages need none\n\
- You cannot give resources you wouldn't plausibly have",
        available = available
    )
}
