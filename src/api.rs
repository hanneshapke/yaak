use serde::{Deserialize, Serialize};

// --- OpenAI-compatible structs ---

#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub stream: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// Non-streaming response structs (retained for tests)
#[cfg(test)]
#[derive(Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[cfg(test)]
#[derive(Deserialize)]
pub struct Choice {
    pub message: MessageContent,
}

#[cfg(test)]
#[derive(Deserialize)]
pub struct MessageContent {
    pub content: String,
}

// --- Anthropic structs ---

#[derive(Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub system: String,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stream: bool,
}

// --- Gemini structs ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    pub system_instruction: GeminiSystemInstruction,
    pub generation_config: GeminiGenerationConfig,
}

#[derive(Serialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
pub struct GeminiSystemInstruction {
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Serialize)]
pub struct GeminiGenerationConfig {
    pub temperature: f32,
}

// Non-streaming response structs (retained for tests)
#[cfg(test)]
#[derive(Deserialize)]
pub struct AnthropicResponse {
    pub content: Vec<AnthropicContent>,
}

#[cfg(test)]
#[derive(Deserialize)]
pub struct AnthropicContent {
    pub text: String,
}

/// Returns true if the API base URL points to Anthropic's API.
pub fn is_anthropic(api_base: &str) -> bool {
    api_base.contains("anthropic.com")
}

/// Returns true if the API base URL points to Google's Gemini API.
pub fn is_gemini(api_base: &str) -> bool {
    api_base.contains("generativelanguage.googleapis.com")
}

// --- API key verification ---

/// A minimal, non-streaming request used to check that an API key works.
pub struct VerifyRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: serde_json::Value,
}

/// Build the smallest possible completion request for the provider behind `api_base`,
/// used by the config wizard to verify an API key before writing it to disk.
pub fn build_verify_request(api_base: &str, model: &str, api_key: &str) -> VerifyRequest {
    let base = api_base.trim_end_matches('/');

    if is_anthropic(api_base) {
        VerifyRequest {
            url: format!("{}/messages", base),
            headers: vec![
                ("x-api-key".into(), api_key.to_string()),
                ("anthropic-version".into(), "2023-06-01".into()),
            ],
            body: serde_json::json!({
                "model": model,
                "max_tokens": 1,
                "messages": [{ "role": "user", "content": "hi" }],
            }),
        }
    } else if is_gemini(api_base) {
        // Gemini takes the key as a query parameter rather than a header.
        VerifyRequest {
            url: format!("{}/models/{}:generateContent?key={}", base, model, api_key),
            headers: Vec::new(),
            body: serde_json::json!({
                "contents": [{ "role": "user", "parts": [{ "text": "hi" }] }],
                "generationConfig": { "maxOutputTokens": 1 },
            }),
        }
    } else {
        VerifyRequest {
            url: format!("{}/chat/completions", base),
            headers: vec![("Authorization".into(), format!("Bearer {}", api_key))],
            body: serde_json::json!({
                "model": model,
                "max_tokens": 1,
                "messages": [{ "role": "user", "content": "hi" }],
            }),
        }
    }
}

/// The outcome of an API key verification attempt.
#[derive(Debug, PartialEq, Eq)]
pub enum KeyCheck {
    /// The provider accepted the key.
    Valid,
    /// The provider rejected the key (HTTP 401/403).
    Rejected(u16),
    /// The key authenticated but the model or base URL was not found (HTTP 404).
    ModelMissing(u16),
    /// The result is inconclusive — a server error, or a status we don't recognise.
    Unknown(String),
}

