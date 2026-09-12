use colored::Colorize;
use dialoguer::{Confirm, Input, Password, Select};
use rust_i18n::t;
use std::path::PathBuf;
use std::time::Duration;

use crate::api::{build_verify_request, classify_verify_status, KeyCheck};
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
        suggested_models: &["qwen3.5", "gemma4:e2b", "llama3.2", "mistral"],
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
        suggested_models: &["qwen3.5", "gemma4:e2b"],
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
        Some(prompt_for_api_key(provider, &model))
    } else {
        None
    };

    // Build config TOML
    let config_content = build_config_toml(provider.api_base, api_key.as_deref(), &model, language);

    // Warn if overwriting, before creating anything on disk
    let path = config_path();
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

    // Write config file
    if let Some(parent) = path.parent() {
        if let Err(e) = create_config_dir(parent) {
            eprintln!(
                "{} {}",
                t!("error_prefix").red().bold(),
                t!("wizard_config_dir_error", error = e)
            );
            std::process::exit(1);
        }
    }

    if let Err(e) = write_config_file(&path, &config_content) {
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
    eprintln!();

    // 5. Offer to create shell alias y -> yaak
    offer_shell_alias();
}

/// Maximum number of times the user may re-enter a key the provider rejects.
const MAX_KEY_ATTEMPTS: usize = 3;

/// Overall bound on prompt rounds, so a repeatedly empty entry still terminates.
const MAX_KEY_PROMPTS: usize = 10;

/// Prompt for an API key with echo suppressed, verifying it against the provider
/// before accepting it. Re-prompts on an empty or rejected key.
fn prompt_for_api_key(provider: &Provider, model: &str) -> String {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());

    // Only a key the provider rejects counts against MAX_KEY_ATTEMPTS. An
    // accidental empty entry shouldn't burn one of the user's real tries.
    let mut rejections = 0;

    for _ in 0..MAX_KEY_PROMPTS {
        let entered = Password::new()
            .with_prompt(t!("wizard_enter_api_key").to_string())
            .interact()
            .unwrap_or_else(|_| {
                eprintln!("{}", t!("aborted").dimmed());
                std::process::exit(0);
            });

        // Keys never carry meaningful surrounding whitespace, and pasting one
        // often picks up a stray space or newline.
        let key = entered.trim().to_string();

        if key.is_empty() {
            eprintln!(
                "{} {}",
                t!("warning_prefix").yellow().bold(),
                t!("wizard_api_key_empty", provider = provider.name)
            );
            continue;
        }

        eprintln!("{}", t!("wizard_verifying_key").dimmed());
        match verify_api_key(&client, provider.api_base, model, &key) {
            KeyCheck::Valid => {
                eprintln!("{} {}", "✓".green().bold(), t!("wizard_key_valid").green());
                eprintln!();
                return key;
            }
            KeyCheck::ModelMissing(_) => {
                // The key authenticated; only the model or base URL looks wrong.
                eprintln!(
                    "{} {}",
                    t!("warning_prefix").yellow().bold(),
                    t!("wizard_key_model_missing", model = model)
                );
                eprintln!();
                return key;
            }
            KeyCheck::Rejected(status) => {
                eprintln!(
                    "{} {}",
                    t!("warning_prefix").yellow().bold(),
                    t!(
                        "wizard_key_rejected",
                        provider = provider.name,
                        status = status
                    )
                );
                rejections += 1;
                if rejections >= MAX_KEY_ATTEMPTS {
                    eprintln!(
                        "{} {}",
                        t!("error_prefix").red().bold(),
                        t!("wizard_key_attempts_exhausted")
                    );
                    std::process::exit(1);
                }
                eprintln!();
            }
            KeyCheck::Unknown(reason) => {
                // Offline, proxied, or a provider-side error. Don't block setup on it.
                eprintln!(
                    "{} {}",
                    t!("warning_prefix").yellow().bold(),
                    t!("wizard_key_unverified", reason = reason)
                );
                let save_anyway = Confirm::new()
                    .with_prompt(t!("wizard_key_save_anyway").to_string())
                    .default(true)
                    .interact()
                    .unwrap_or(true);
                eprintln!();
                if save_anyway {
                    return key;
                }
            }
        }
    }

    eprintln!(
        "{} {}",
        t!("error_prefix").red().bold(),
        t!("wizard_key_attempts_exhausted")
    );
    std::process::exit(1);
}

