use colored::Colorize;
use dialoguer::{Confirm, Input, Password, Select};
use rust_i18n::t;

use crate::config::config_path;

pub struct Provider {
    pub name: &'static str,
    pub api_base: &'static str,
    pub needs_api_key: bool,
    pub suggested_models: &'static [&'static str],
}

pub const PROVIDERS: &[Provider] = &[
    Provider {
        name: "OpenAI",
        api_base: "https://api.openai.com/v1",
        needs_api_key: true,
        suggested_models: &["gpt-4o-mini", "gpt-4o", "gpt-4.1-mini", "gpt-4.1-nano"],
    },
    Provider {
        name: "Anthropic",
        api_base: "https://api.anthropic.com/v1",
        needs_api_key: true,
        suggested_models: &[
            "claude-sonnet-4-6",
            "claude-haiku-4-5-20251001",
            "claude-opus-4-6",
        ],
    },
    Provider {
        name: "Google Gemini",
        api_base: "https://generativelanguage.googleapis.com/v1beta",
        needs_api_key: true,
        suggested_models: &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.0-flash"],
    },
    Provider {
        name: "Ollama",
        api_base: "http://localhost:11434/v1",
        needs_api_key: false,
        suggested_models: &["qwen3.5", "gemma3:4b", "llama3.2", "mistral"],
    },
    Provider {
        name: "Groq",
        api_base: "https://api.groq.com/openai/v1",
        needs_api_key: true,
        suggested_models: &[
            "llama-3.3-70b-versatile",
            "gemma2-9b-it",
            "mixtral-8x7b-32768",
        ],
    },
    Provider {
        name: "Together AI",
        api_base: "https://api.together.xyz/v1",
        needs_api_key: true,
        suggested_models: &[
            "meta-llama/Llama-3-70b-chat-hf",
            "mistralai/Mixtral-8x7B-Instruct-v0.1",
        ],
    },
    Provider {
        name: "OpenRouter",
        api_base: "https://openrouter.ai/api/v1",
        needs_api_key: true,
        suggested_models: &[
            "openai/gpt-4o-mini",
            "anthropic/claude-sonnet-4-6",
            "meta-llama/llama-3-70b-instruct",
        ],
    },
    Provider {
        name: "Doubleword",
        api_base: "https://api.doubleword.ai/v1",
        needs_api_key: true,
        suggested_models: &[
            "Qwen/Qwen3.5-35B-A3B-FP8",
            "Qwen/Qwen3.5-9B",
            "Qwen/Qwen3-14B-FP8",
        ],
    },
    Provider {
        name: "LM Studio",
        api_base: "http://localhost:1234/v1",
        needs_api_key: false,
        suggested_models: &["qwen3.5", "gemma3:4b"],
    },
    Provider {
        name: "vLLM",
        api_base: "http://localhost:8000/v1",
        needs_api_key: false,
        suggested_models: &["meta-llama/Llama-3-8b-chat-hf"],
    },
    Provider {
        name: "LocalAI",
        api_base: "http://localhost:8080/v1",
        needs_api_key: false,
        suggested_models: &["gpt-4o-mini"],
    },
];

const LANGUAGE_NAMES: &[&str] = &[
    "English",
    "Deutsch",
    "Español",
    "Français",
    "Português",
    "中文",
    "日本語",
    "한국어",
];
const LANGUAGE_CODES: &[&str] = &["en", "de", "es", "fr", "pt", "zh", "ja", "ko"];

