use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Maximum number of back-and-forth exchanges kept in context.
/// Each exchange = 1 user message + 1 assistant message.
const MAX_EXCHANGES: usize = 20;

// ── API request / response types ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ApiRequest {
    model: &'static str,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

// ── ShipComputer ─────────────────────────────────────────────────────────────

pub struct ShipComputer {
    history: Vec<Message>,
    client: reqwest::blocking::Client,
}

impl ShipComputer {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");
        ShipComputer { history: vec![], client }
    }

    /// Send a message to Claude. Returns the assistant's reply or an error string.
    /// The system prompt is rebuilt each call so it always reflects current game state.
    pub fn ask(&mut self, user_message: &str, system_prompt: &str) -> Result<String, String> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY is not set".to_string())?;

        self.history.push(Message {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        // Trim history: keep at most MAX_EXCHANGES exchanges (pairs of messages)
        let max_msgs = MAX_EXCHANGES * 2;
        if self.history.len() > max_msgs {
            self.history.drain(..self.history.len() - max_msgs);
        }

        let request_body = ApiRequest {
            model: "claude-opus-4-6",
            max_tokens: 1024,
            system: system_prompt.to_string(),
            messages: self.history.clone(),
        };

        let body = serde_json::to_string(&request_body)
            .map_err(|e| format!("Serialization error: {}", e))?;

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body)
            .send()
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status();
        let text = response.text()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if !status.is_success() {
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("HTTP {}", status));
            // Remove the user message we just added since it failed
            self.history.pop();
            return Err(msg);
        }

        let parsed: ApiResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {} (body: {})", e, &text[..text.len().min(200)]))?;

        let content = parsed.content.iter()
            .find(|b| b.block_type == "text")
            .and_then(|b| b.text.clone())
            .ok_or_else(|| "No text content in response".to_string())?;

        self.history.push(Message {
            role: "assistant".to_string(),
            content: content.clone(),
        });

        Ok(content)
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn exchange_count(&self) -> usize {
        self.history.len() / 2
    }
}