/// Interpret an HTTP status from a verification request.
///
/// A 429 counts as valid: the provider rate-limited us, which means it
/// authenticated the key first.
pub fn classify_verify_status(status: u16) -> KeyCheck {
    match status {
        200..=299 | 429 => KeyCheck::Valid,
        401 | 403 => KeyCheck::Rejected(status),
        404 => KeyCheck::ModelMissing(status),
        other => KeyCheck::Unknown(format!("HTTP {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_anthropic_provider() {
        assert!(is_anthropic("https://api.anthropic.com/v1"));
        assert!(is_anthropic("https://api.anthropic.com"));
        assert!(!is_anthropic("https://api.openai.com/v1"));
        assert!(!is_anthropic("http://localhost:11434/v1"));
        assert!(!is_anthropic("http://localhost:1234/v1"));
        assert!(!is_anthropic("https://api.together.xyz/v1"));
    }

    #[test]
    fn anthropic_request_serializes_correctly() {
        let req = AnthropicRequest {
            model: "claude-sonnet-4-6".into(),
            system: "You are helpful.".into(),
            messages: vec![Message {
                role: "user".into(),
                content: "list files".into(),
            }],
            max_tokens: 1024,
            temperature: 0.0,
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-6");
        assert_eq!(json["system"], "You are helpful.");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["messages"].as_array().unwrap().len(), 1);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "list files");
    }

    #[test]
    fn anthropic_response_deserializes_correctly() {
        let json = r#"{"content":[{"type":"text","text":"ls -la"}]}"#;
        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content[0].text, "ls -la");
    }

    #[test]
    fn anthropic_response_multiple_blocks() {
        let json =
            r#"{"content":[{"type":"text","text":"first"},{"type":"text","text":"second"}]}"#;
        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content.len(), 2);
        assert_eq!(resp.content[0].text, "first");
        assert_eq!(resp.content[1].text, "second");
    }

    #[test]
    fn detects_gemini_provider() {
        assert!(is_gemini(
            "https://generativelanguage.googleapis.com/v1beta"
        ));
        assert!(!is_gemini("https://api.openai.com/v1"));
        assert!(!is_gemini("https://api.anthropic.com/v1"));
        assert!(!is_gemini("http://localhost:11434/v1"));
    }

    #[test]
    fn gemini_request_serializes_correctly() {
        let req = GeminiRequest {
            contents: vec![GeminiContent {
                role: "user".into(),
                parts: vec![GeminiPart {
                    text: "list files".into(),
                }],
            }],
            system_instruction: GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: "You are helpful.".into(),
                }],
            },
            generation_config: GeminiGenerationConfig { temperature: 0.0 },
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["contents"][0]["role"], "user");
        assert_eq!(json["contents"][0]["parts"][0]["text"], "list files");
        assert_eq!(
            json["systemInstruction"]["parts"][0]["text"],
            "You are helpful."
        );
        assert_eq!(json["generationConfig"]["temperature"], 0.0);
    }

    #[test]
    fn openai_request_serializes_correctly() {
        let req = ChatRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![
                Message {
                    role: "system".into(),
                    content: "You are helpful.".into(),
                },
                Message {
                    role: "user".into(),
                    content: "list files".into(),
                },
            ],
            temperature: 0.0,
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "gpt-4o-mini");
        assert_eq!(json["messages"].as_array().unwrap().len(), 2);
        assert_eq!(json["messages"][0]["role"], "system");
        assert!(json.get("system").is_none());
    }

    #[test]
    fn openai_response_deserializes_correctly() {
        let json = r#"{"choices":[{"message":{"content":"ls -la"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ls -la");
    }

    fn header<'a>(req: &'a VerifyRequest, name: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn verify_request_anthropic_shape() {
        let req = build_verify_request("https://api.anthropic.com/v1", "claude-sonnet-4-6", "sk-1");
        assert_eq!(req.url, "https://api.anthropic.com/v1/messages");
        assert_eq!(header(&req, "x-api-key"), Some("sk-1"));
        assert_eq!(header(&req, "anthropic-version"), Some("2023-06-01"));
        assert!(header(&req, "Authorization").is_none());
        assert_eq!(req.body["model"], "claude-sonnet-4-6");
        assert_eq!(req.body["max_tokens"], 1);
    }

    #[test]
    fn verify_request_gemini_shape() {
        let req = build_verify_request(
            "https://generativelanguage.googleapis.com/v1beta",
            "gemini-2.5-flash",
            "sk-2",
        );
        assert_eq!(
            req.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key=sk-2"
        );
        // Gemini authenticates via the query string, so no auth header.
        assert!(req.headers.is_empty());
        assert_eq!(req.body["contents"][0]["parts"][0]["text"], "hi");
    }

    #[test]
    fn verify_request_openai_shape() {
        let req = build_verify_request("https://api.openai.com/v1", "gpt-4o-mini", "sk-3");
        assert_eq!(req.url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(header(&req, "Authorization"), Some("Bearer sk-3"));
        assert_eq!(req.body["model"], "gpt-4o-mini");
    }

    #[test]
    fn verify_request_trims_trailing_slash() {
        let req = build_verify_request("https://api.openai.com/v1/", "gpt-4o-mini", "sk-4");
        assert_eq!(req.url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn classify_verify_status_maps_outcomes() {
        assert_eq!(classify_verify_status(200), KeyCheck::Valid);
        assert_eq!(classify_verify_status(204), KeyCheck::Valid);
        // Rate limited means the key authenticated first.
        assert_eq!(classify_verify_status(429), KeyCheck::Valid);
        assert_eq!(classify_verify_status(401), KeyCheck::Rejected(401));
        assert_eq!(classify_verify_status(403), KeyCheck::Rejected(403));
        assert_eq!(classify_verify_status(404), KeyCheck::ModelMissing(404));
        assert_eq!(
            classify_verify_status(500),
            KeyCheck::Unknown("HTTP 500".into())
        );
    }
}
