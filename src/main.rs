mod api;
mod cache;
mod command;
mod config;
mod context;
mod explain;
mod history;
mod streaming;
mod wizard;

use api::{
    AnthropicRequest, ChatRequest, GeminiContent, GeminiGenerationConfig, GeminiPart,
    GeminiRequest, GeminiSystemInstruction, Message,
};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use colored::Colorize;
use command::{detect_destructive, extract_command};
use config::{load_config, resolve};
use dialoguer::{Confirm, Input, Select};
use explain::render_explanation;
use rust_i18n::t;
use std::env;
use std::io::Write;
use std::process::Command;

rust_i18n::i18n!("locales");

/// yaak — translate natural language into bash commands using an OpenAI-compatible LLM
#[derive(Parser, Debug)]
#[command(name = "yaak", about)]
struct Args {
    /// Print version
    #[arg(short = 'v', long)]
    version: bool,

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

    /// Allow destructive commands (bypass safety check)
    #[arg(short = 'f', long)]
    force: bool,

    /// Reverse mode: explain a command instead of generating one
    #[arg(
        short = 'r',
        long = "reverse",
        visible_short_alias = 'e',
        visible_alias = "explain"
    )]
    reverse: bool,

    /// Copy the generated command to clipboard instead of executing
    #[arg(short = 'C', long)]
    copy: bool,

    /// Include working directory context (project type, file listing) in the prompt
    #[arg(long)]
    context: bool,
    /// Use cached result for identical descriptions (skips API call)
    #[arg(long)]
    cache: bool,

    /// Force a fresh API call, ignoring any cached result
    #[arg(long)]
    no_cache: bool,

    /// Generate shell completions and print to stdout
    #[arg(long, exclusive = true, value_name = "SHELL")]
    completions: Option<Shell>,
    /// Show recent command history
    #[arg(short = 'H', long, exclusive = true)]
    history: bool,

    /// Re-execute the most recent generated command
    #[arg(short = 'l', long, exclusive = true)]
    last: bool,

    /// Search command history by keyword
    #[arg(short = 's', long, exclusive = true)]
    search: Option<String>,

    /// Number of history entries to show (default: 20)
    #[arg(long, default_value = "20")]
    limit: usize,

    /// UI language: en, de, es, fr (overrides YAAK_LANGUAGE / config)
    #[arg(short = 'L', long, env = "YAAK_LANGUAGE")]
    language: Option<String>,

    /// Open the feedback page in your browser
    #[arg(long, exclusive = true)]
    feedback: bool,
}

