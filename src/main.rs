use clap::Parser;
use colored::Colorize;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;

/// yaak — translate natural language into bash commands using an OpenAI-compatible LLM
#[derive(Parser, Debug)]
#[command(name = "yaak", version, about)]
struct Args {
    /// The natural language description of the command you want
    #[arg(trailing_var_arg = true, required = true)]
    description: Vec<String>,

    /// API base URL (overrides YAAK_API_BASE / config)
    #[arg(short = 'u', long, env = "YAAK_API_BASE")]
    api_base: Option<String>,

    /// API key (overrides YAAK_API_KEY / config)
    #[arg(short = 'k', long, env = "YAAK_API_KEY")]
    api_key: Option<String>,

    /// Model name (overrides YAAK_MODEL / config)
    #[arg(short = 'm', long, env = "YAAK_MODEL")]
    model: Option<String>,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    yes: bool,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Deserialize, Default)]
struct Config {
    api_base: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
}

fn load_config() -> Config {
    let mut candidates = Vec::new();
    // XDG-style ~/.config (works on all platforms)
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".config").join("yaak").join("config.toml"));
    }
    // Platform-native config dir (e.g. ~/Library/Application Support on macOS)
    if let Some(config_dir) = dirs::config_dir() {
        candidates.push(config_dir.join("yaak").join("config.toml"));
    }
    for path in candidates {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&contents) {
                return config;
            }
        }
    }
    Config::default()
}

fn resolve(cli: Option<String>, config: Option<String>, fallback: &str) -> String {
    cli.or(config).unwrap_or_else(|| fallback.to_string())
}

fn extract_command(raw: &str) -> String {
    let trimmed = raw.trim();

    // Try to extract from ```bash ... ``` or ``` ... ``` fenced blocks
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        // Skip optional language tag on the same line
        let code_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let code_body = &after_fence[code_start..];
        if let Some(end) = code_body.find("```") {
            return code_body[..end].trim().to_string();
        }
    }

    // Otherwise strip any leading "$ " and return as-is
    if let Some(stripped) = trimmed.strip_prefix("$ ") {
        return stripped.to_string();
    }

    trimmed.to_string()
}

fn main() {
    let args = Args::parse();
    let config = load_config();

    let api_base = resolve(args.api_base, config.api_base, "https://api.openai.com/v1");
    let api_key = resolve(args.api_key, config.api_key, "");
    let model = resolve(args.model, config.model, "gpt-4o-mini");

    if api_key.is_empty() {
        eprintln!(
            "{} No API key found. Set YAAK_API_KEY, pass --api-key, or add it to ~/.config/yaak/config.toml",
            "error:".red().bold()
        );
        std::process::exit(1);
    }

    let description = args.description.join(" ");
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let os_name = env::consts::OS;
    let shell = env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let shell_name = shell.rsplit('/').next().unwrap_or("bash");

    let system_prompt = format!(
        "You are a command-line assistant. The user is running {} on {}. \
         The user will describe what they want to do \
         and you must respond with ONLY the exact shell command to accomplish it. \
         No explanation, no markdown fences, no commentary — just the raw command. \
         Only use flags and tools available on {}. \
         If multiple commands are needed, join them with && or ;. \
         Use common, portable tools when possible.",
        shell_name, os_name, os_name
    );

    let request_body = ChatRequest {
        model: model.clone(),
        messages: vec![
            Message {
                role: "system".into(),
                content: system_prompt,
            },
            Message {
                role: "user".into(),
                content: description.clone(),
            },
        ],
        temperature: 0.0,
    };

    // --- Call the LLM ---
    eprint!("{}", "Thinking... ".dimmed());

    let client = reqwest::blocking::Client::new();
    let response = match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("\n{} Failed to reach API: {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        eprintln!(
            "\n{} API returned {} — {}",
            "error:".red().bold(),
            status,
            body
        );
        std::process::exit(1);
    }

    let chat_resp: ChatResponse = match response.json() {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "\n{} Failed to parse response: {}",
                "error:".red().bold(),
                e
            );
            std::process::exit(1);
        }
    };

    let raw_content = &chat_resp.choices[0].message.content;
    let command = extract_command(raw_content);

    // --- Display and confirm ---
    eprintln!("\r{}{}", "  Command: ".bold(), command.green().bold());

    let should_run = args.yes
        || Confirm::new()
            .with_prompt("Execute?")
            .default(true)
            .interact()
            .unwrap_or(false);

    if !should_run {
        eprintln!("{}", "Aborted.".dimmed());
        std::process::exit(0);
    }

    // --- Execute ---
    let status = Command::new(&shell).arg("-c").arg(&command).status();

    match status {
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("{} Failed to execute: {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}