/// Send a minimal completion request to check whether the provider accepts `api_key`.
fn verify_api_key(
    client: &reqwest::blocking::Client,
    api_base: &str,
    model: &str,
    api_key: &str,
) -> KeyCheck {
    let req = build_verify_request(api_base, model, api_key);
    let mut builder = client
        .post(&req.url)
        .header("Content-Type", "application/json");
    for (name, value) in &req.headers {
        builder = builder.header(name, value);
    }

    match builder.json(&req.body).send() {
        Ok(resp) => classify_verify_status(resp.status().as_u16()),
        Err(e) => KeyCheck::Unknown(e.to_string()),
    }
}

/// Render `value` as a properly escaped TOML string literal, quotes included.
///
/// API keys and custom model names are free-form, so interpolating them raw would
/// produce a file that `load_config` silently fails to parse.
fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}

/// Assemble the config file contents.
pub fn build_config_toml(
    api_base: &str,
    api_key: Option<&str>,
    model: &str,
    language: &str,
) -> String {
    let mut out = String::new();
    out.push_str("# yaak configuration — generated by `yaak --config`\n\n");
    out.push_str(&format!("api_base = {}\n", toml_string(api_base)));
    if let Some(key) = api_key {
        out.push_str(&format!("api_key = {}\n", toml_string(key)));
    }
    out.push_str(&format!("model = {}\n", toml_string(model)));
    out.push_str(&format!("language = {}\n", toml_string(language)));
    out
}

/// Create the config directory, restricted to the current user.
#[cfg(unix)]
fn create_config_dir(dir: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    if dir.exists() {
        return Ok(());
    }
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

/// Create the config directory.
#[cfg(not(unix))]
fn create_config_dir(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// Write the config file. It holds an API key, so it is created with 0600
/// rather than left at the default umask.
#[cfg(unix)]
fn write_config_file(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents.as_bytes())?;

    // `mode` only applies when the file is created, so tighten an existing
    // file that was previously written with looser permissions.
    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
        eprintln!(
            "{} {}",
            t!("warning_prefix").yellow().bold(),
            t!("wizard_config_permissions_error", error = e)
        );
    }
    Ok(())
}

/// Write the config file. Windows has no umask equivalent here, so the file
/// inherits the permissions of its parent directory.
#[cfg(not(unix))]
fn write_config_file(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    std::fs::write(path, contents)
}

