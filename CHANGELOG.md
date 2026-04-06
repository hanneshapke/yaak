# Changelog

All notable changes to yaak will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
