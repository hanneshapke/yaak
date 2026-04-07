mod api;
mod command;
mod config;
mod explain;
mod streaming;
mod wizard;

use api::{AnthropicRequest, ChatRequest, Message};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use colored::Colorize;
use command::{detect_destructive, extract_command};
use config::{load_config, resolve};
use dialoguer::Confirm;
use explain::render_explanation;
use std::env;
use std::io::Write;
use std::process::Command;

/// yaak — translate natural language into bash commands using an OpenAI-compatible LLM
#[derive(Parser, Debug)]
#[command(name = "yaak", version, about)]
struct Args {
    /// The natural language description of the command you want
    #[arg(trailing_var_arg = true)]
    description: Vec<String>,

    /// Interactive configuration wizard — set up your config file
    #[arg(short = 'c', long, exclusive = true)]
    config: bool,

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

    /// Generate shell completions and print to stdout
    #[arg(long, exclusive = true, value_name = "SHELL")]
    completions: Option<Shell>,
}

fn main() {
    let args = Args::parse();

    if let Some(shell) = args.completions {
        let mut cmd = Args::command();
        generate(shell, &mut cmd, "yaak", &mut std::io::stdout());
        std::process::exit(0);
    }

    if args.config {
        wizard::run_config_wizard();
        std::process::exit(0);
    }

    if args.description.is_empty() {
        eprintln!(
            "{} No description provided. Run `yaak --help` for usage or `yaak --config` to set up.",
            "error:".red().bold()
        );
        std::process::exit(1);
    }

    let config = load_config();

    let api_base = resolve(args.api_base, config.api_base, "https://api.openai.com/v1");
    let api_key = resolve(args.api_key, config.api_key, "");
    let anthropic = api::is_anthropic(&api_base);
    let default_model = if anthropic {
        "claude-sonnet-4-6"
    } else {
        "gpt-4o-mini"
    };
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

    // --- Call the LLM (streaming) ---
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
            stream: true,
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
            stream: true,
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
            eprintln!("{} Failed to reach API: {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        eprintln!(
            "{} API returned {} — {}",
            "error:".red().bold(),
            status,
            body
        );
        std::process::exit(1);
    }

    // --- Stream and collect tokens ---
    let raw_content = if args.reverse {
        // Explain mode: stream tokens to stderr for real-time feedback
        let mut collected = String::new();
        let mut first_token = true;
        streaming::stream_tokens(response, anthropic, |token| {
            if first_token {
                // Clear the line and print header before first token
                eprint!("\r\x1b[K");
                first_token = false;
            }
            eprint!("{}", token);
            let _ = std::io::stderr().flush();
            collected.push_str(token);
        });
        eprintln!();
        collected
    } else {
        // Generate mode: collect tokens silently, show a dot progress indicator
        eprint!("{}", "Thinking ".dimmed());
        let mut collected = String::new();
        let mut token_count = 0usize;
        streaming::stream_tokens(response, anthropic, |token| {
            collected.push_str(token);
            token_count += 1;
            if token_count.is_multiple_of(4) {
                eprint!("{}", ".".dimmed());
                let _ = std::io::stderr().flush();
            }
        });
        eprint!("\r\x1b[K"); // clear progress line
        collected
    };

    if args.reverse {
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

#[cfg(test)]
mod tests {
    use crate::api::is_anthropic;
    use crate::config::resolve;

    #[test]
    fn default_model_for_anthropic() {
        let api_base = "https://api.anthropic.com/v1";
        let default_model = if is_anthropic(api_base) {
            "claude-sonnet-4-6"
        } else {
            "gpt-4o-mini"
        };
        assert_eq!(resolve(None, None, default_model), "claude-sonnet-4-6");
    }

    #[test]
    fn default_model_for_openai() {
        let api_base = "https://api.openai.com/v1";
        let default_model = if is_anthropic(api_base) {
            "claude-sonnet-4-6"
        } else {
            "gpt-4o-mini"
        };
        assert_eq!(resolve(None, None, default_model), "gpt-4o-mini");
    }

    #[test]
    fn explicit_model_overrides_anthropic_default() {
        let api_base = "https://api.anthropic.com/v1";
        let default_model = if is_anthropic(api_base) {
            "claude-sonnet-4-6"
        } else {
            "gpt-4o-mini"
        };
        assert_eq!(
            resolve(Some("claude-opus-4-6".into()), None, default_model),
            "claude-opus-4-6"
        );
    }
}