/// Detect the user's shell rc file and offer to append `alias y='yaak'`.
fn offer_shell_alias() {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return, // no HOME — skip silently
    };

    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = shell.rsplit('/').next().unwrap_or("");

    let (rc_path, alias_line) = match shell_name {
        "zsh" => (home.join(".zshrc"), "alias y='yaak'"),
        "fish" => (home.join(".config/fish/config.fish"), "alias y 'yaak'"),
        // bash and anything else
        _ => (home.join(".bashrc"), "alias y='yaak'"),
    };

    // Check if the exact alias already exists in the rc file
    if rc_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&rc_path) {
            if contents.contains(alias_line) {
                eprintln!(
                    "{} {}",
                    "✓".green().bold(),
                    t!("wizard_alias_already_exists", path = rc_path.display())
                );
                return;
            }
        }
    }

    // Check if `y` is already mapped to something else
    let existing_target = detect_existing_y(&shell, &rc_path, shell_name);
    if let Some(target) = &existing_target {
        eprintln!(
            "{} {}",
            t!("warning_prefix").yellow().bold(),
            t!("wizard_alias_conflict", target = target.as_str())
        );
        let overwrite = Confirm::new()
            .with_prompt(t!("wizard_alias_overwrite_prompt").to_string())
            .default(false)
            .interact()
            .unwrap_or(false);
        if !overwrite {
            return;
        }
    } else {
        let create_alias = Confirm::new()
            .with_prompt(t!("wizard_alias_prompt").to_string())
            .default(true)
            .interact()
            .unwrap_or(false);
        if !create_alias {
            return;
        }
    }

    // Ensure parent directory exists (relevant for fish)
    if let Some(parent) = rc_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Append alias line
    let line = format!("\n# yaak shortcut\n{}\n", alias_line);
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_path)
    {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(e) = file.write_all(line.as_bytes()) {
                eprintln!(
                    "{} {}",
                    t!("error_prefix").red().bold(),
                    t!("wizard_alias_write_error", error = e)
                );
                return;
            }
            eprintln!(
                "{} {}",
                "✓".green().bold(),
                t!("wizard_alias_added", path = rc_path.display())
            );
            eprintln!(
                "  {}",
                t!("wizard_alias_source_hint", path = rc_path.display()).yellow()
            );
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                t!("error_prefix").red().bold(),
                t!("wizard_alias_write_error", error = e)
            );
        }
    }
}

