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
}