fn main() {
    let args = Args::parse();

    if args.version {
        println!("yaak {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.feedback {
        let version = env!("CARGO_PKG_VERSION");
        let os = env::consts::OS;
        let arch = env::consts::ARCH;
        let body = format!("yaak v{} on {} {}", version, os, arch);
        let url = format!(
            "https://github.com/hanneshapke/yaak/issues/new?labels=feedback&title=Feedback&body={}",
            urlencoding(&body)
        );
        eprintln!("{} Opening feedback page...", "✓".green().bold());
        open_url(&url);
        return;
    }

    // --- Resolve language and set locale ---
    {
        let config_lang = load_config().language;
        let lang = args.language.clone().or(config_lang).unwrap_or_else(|| {
            sys_locale::get_locale()
                .and_then(|l| l.split(['-', '_']).next().map(String::from))
                .unwrap_or_else(|| "en".into())
        });
        let lang = match lang.as_str() {
            "en" | "de" | "es" | "fr" | "pt" | "zh" | "ja" | "ko" => lang,
            _ => "en".into(),
        };
        rust_i18n::set_locale(&lang);
    }

    if let Some(shell) = args.completions {
        let mut cmd = Args::command();
        generate(shell, &mut cmd, "yaak", &mut std::io::stdout());
        std::process::exit(0);
    }

    if args.config {
        wizard::run_config_wizard();
        std::process::exit(0);
    }

    // --- History commands (no API key needed) ---
    if args.history {
        history::show_history(args.limit);
        std::process::exit(0);
    }

    if let Some(query) = &args.search {
        history::search_history(query);
        std::process::exit(0);
    }

    if args.last {
        let shell = env::var("SHELL").unwrap_or_else(|_| "bash".into());
        match history::get_last() {
            Some(entry) => {
                eprintln!(
                    "{}{}",
                    t!("label_command").bold(),
                    entry.command.green().bold()
                );
                let should_run = args.yes
                    || Confirm::new()
                        .with_prompt(t!("prompt_execute").to_string())
                        .default(true)
                        .interact()
                        .unwrap_or(false);
                if !should_run {
                    eprintln!("{}", t!("aborted").dimmed());
                    std::process::exit(0);
                }
                let status = Command::new(&shell).arg("-c").arg(&entry.command).status();
                match status {
                    Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                    Err(e) => {
                        eprintln!(
                            "{} {}",
                            t!("error_prefix").red().bold(),
                            t!("error_failed_execute", error = e)
                        );
                        std::process::exit(1);
                    }
                }
            }
            None => {
                eprintln!(
                    "{} {}",
                    t!("info_prefix").dimmed(),
                    t!("no_history_entries")
                );
                std::process::exit(0);
            }
        }
    }

    if args.description.is_empty() {
        eprintln!(
            "{} {}",
            t!("error_prefix").red().bold(),
            t!("error_no_description")
        );
        std::process::exit(1);
    }

    let config = load_config();

    let api_base = resolve(args.api_base, config.api_base, "https://api.openai.com/v1");
    let api_key = resolve(args.api_key, config.api_key, "");
    let anthropic = api::is_anthropic(&api_base);
    let gemini = api::is_gemini(&api_base);
    let default_model = if anthropic {
        "claude-sonnet-4-6"
    } else if gemini {
        "gemini-2.5-flash"
    } else {
        "gpt-4o-mini"
    };
    let model = resolve(args.model, config.model, default_model);

    let needs_api_key = !api_base.contains("localhost") && !api_base.contains("127.0.0.1");
    if needs_api_key && api_key.is_empty() {
        eprintln!(
            "{} {}",
            t!("error_prefix").red().bold(),
            t!("error_no_api_key")
        );
        std::process::exit(1);
    }

    let description = args.description.join(" ");

    let os_name = env::consts::OS;
    let shell = env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let shell_name = shell.rsplit('/').next().unwrap_or("bash");
    let cwd = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let locale = rust_i18n::locale().to_string();
    let lang_name = match locale.as_str() {
        "de" => "German",
        "es" => "Spanish",
        "fr" => "French",
        "pt" => "Portuguese",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        _ => "English",
    };
    let lang_hint = if locale != "en" {
        format!(
            "\n\nThe user's preferred language is {}. \
             Write all explanatory text (summaries, descriptions, cautions) in {}. \
             Keep the structural headers (SUMMARY:, BREAKDOWN:, PART:, EXAMPLES:, EXAMPLE:, CAUTION:) \
             exactly as shown — they are parsing markers.",
            lang_name, lang_name
        )
    } else {
        String::new()
    };

    let system_prompt = if args.reverse {
        format!(
            "You are a command-line teacher. The user is running {} on {}. \
             Their current working directory is {}. \
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
             Do NOT use markdown. Do NOT use code fences. Use the exact format above.{}",
            shell_name, os_name, cwd, lang_hint
        )
    } else {
        format!(
            "You are a command-line assistant. The user is running {} on {}. \
             Their current working directory is {}. \
             The user will describe what they want to do \
             and you must respond with ONLY the exact shell command to accomplish it. \
             No explanation, no markdown fences, no commentary — just the raw command. \
             Only use flags and tools available on {}. \
             If multiple commands are needed, join them with && or ;. \
             Use common, portable tools when possible.{}",
            shell_name, os_name, cwd, os_name, lang_hint
        )
    };

    // --- Build user message (with optional context) ---
    let user_message = if args.context {
        let ctx = context::gather_context();
        format!("{}\n\nContext:\n{}", description, ctx)
    } else {
        description.clone()
    };
    let client = reqwest::blocking::Client::new();
    let mut current_description = description.clone();
    let mut current_user_message = user_message.clone();

    loop {
        // --- Check cache (generate mode only) ---
        let command = if !args.reverse && args.cache && !args.no_cache {
            if let Some(cached) = cache::get(&current_description, &model) {
                eprintln!("{}", t!("cached").dimmed());
                Some(extract_command(&cached.command))
            } else {
                None
            }
        } else {
            None
        };

        let command = if let Some(cmd) = command {
            cmd
        } else {
            // --- Call the LLM (streaming) ---
            let response = if anthropic {
                let url = format!("{}/messages", api_base.trim_end_matches('/'));
                let request_body = AnthropicRequest {
                    model: model.clone(),
                    system: system_prompt.clone(),
                    messages: vec![Message {
                        role: "user".into(),
                        content: current_user_message.clone(),
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
            } else if gemini {
                let url = format!(
                    "{}/models/{}:streamGenerateContent?alt=sse&key={}",
                    api_base.trim_end_matches('/'),
                    model,
                    api_key
                );
                let request_body = GeminiRequest {
                    contents: vec![GeminiContent {
                        role: "user".into(),
                        parts: vec![GeminiPart {
                            text: current_user_message.clone(),
                        }],
                    }],
                    system_instruction: GeminiSystemInstruction {
                        parts: vec![GeminiPart {
                            text: system_prompt.clone(),
                        }],
                    },
                    generation_config: GeminiGenerationConfig { temperature: 0.0 },
                };
                client
                    .post(&url)
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
                            content: system_prompt.clone(),
                        },
                        Message {
                            role: "user".into(),
                            content: current_user_message.clone(),
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
                    eprintln!(
                        "{} {}",
                        t!("error_prefix").red().bold(),
                        t!("error_failed_api", error = e)
                    );
                    std::process::exit(1);
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().unwrap_or_default();
                eprintln!(
                    "{} {}",
                    t!("error_prefix").red().bold(),
                    t!("error_api_returned", status = status, body = body)
                );
                std::process::exit(1);
            }

            // --- Stream and collect tokens ---
            let stream_format = if anthropic {
                streaming::StreamFormat::Anthropic
            } else if gemini {
                streaming::StreamFormat::Gemini
            } else {
                streaming::StreamFormat::OpenAi
            };

            let raw_content = if args.reverse {
                let mut collected = String::new();
                let mut first_token = true;
                streaming::stream_tokens(response, stream_format, |token| {
                    if first_token {
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
                eprint!("{}", t!("label_thinking").dimmed());
                let mut collected = String::new();
                let mut token_count = 0usize;
                streaming::stream_tokens(response, stream_format, |token| {
                    collected.push_str(token);
                    token_count += 1;
                    if token_count.is_multiple_of(4) {
                        eprint!("{}", ".".dimmed());
                        let _ = std::io::stderr().flush();
                    }
                });
                eprint!("\r\x1b[K");
                collected
            };

            if args.reverse {
                render_explanation(&current_description, &raw_content);
                std::process::exit(0);
            }

            let cmd = extract_command(&raw_content);

            // --- Save to cache ---
            if !args.no_cache {
                cache::put(&current_description, &model, &cmd);
            }

            cmd
        };

        // --- Save to history ---
        history::save_entry(&current_description, &command, &model);

        // --- Display and confirm ---
        eprintln!("\r{}{}", t!("label_command").bold(), command.green().bold());

        // --- Destructive command check ---
        if let Some(keyword) = detect_destructive(&command) {
            let allow = args.force || config.allow_destructive;
            if allow {
                eprintln!(
                    "{} {}",
                    t!("warning_prefix").yellow().bold(),
                    t!("destructive_warning", keyword = keyword)
                );
            } else {
                eprintln!(
                    "{} {}",
                    t!("warning_prefix").yellow().bold(),
                    t!("destructive_warning", keyword = keyword)
                );
                let confirmed = Confirm::new()
                    .with_prompt(t!("destructive_confirm").to_string())
                    .default(false)
                    .interact()
                    .unwrap_or(false);
                if !confirmed {
                    eprintln!("{}", t!("aborted").dimmed());
                    std::process::exit(0);
                }
            }
        }

        // --- Copy-only mode ---
        if args.copy {
            copy_to_clipboard(&command);
            std::process::exit(0);
        }

        // --- Confirm: Execute / Refine / Copy / Abort ---
        if args.yes {
            // Skip prompt, execute directly
        } else {
            let choices = &[
                t!("choice_execute").to_string(),
                t!("choice_refine").to_string(),
                t!("choice_copy").to_string(),
                t!("choice_abort").to_string(),
            ];
            let selection = Select::new()
                .with_prompt(t!("prompt_what_next").to_string())
                .items(choices)
                .default(0)
                .interact()
                .unwrap_or(3);

            match selection {
                0 => {} // execute below
                1 => {
                    let refinement: String = Input::new()
                        .with_prompt(t!("prompt_refine").to_string())
                        .interact_text()
                        .unwrap_or_else(|_| {
                            eprintln!("{}", t!("aborted").dimmed());
                            std::process::exit(0);
                        });
                    current_description = format!(
                        "{}\n\nYou previously suggested: {}\nThe user wants this change: {}",
                        current_description, command, refinement
                    );
                    current_user_message = if args.context {
                        let ctx = context::gather_context();
                        format!("{}\n\nContext:\n{}", current_description, ctx)
                    } else {
                        current_description.clone()
                    };
                    continue;
                }
                2 => {
                    copy_to_clipboard(&command);
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("{}", t!("aborted").dimmed());
                    std::process::exit(0);
                }
            }
        }

        // --- Execute ---
        let status = Command::new(&shell).arg("-c").arg(&command).status();

        match status {
            Ok(s) => std::process::exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!(
                    "{} {}",
                    t!("error_prefix").red().bold(),
                    t!("error_failed_execute", error = e)
                );
                std::process::exit(1);
            }
        }
    }
}

fn copy_to_clipboard(text: &str) {
    let (cmd, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("pbcopy", &[])
    } else if cfg!(target_os = "windows") {
        ("clip", &[])
    } else {
        // Linux: try wl-copy (Wayland) first, fall back to xclip (X11)
        if Command::new("wl-copy").arg("--version").output().is_ok() {
            ("wl-copy", &[])
        } else {
            ("xclip", &["-selection", "clipboard"])
        }
    };

    let result = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });

    match result {
        Ok(status) if status.success() => {
            eprintln!("{} {}", "✓".green().bold(), t!("clipboard_copied"));
        }
        _ => {
            eprintln!(
                "{} {}",
                t!("warning_prefix").yellow().bold(),
                t!("clipboard_failed")
            );
            println!("{}", text);
        }
    }
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", b));
            }
        }
    }
    result
}

fn open_url(url: &str) {
    let (cmd, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(target_os = "windows") {
        ("cmd", &["/C", "start"])
    } else {
        ("xdg-open", &[])
    };
    let mut command = Command::new(cmd);
    for arg in args {
        command.arg(arg);
    }
    let _ = command.arg(url).spawn();
}

#[cfg(test)]
mod tests {
    use crate::api::{is_anthropic, is_gemini};
    use crate::config::resolve;

    fn default_model_for(api_base: &str) -> &'static str {
        if is_anthropic(api_base) {
            "claude-sonnet-4-6"
        } else if is_gemini(api_base) {
            "gemini-2.5-flash"
        } else {
            "gpt-4o-mini"
        }
    }

    #[test]
    fn default_model_for_anthropic() {
        let m = default_model_for("https://api.anthropic.com/v1");
        assert_eq!(resolve(None, None, m), "claude-sonnet-4-6");
    }

    #[test]
    fn default_model_for_openai() {
        let m = default_model_for("https://api.openai.com/v1");
        assert_eq!(resolve(None, None, m), "gpt-4o-mini");
    }

    #[test]
    fn default_model_for_gemini() {
        let m = default_model_for("https://generativelanguage.googleapis.com/v1beta");
        assert_eq!(resolve(None, None, m), "gemini-2.5-flash");
    }

    #[test]
    fn explicit_model_overrides_anthropic_default() {
        let m = default_model_for("https://api.anthropic.com/v1");
        assert_eq!(
            resolve(Some("claude-opus-4-6".into()), None, m),
            "claude-opus-4-6"
        );
    }
}