/// Check whether `y` is already defined as an alias (to something other than yaak)
/// or exists as a command on the system. Returns the existing target if found.
fn detect_existing_y(shell_path: &str, rc_path: &PathBuf, shell_name: &str) -> Option<String> {
    // 1. Check rc file for an existing alias definition for `y`
    if rc_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(rc_path) {
            for line in contents.lines() {
                let trimmed = line.trim();
                match shell_name {
                    "fish" => {
                        // fish: `alias y 'something'` or `alias y "something"` or `alias y something`
                        if let Some(rest) = trimmed.strip_prefix("alias y ") {
                            let value = rest.trim().trim_matches(|c| c == '\'' || c == '"');
                            if value != "yaak" {
                                return Some(value.to_string());
                            }
                        }
                    }
                    _ => {
                        // bash/zsh: `alias y='something'` or `alias y="something"`
                        if let Some(rest) = trimmed.strip_prefix("alias y=") {
                            let value = rest.trim_matches(|c| c == '\'' || c == '"');
                            if value != "yaak" {
                                return Some(value.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Check if `y` exists as a command on PATH (binary, script, etc.)
    let check_cmd = match shell_name {
        "fish" => "command -v y",
        _ => "command -v y",
    };
    if let Ok(output) = std::process::Command::new(shell_path)
        .args(["-c", check_cmd])
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn config_toml_includes_all_fields() {
        let toml_out = build_config_toml(
            "https://api.openai.com/v1",
            Some("sk-test"),
            "gpt-4o-mini",
            "en",
        );
        let config: Config = toml::from_str(&toml_out).unwrap();
        assert_eq!(
            config.api_base.as_deref(),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(config.api_key.as_deref(), Some("sk-test"));
        assert_eq!(config.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(config.language.as_deref(), Some("en"));
        assert!(!config.allow_destructive);
    }

    #[test]
    fn config_toml_omits_api_key_when_absent() {
        let toml_out = build_config_toml("http://localhost:11434/v1", None, "qwen3.5", "de");
        assert!(!toml_out.contains("api_key"));
        let config: Config = toml::from_str(&toml_out).unwrap();
        assert!(config.api_key.is_none());
        assert_eq!(config.model.as_deref(), Some("qwen3.5"));
    }

    #[test]
    fn config_toml_escapes_special_characters() {
        // A key or model name containing quotes, backslashes or newlines must
        // still produce a file that parses back to the exact input.
        let nasty_key = "sk-\"quoted\"\\back\nslash\t";
        let nasty_model = "my\"model\\v2";
        let toml_out = build_config_toml(
            "https://api.openai.com/v1",
            Some(nasty_key),
            nasty_model,
            "en",
        );
        let config: Config = toml::from_str(&toml_out).unwrap();
        assert_eq!(config.api_key.as_deref(), Some(nasty_key));
        assert_eq!(config.model.as_deref(), Some(nasty_model));
    }

    #[test]
    fn toml_string_survives_a_roundtrip() {
        // The exact literal form is the toml crate's choice — it may emit a basic
        // string or a literal one. What matters is that it parses back unchanged.
        for value in [
            "plain",
            "a\"b",
            "a\\b",
            "both \" and \\",
            "line\nbreak",
            "tab\there",
            "'single'",
            "",
        ] {
            let doc = format!("v = {}", toml_string(value));
            let parsed: toml::Value = toml::from_str(&doc)
                .unwrap_or_else(|e| panic!("{:?} produced invalid TOML: {}", value, e));
            assert_eq!(parsed["v"].as_str(), Some(value), "roundtrip failed");
        }
    }

    #[test]
    fn write_config_file_creates_and_reads_back() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        create_config_dir(path.parent().unwrap()).unwrap();
        let contents = build_config_toml("https://api.openai.com/v1", Some("sk-x"), "m", "en");
        write_config_file(&path, &contents).unwrap();

        let read_back = std::fs::read_to_string(&path).unwrap();
        let config: Config = toml::from_str(&read_back).unwrap();
        assert_eq!(config.api_key.as_deref(), Some("sk-x"));
    }

    #[cfg(unix)]
    #[test]
    fn write_config_file_restricts_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        // Pre-create the file world-readable to prove existing files get tightened.
        std::fs::write(&path, "stale = true\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_config_file(&path, "api_key = \"sk-x\"\n").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn create_config_dir_restricts_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("yaak");
        create_config_dir(&nested).unwrap();

        let mode = std::fs::metadata(&nested).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    /// Serve one canned HTTP response on a loopback port and return its base URL.
    /// Lets the verification path be exercised over a real socket, with no network.
    fn stub_server(status_line: &'static str) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                // Drain what's available so the client isn't writing into a closed pipe.
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}",
                        status_line
                    )
                    .as_bytes(),
                );
                let _ = stream.flush();
            }
        });
        (format!("http://{}/v1", addr), handle)
    }

    fn test_client() -> reqwest::blocking::Client {
        // no_proxy so an ambient HTTPS_PROXY doesn't intercept the loopback request.
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .build()
            .unwrap()
    }

    #[test]
    fn verify_api_key_accepts_a_good_key() {
        let (base, handle) = stub_server("200 OK");
        let result = verify_api_key(&test_client(), &base, "some-model", "sk-good");
        handle.join().unwrap();
        assert_eq!(result, KeyCheck::Valid);
    }

    #[test]
    fn verify_api_key_rejects_a_bad_key() {
        let (base, handle) = stub_server("401 Unauthorized");
        let result = verify_api_key(&test_client(), &base, "some-model", "sk-bad");
        handle.join().unwrap();
        assert_eq!(result, KeyCheck::Rejected(401));
    }

    #[test]
    fn verify_api_key_reports_unknown_when_unreachable() {
        // Bind then drop, so the port is almost certainly closed.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let base = format!("http://{}/v1", addr);
        let result = verify_api_key(&test_client(), &base, "some-model", "sk-x");
        assert!(
            matches!(result, KeyCheck::Unknown(_)),
            "expected Unknown, got {:?}",
            result
        );
    }

    #[test]
    fn every_provider_needing_a_key_has_suggested_models() {
        for provider in PROVIDERS {
            assert!(
                !provider.suggested_models.is_empty(),
                "{} has no suggested models",
                provider.name
            );
        }
    }
}