pub fn run_config_wizard() {
    eprintln!("{}", t!("wizard_title").bold());
    eprintln!("{}", "─".repeat(40).dimmed());
    eprintln!();

    // 0. Select language
    let lang_idx = Select::new()
        .with_prompt(t!("wizard_select_language").to_string())
        .items(LANGUAGE_NAMES)
        .default(0)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("{}", t!("aborted").dimmed());
            std::process::exit(0);
        });
    let language = LANGUAGE_CODES[lang_idx];
    rust_i18n::set_locale(language);
    eprintln!();

    // 1. Select provider
    let provider_names: Vec<&str> = PROVIDERS.iter().map(|p| p.name).collect();
    let provider_idx = Select::new()
        .with_prompt(t!("wizard_select_provider").to_string())
        .items(&provider_names)
        .default(0)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("{}", t!("aborted").dimmed());
            std::process::exit(0);
        });

    let provider = &PROVIDERS[provider_idx];
    eprintln!();

    // 2. Select model
    let mut model_options: Vec<String> = provider
        .suggested_models
        .iter()
        .map(|m| m.to_string())
        .collect();
    model_options.push(t!("wizard_enter_custom_model").to_string());

    let model_idx = Select::new()
        .with_prompt(t!("wizard_select_model").to_string())
        .items(&model_options)
        .default(0)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("{}", t!("aborted").dimmed());
            std::process::exit(0);
        });

    let model = if model_idx == model_options.len() - 1 {
        Input::<String>::new()
            .with_prompt(t!("wizard_enter_model").to_string())
            .interact_text()
            .unwrap_or_else(|_| {
                eprintln!("{}", t!("aborted").dimmed());
                std::process::exit(0);
            })
    } else {
        model_options[model_idx].clone()
    };
    eprintln!();

    // 3. API key (if needed)
    let api_key = if provider.needs_api_key {
        let key = Password::new()
            .with_prompt(t!("wizard_enter_api_key").to_string())
            .interact()
            .unwrap_or_else(|_| {
                eprintln!("{}", t!("aborted").dimmed());
                std::process::exit(0);
            });
        if key.is_empty() {
            eprintln!(
                "{} {}",
                t!("warning_prefix").yellow().bold(),
                t!("wizard_api_key_required", provider = provider.name)
            );
            std::process::exit(1);
        }
        eprintln!();
        Some(key)
    } else {
        None
    };

    // Build config TOML
    let mut config_content = String::new();
    config_content.push_str("# yaak configuration — generated by `yaak --config`\n\n");
    config_content.push_str(&format!("api_base = \"{}\"\n", provider.api_base));
    if let Some(key) = &api_key {
        config_content.push_str(&format!("api_key = \"{}\"\n", key));
    }
    config_content.push_str(&format!("model = \"{}\"\n", model));
    config_content.push_str(&format!("language = \"{}\"\n", language));

    // Write config file
    let path = config_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!(
                "{} {}",
                t!("error_prefix").red().bold(),
                t!("wizard_config_dir_error", error = e)
            );
            std::process::exit(1);
        }
    }

    // Warn if overwriting
    if path.exists() {
        let overwrite = Confirm::new()
            .with_prompt(t!("wizard_config_overwrite", path = path.display()).to_string())
            .default(false)
            .interact()
            .unwrap_or(false);
        if !overwrite {
            eprintln!("{}", t!("aborted").dimmed());
            std::process::exit(0);
        }
    }

    if let Err(e) = std::fs::write(&path, &config_content) {
        eprintln!(
            "{} {}",
            t!("error_prefix").red().bold(),
            t!("wizard_config_write_error", error = e)
        );
        std::process::exit(1);
    }

    eprintln!("{}", "─".repeat(40).dimmed());
    eprintln!(
        "{} {}",
        "✓".green().bold(),
        t!("wizard_config_written", path = path.display())
    );
    eprintln!(
        "  {} {}",
        t!("wizard_label_provider").dimmed(),
        provider.name.bold()
    );
    eprintln!("  {} {}", t!("wizard_label_model").dimmed(), model.bold());
    if api_key.is_some() {
        eprintln!(
            "  {} {}",
            t!("wizard_label_api_key").dimmed(),
            "••••••••".dimmed()
        );
    }
    eprintln!();
    eprintln!("{}", t!("wizard_success").green());
}
