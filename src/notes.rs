use std::env;
use std::time::Duration;
use serde::Serialize;

#[derive(Serialize)]
struct IngestPayload<'a> {
    body: &'a str,
    source: &'a str,
    tags: Vec<&'a str>,
}

/// Post a note to study-tools via the ingest endpoint.
/// Returns Ok(()) on success, Err(message) on failure.
pub fn send_note(body: &str, source: &str) -> Result<(), String> {
    let base_url = env::var("STUDY_TOOLS_URL")
        .map_err(|_| "STUDY_TOOLS_URL is not set".to_string())?;
    let api_key = env::var("STUDY_TOOLS_KEY")
        .map_err(|_| "STUDY_TOOLS_KEY is not set".to_string())?;

    let url = format!("{}/api/notes/ingest", base_url.trim_end_matches('/'));

    let payload = IngestPayload {
        body,
        source,
        tags: vec!["cosmic-sim"],
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .post(&url)
        .bearer_auth(&api_key)
        .json(&payload)
        .send()
        .map_err(|e| format!("Request failed: {e}"))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("Server returned {}", resp.status()))
    }
}

/// True if study-tools ingest is configured (env vars present).
pub fn is_configured() -> bool {
    env::var("STUDY_TOOLS_URL").is_ok() && env::var("STUDY_TOOLS_KEY").is_ok()
}
