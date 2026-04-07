use serde::Deserialize;
use std::io::{BufRead, BufReader, Read};

/// Extract text tokens from an SSE stream (Server-Sent Events).
/// Works with both OpenAI-compatible and Anthropic streaming formats.
/// Calls `on_token` for each text delta received.
pub fn stream_tokens<R: Read>(reader: R, anthropic: bool, mut on_token: impl FnMut(&str)) {
    let buf = BufReader::new(reader);

    for line in buf.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let Some(data) = line.strip_prefix("data: ") else {
            continue;
        };

        // OpenAI signals end with [DONE]
        if data == "[DONE]" {
            break;
        }

        if anthropic {
            if let Some(token) = parse_anthropic_delta(data) {
                on_token(&token);
            }
        } else if let Some(token) = parse_openai_delta(data) {
            on_token(&token);
        }
    }
}

// --- OpenAI streaming delta ---

#[derive(Deserialize)]
struct OpenAiChunk {
    choices: Vec<OpenAiChunkChoice>,
}

#[derive(Deserialize)]
struct OpenAiChunkChoice {
    delta: OpenAiDelta,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}

fn parse_openai_delta(data: &str) -> Option<String> {
    let chunk: OpenAiChunk = serde_json::from_str(data).ok()?;
    chunk.choices.first()?.delta.content.clone()
}

// --- Anthropic streaming delta ---

#[derive(Deserialize)]
struct AnthropicEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicDelta>,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
}

fn parse_anthropic_delta(data: &str) -> Option<String> {
    let event: AnthropicEvent = serde_json::from_str(data).ok()?;
    if event.event_type != "content_block_delta" {
        return None;
    }
    let delta = event.delta?;
    if delta.delta_type.as_deref() != Some("text_delta") {
        return None;
    }
    delta.text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_delta_extracts_content() {
        let data = r#"{"id":"x","choices":[{"index":0,"delta":{"content":"hello"}}]}"#;
        assert_eq!(parse_openai_delta(data), Some("hello".into()));
    }

    #[test]
    fn parse_openai_delta_empty_delta() {
        // First chunk often has role but no content
        let data = r#"{"id":"x","choices":[{"index":0,"delta":{"role":"assistant"}}]}"#;
        assert_eq!(parse_openai_delta(data), None);
    }

    #[test]
    fn parse_anthropic_delta_extracts_text() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"world"}}"#;
        assert_eq!(parse_anthropic_delta(data), Some("world".into()));
    }

    #[test]
    fn parse_anthropic_delta_ignores_other_events() {
        let data = r#"{"type":"message_start","message":{"id":"x"}}"#;
        assert_eq!(parse_anthropic_delta(data), None);
    }

    #[test]
    fn stream_tokens_openai_format() {
        let input = "\
data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"}}]}\n\
\n\
data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ls\"}}]}\n\
\n\
data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" -la\"}}]}\n\
\n\
data: [DONE]\n";

        let mut collected = String::new();
        stream_tokens(input.as_bytes(), false, |t| collected.push_str(t));
        assert_eq!(collected, "ls -la");
    }

    #[test]
    fn stream_tokens_anthropic_format() {
        let input = "\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"x\"}}\n\
\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"find\"}}\n\
\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" . -name\"}}\n\
\n\
data: {\"type\":\"message_stop\"}\n";

        let mut collected = String::new();
        stream_tokens(input.as_bytes(), true, |t| collected.push_str(t));
        assert_eq!(collected, "find . -name");
    }

    #[test]
    fn stream_tokens_handles_empty_input() {
        let mut collected = String::new();
        stream_tokens("".as_bytes(), false, |t| collected.push_str(t));
        assert_eq!(collected, "");
    }
}
