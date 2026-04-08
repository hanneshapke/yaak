# Changelog

All notable changes to yaak will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] — 2026-04-08

### Changed
- Cache mode now uses BM25 fuzzy matching — similar descriptions hit the cache even when wording differs
- Updated landing page, docs, and README with BM25 cache details

---

## [0.1.1] — 2026-04-08

### Added
- `--feedback` flag to open a pre-filled GitHub Issue in your browser with version and platform info
- Troubleshooting section in documentation for zsh glob expansion issues

---

## [0.1.0] — 2026-04-08

### Added
- Multi-language support — UI available in English, German, Spanish, French, Portuguese, Chinese, Japanese, and Korean
- Destructive command override (`--force` / `-f` flag and `allow_destructive` config option) — destructive commands now prompt for confirmation instead of being hard-blocked

---

## [0.0.12] — 2026-04-08

### Added
- Google Gemini as a native API provider with streaming support and model suggestions
- Curl install command in the landing page CTA section for easier onboarding

---

## [0.0.11] — 2026-04-07

### Added
- Doubleword as a provider (`https://api.doubleword.ai/v1`) with suggested models

---

## [0.0.10] — 2026-04-07

### Added
- `-v` / `--version` flag to print version number
- Refine option in the `What next?` prompt — iteratively adjust generated commands without restarting

---

## [0.0.9] — 2026-04-07

### Added
- Documentation page with full CLI reference, sticky sidebar TOC, and flag table
- Curl-based install script (`curl -fsSL https://getyaak.ai/install.sh | bash`) with SHA256 checksum verification
- Mobile hamburger menu for responsive navigation

### Fixed
- API key no longer required for local providers (Ollama, LM Studio, vLLM, LocalAI)

---

## [0.0.8] — 2026-04-07

### Added
- Contextual awareness (`--context`) — auto-detects project type, git branch, and file listing to enrich LLM prompts
- Current working directory always included in system prompt for path-aware commands
- Offline cache mode (`--cache` / `--no-cache`) — reuse cached results for identical descriptions, skip API calls

### Fixed
- Landing page footer license corrected from MIT to Apache-2.0

---

## [0.0.7] — 2026-04-06

### Added
- Streaming LLM output with real-time token display and dot progress indicator
- Command history with `--history` / `-H`, `--last` / `-l`, and `--search` / `-s` for recall, re-execution, and keyword search
- Shell completion generation (`--completions <bash|zsh|fish>`) via `clap_complete`
- Clipboard integration (`--copy` / `-C`) to copy generated commands; interactive prompt now offers Execute / Copy / Abort

### Fixed
- Terminal demo animation bug where all lines after "Thinking..." overwrote the same element

---

## [0.0.6] — 2026-04-06

### Added
- Interactive configuration wizard (`--config` / `-c`) with provider selection, model picker, and API key prompt
- Homebrew install option on landing page (`brew tap hanneshapke/yaak && brew install yaak`)

### Changed
- Refactored `main.rs` into modules (`api`, `config`, `command`, `explain`, `wizard`) for maintainability

---

## [0.0.5] — 2026-04-05

### Added
- Native Anthropic API support with auto-detection based on API base URL or model name
- Automated crates.io publishing in CI on tag push, with version-tag consistency check

### Fixed
- Landing page GitHub links now point to the correct `hanneshapke/yaak` repository

---

## [0.0.4] — 2026-04-05

### Added
- Reverse/explain mode (`-r`/`--reverse`/`-e`/`--explain`) that takes a shell command and returns a detailed visual breakdown with per-token explanations, examples, and caution warnings
- Package metadata (`license`, `homepage`, `repository`) in `Cargo.toml` for crates.io publishing

---

## [0.0.3] — 2026-04-05

### Added
- Safety check that blocks destructive commands (`rm`, `dd`, `mkfs`, `shred`, `wipefs`, etc.) before execution
- Detection of `sudo` variants of dangerous commands in piped and chained command sequences

---

## [0.0.2] — 2026-04-05

### Added
- OS and shell detection injected into LLM system prompt for platform-appropriate commands
- Config file lookup at `~/.config/yaak/config.toml` (XDG-style) with fallback to platform-native config dir

### Changed
- Commands now run in the user's actual shell (`$SHELL`) instead of hardcoded `bash`
- Switched `reqwest` from `native-tls` to `rustls-tls` for easier cross-compilation

### Fixed
- Config file not found on macOS where `dirs::config_dir()` returns `~/Library/Application Support`
- LLM generating GNU/Linux-specific flags (e.g. `find -printf`) on macOS

---

## [0.0.1] — 2026-04-04

### Added
- Natural language to bash command translation via OpenAI-compatible `/v1/chat/completions` endpoint
- Configuration via CLI flags (`--api-key`, `--api-base`, `--model`), environment variables (`YAAK_API_KEY`, `YAAK_API_BASE`, `YAAK_MODEL`), and config file (`~/.config/yaak/config.toml`)
- Configuration resolution order: CLI flags → env vars → config file → defaults
- Interactive confirmation prompt before command execution, with `--yes` / `-y` flag to skip
- Smart command extraction from markdown fenced code blocks and `$ `-prefixed lines
- Colored terminal output for errors, commands, and status messages
- Support for any OpenAI-compatible provider (OpenAI, Ollama, Groq, Together AI, OpenRouter, vLLM, LM Studio, LocalAI)
- Cross-platform CI pipeline with GitHub Actions (lint, test, build)
- Prebuilt binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64)
- Shell completion generation for bash, zsh, and fish via `yaak --completions <shell>`
