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

    /// Reverse mode: explain a command instead of generating one
    #[arg(
        short = 'r',
        long = "reverse",
        visible_short_alias = 'e',
        visible_alias = "explain"
    )]
    reverse: bool,
}

// --- OpenAI-compatible structs ---

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

// --- Anthropic structs ---

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    system: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

/// Returns true if the API base URL points to Anthropic's API.
fn is_anthropic(api_base: &str) -> bool {
    api_base.contains("anthropic.com")
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

/// Destructive command prefixes/patterns that should be blocked.
const DESTRUCTIVE_COMMANDS: &[&str] = &[
    "rm ",
    "rm\t",
    "rmdir ",
    "mkfs",
    "dd ",
    "dd\t",
    "> /dev/",
    "chmod -R 000",
    "chmod 000",
    ":(){ :|:& };:",
    "shred ",
    "wipefs ",
];

/// Returns `Some(keyword)` if the command contains a destructive operation.
fn detect_destructive(command: &str) -> Option<&'static str> {
    // Normalise: check each segment separated by && ; | ||
    for segment in command.split(['&', ';', '|']) {
        let trimmed = segment.trim().trim_start_matches('!');
        let trimmed = trimmed.trim();
        for &pattern in DESTRUCTIVE_COMMANDS {
            if trimmed.starts_with(pattern) || trimmed == pattern.trim() {
                return Some(pattern.trim());
            }
        }
        // Also catch "sudo rm ..." etc.
        if let Some(after_sudo) = trimmed.strip_prefix("sudo ") {
            let after_sudo = after_sudo.trim();
            for &pattern in DESTRUCTIVE_COMMANDS {
                if after_sudo.starts_with(pattern) || after_sudo == pattern.trim() {
                    return Some(pattern.trim());
                }
            }
        }
    }
    None
}

