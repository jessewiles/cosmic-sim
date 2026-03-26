use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

/// Exchanges sent to the API per call. Full history is stored on disk.
const MAX_CONTEXT_EXCHANGES: usize = 6;

// ── API request / response types ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ApiRequest {
    model: &'static str,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
    stream: bool,
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
    /// Sliding context window sent to the API (capped at MAX_CONTEXT_EXCHANGES).
    history: Vec<Message>,
    /// Complete exchange log — persisted to disk, never trimmed in memory.
    full_log: Vec<Message>,
    /// Where the log is written. None for ephemeral instances (e.g. ARIA).
    log_path: Option<PathBuf>,
    client: reqwest::blocking::Client,
}

impl ShipComputer {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");
        ShipComputer { history: vec![], full_log: vec![], log_path: None, client }
    }

    /// Create a computer backed by a persistent log file.
    /// If the file exists, the recent exchanges are loaded into the context window.
    pub fn with_log(path: PathBuf) -> Self {
        let mut computer = Self::new();
        computer.log_path = Some(path.clone());

        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(log) = serde_json::from_str::<Vec<Message>>(&data) {
                computer.full_log = log;
                let start = computer.full_log.len()
                    .saturating_sub(MAX_CONTEXT_EXCHANGES * 2);
                computer.history = computer.full_log[start..].to_vec();
            }
        }
        computer
    }

    fn save_log(&self) {
        let Some(path) = &self.log_path else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.full_log) {
            let _ = std::fs::write(path, json);
        }
    }

    /// All messages ever exchanged, oldest first.
    pub fn full_log(&self) -> &[Message] {
        &self.full_log
    }

    /// The last `n` exchanges (2*n messages) from the full log.
    pub fn recent_log(&self, n_exchanges: usize) -> &[Message] {
        let start = self.full_log.len().saturating_sub(n_exchanges * 2);
        &self.full_log[start..]
    }

    /// Number of complete exchanges (user + assistant pairs) in the full log.
    pub fn exchange_count(&self) -> usize {
        self.full_log.len() / 2
    }

    /// Send a message to Claude. Returns the assistant's reply or an error string.
    /// The system prompt is rebuilt each call so it always reflects current game state.
    pub fn ask(&mut self, user_message: &str, system_prompt: &str) -> Result<String, String> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY is not set".to_string())?;

        let user_msg = Message { role: "user".to_string(), content: user_message.to_string() };
        self.history.push(user_msg.clone());
        self.full_log.push(user_msg);

        let max_msgs = MAX_CONTEXT_EXCHANGES * 2;
        if self.history.len() > max_msgs {
            self.history.drain(..self.history.len() - max_msgs);
        }

        let request_body = ApiRequest {
            model: "claude-opus-4-6",
            max_tokens: 1024,
            system: system_prompt.to_string(),
            messages: self.history.clone(),
            stream: false,
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
            self.history.pop();
            self.full_log.pop();
            return Err(msg);
        }

        let parsed: ApiResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {} (body: {})", e, &text[..text.len().min(200)]))?;

        let content = parsed.content.iter()
            .find(|b| b.block_type == "text")
            .and_then(|b| b.text.clone())
            .ok_or_else(|| "No text content in response".to_string())?;

        let asst_msg = Message { role: "assistant".to_string(), content: content.clone() };
        self.history.push(asst_msg.clone());
        self.full_log.push(asst_msg);
        self.save_log();

        Ok(content)
    }

    /// Send a message with streaming. Calls `on_delta` for each text chunk as
    /// it arrives. Returns the complete response text when done.
    pub fn ask_streaming<F>(
        &mut self,
        user_message: &str,
        system_prompt: &str,
        mut on_delta: F,
    ) -> Result<String, String>
    where
        F: FnMut(&str),
    {
        use std::io::{BufRead, BufReader};

        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY is not set".to_string())?;

        let user_msg = Message { role: "user".to_string(), content: user_message.to_string() };
        self.history.push(user_msg.clone());
        self.full_log.push(user_msg);

        let max_msgs = MAX_CONTEXT_EXCHANGES * 2;
        if self.history.len() > max_msgs {
            self.history.drain(..self.history.len() - max_msgs);
        }

        let request_body = ApiRequest {
            model: "claude-opus-4-6",
            max_tokens: 1024,
            system: system_prompt.to_string(),
            messages: self.history.clone(),
            stream: true,
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
        if !status.is_success() {
            let text = response.text().unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("HTTP {}", status));
            self.history.pop();
            self.full_log.pop();
            return Err(msg);
        }

        let mut full_text = String::new();
        let reader = BufReader::new(response);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Stream read error: {}", e))?;

            let Some(data) = line.strip_prefix("data: ") else { continue };
            if data == "[DONE]" { break; }

            let Ok(val) = serde_json::from_str::<serde_json::Value>(data) else { continue };

            if val["type"] == "content_block_delta" && val["delta"]["type"] == "text_delta" {
                if let Some(chunk) = val["delta"]["text"].as_str() {
                    full_text.push_str(chunk);
                    on_delta(chunk);
                }
            }
        }

        let asst_msg = Message { role: "assistant".to_string(), content: full_text.clone() };
        self.history.push(asst_msg.clone());
        self.full_log.push(asst_msg);
        self.save_log();

        Ok(full_text)
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.full_log.clear();
        self.save_log();
    }
}