fn main() {
    let args = Args::parse();
    let config = load_config();

    let api_base = resolve(args.api_base, config.api_base, "https://api.openai.com/v1");
    let api_key = resolve(args.api_key, config.api_key, "");
    let anthropic = is_anthropic(&api_base);
    let default_model = if anthropic { "claude-sonnet-4-6" } else { "gpt-4o-mini" };
    let model = resolve(args.model, config.model, default_model);

    if api_key.is_empty() {
        eprintln!(
            "{} No API key found. Set YAAK_API_KEY, pass --api-key, or add it to ~/.config/yaak/config.toml",
            "error:".red().bold()
        );
        std::process::exit(1);
    }

    let description = args.description.join(" ");

    let os_name = env::consts::OS;
    let shell = env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let shell_name = shell.rsplit('/').next().unwrap_or("bash");

    let system_prompt = if args.reverse {
        format!(
            "You are a command-line teacher. The user is running {} on {}. \
             The user will give you a shell command and you must explain it in detail.\n\n\
             Format your response EXACTLY as follows (use these exact section headers):\n\
             SUMMARY: A single-sentence plain-English summary of what the command does.\n\n\
             BREAKDOWN:\n\
             For each part of the command, output a line like:\n\
             PART: <token> | <explanation>\n\
             Include the base command, every flag, every argument, operators (|, &&, >, etc.), \
             and subcommands. Each on its own PART line.\n\n\
             EXAMPLES:\n\
             Provide 1-2 short related example variations, each on a line starting with EXAMPLE: <command> | <description>\n\n\
             CAUTION: (only if the command is dangerous or has side effects, otherwise omit this section entirely)\n\
             A short warning about what could go wrong.\n\n\
             Do NOT use markdown. Do NOT use code fences. Use the exact format above.",
            shell_name, os_name
        )
    } else {
        format!(
            "You are a command-line assistant. The user is running {} on {}. \
             The user will describe what they want to do \
             and you must respond with ONLY the exact shell command to accomplish it. \
             No explanation, no markdown fences, no commentary — just the raw command. \
             Only use flags and tools available on {}. \
             If multiple commands are needed, join them with && or ;. \
             Use common, portable tools when possible.",
            shell_name, os_name, os_name
        )
    };

    // --- Call the LLM ---
    if args.reverse {
        eprint!("{}", "Explaining... ".dimmed());
    } else {
        eprint!("{}", "Thinking... ".dimmed());
    }

    let client = reqwest::blocking::Client::new();

    let response = if anthropic {
        let url = format!("{}/messages", api_base.trim_end_matches('/'));
        let request_body = AnthropicRequest {
            model: model.clone(),
            system: system_prompt,
            messages: vec![Message {
                role: "user".into(),
                content: description.clone(),
            }],
            max_tokens: 1024,
            temperature: 0.0,
        };
        client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
    } else {
        let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
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
        client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
    };

    let response = match response {
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

    let raw_content = if anthropic {
        let resp: AnthropicResponse = match response.json() {
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
        resp.content[0].text.clone()
    } else {
        let resp: ChatResponse = match response.json() {
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
        resp.choices[0].message.content.clone()
    };

    if args.reverse {
        // --- Reverse / Explain mode ---
        eprintln!("\r"); // clear "Thinking..."
        render_explanation(&description, &raw_content);
        std::process::exit(0);
    }

    let command = extract_command(&raw_content);

    // --- Display and confirm ---
    eprintln!("\r{}{}", "  Command: ".bold(), command.green().bold());

    // --- Block destructive commands ---
    if let Some(keyword) = detect_destructive(&command) {
        eprintln!(
            "{} Destructive command blocked: `{}` is not allowed.",
            "blocked:".red().bold(),
            keyword
        );
        std::process::exit(1);
    }

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

/// Render a richly formatted explanation of a command in the terminal.
fn render_explanation(command: &str, raw: &str) {
    let separator = "─".repeat(60);

    // Header: the command being explained
    eprintln!("{}", separator.dimmed());
    eprintln!("  {} {}", "⟩".cyan().bold(), command.green().bold());
    eprintln!("{}", separator.dimmed());

    let mut in_breakdown = false;
    let mut in_examples = false;

    for line in raw.lines() {
        let trimmed = line.trim();

        if let Some(summary) = trimmed.strip_prefix("SUMMARY:") {
            eprintln!();
            eprintln!("  {} {}", "💡".bold(), summary.trim().bold());
            eprintln!();
            in_breakdown = false;
            in_examples = false;
        } else if trimmed == "BREAKDOWN:" {
            eprintln!("  {}", "BREAKDOWN".cyan().bold().underline());
            in_breakdown = true;
            in_examples = false;
        } else if trimmed == "EXAMPLES:" {
            eprintln!();
            eprintln!("  {}", "EXAMPLES".cyan().bold().underline());
            in_breakdown = false;
            in_examples = true;
        } else if let Some(caution) = trimmed.strip_prefix("CAUTION:") {
            eprintln!();
            eprintln!(
                "  {} {}",
                "⚠  CAUTION:".yellow().bold(),
                caution.trim().yellow()
            );
            in_breakdown = false;
            in_examples = false;
        } else if let Some(part_body) = trimmed.strip_prefix("PART:") {
            if in_breakdown {
                if let Some((token, explanation)) = part_body.split_once('|') {
                    eprintln!(
                        "    {} {:<20} {} {}",
                        "│".dimmed(),
                        token.trim().green().bold(),
                        "→".dimmed(),
                        explanation.trim()
                    );
                } else {
                    eprintln!("    {} {}", "│".dimmed(), part_body.trim());
                }
            }
        } else if let Some(ex_body) = trimmed.strip_prefix("EXAMPLE:") {
            if in_examples {
                if let Some((cmd, desc)) = ex_body.split_once('|') {
                    eprintln!(
                        "    {} {}  {}",
                        "$".dimmed(),
                        cmd.trim().green(),
                        desc.trim().dimmed()
                    );
                } else {
                    eprintln!("    {} {}", "$".dimmed(), ex_body.trim().green());
                }
            }
        } else if !trimmed.is_empty() && !in_breakdown && !in_examples {
            // Fallback for lines that don't match structured format
            eprintln!("  {}", trimmed);
        }
    }

    eprintln!();
    eprintln!("{}", separator.dimmed());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_rm() {
        assert!(detect_destructive("rm -rf /").is_some());
        assert!(detect_destructive("rm file.txt").is_some());
    }

    #[test]
    fn blocks_sudo_rm() {
        assert!(detect_destructive("sudo rm -rf /").is_some());
    }

    #[test]
    fn blocks_rm_in_chain() {
        assert!(detect_destructive("echo hello && rm -rf /tmp").is_some());
        assert!(detect_destructive("ls; rm foo").is_some());
    }

    #[test]
    fn blocks_other_destructive() {
        assert!(detect_destructive("dd if=/dev/zero of=/dev/sda").is_some());
        assert!(detect_destructive("mkfs.ext4 /dev/sda1").is_some());
        assert!(detect_destructive("shred /dev/sda").is_some());
    }

    #[test]
    fn allows_safe_commands() {
        assert!(detect_destructive("ls -la").is_none());
        assert!(detect_destructive("cat file.txt").is_none());
        assert!(detect_destructive("grep -r pattern .").is_none());
        assert!(detect_destructive("echo remove").is_none());
    }

    #[test]
    fn render_explanation_does_not_panic() {
        let raw = "\
SUMMARY: Lists all files in the current directory including hidden ones.

BREAKDOWN:
PART: ls | List directory contents
PART: -la | Show long format and include hidden files
PART: /tmp | Target directory

EXAMPLES:
EXAMPLE: ls -lah | Include human-readable file sizes
EXAMPLE: ls -lt | Sort by modification time

CAUTION: None for this command.";

        // Should not panic; output goes to stderr
        render_explanation("ls -la /tmp", raw);
    }

    #[test]
    fn render_explanation_handles_minimal_input() {
        render_explanation("echo hello", "SUMMARY: Prints hello to stdout.");
    }

    #[test]
    fn detects_anthropic_provider() {
        assert!(is_anthropic("https://api.anthropic.com/v1"));
        assert!(is_anthropic("https://api.anthropic.com"));
        assert!(!is_anthropic("https://api.openai.com/v1"));
        assert!(!is_anthropic("http://localhost:11434/v1"));
    }
}
