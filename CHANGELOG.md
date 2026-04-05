# Changelog

All notable changes to yaak will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Shell completion generation for bash, zsh, and fish via `yaak --completions <shell>`

---

## [0.3.0] — 2026-04-01

### Added
- Configuration file support at `~/.config/yaak/config.toml`
- New `--model` / `-m` flag to override the model per invocation
- `--api-base` / `-u` flag for switching providers on the fly
- Automatic extraction of commands from markdown fenced code blocks in LLM responses
- Environment variable support: `YAAK_API_KEY`, `YAAK_API_BASE`, `YAAK_MODEL`

### Changed
- Configuration resolution now follows CLI flags → env vars → config file → defaults
- Improved error messages with colored output for missing API keys

### Fixed
- Commands prefixed with `$ ` are now stripped correctly before execution

---

## [0.2.0] — 2026-03-15

### Added
- Cross-platform CI pipeline with GitHub Actions (lint, test, build)
- Prebuilt binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64)
- SHA256 checksum file included in every GitHub Release
- Colored terminal output using the `colored` crate
- Interactive confirmation prompt via `dialoguer`

### Changed
- Switched HTTP client from `ureq` to `reqwest` for better TLS and async support
- Exit code now mirrors the executed command's exit code

### Fixed
- Panic when the LLM returns an empty choices array
- Trailing whitespace in generated commands causing execution failures

---

## [0.1.0] — 2026-03-01

### Added
- Initial release of yaak
- Natural language to bash command translation via OpenAI-compatible `/v1/chat/completions` endpoint
- Interactive confirmation before command execution
- `--yes` / `-y` flag to skip confirmation
- `--api-key` / `-k` flag for API key
- Support for OpenAI, Ollama, Groq, Together AI, OpenRouter, vLLM, LM Studio, and LocalAI
- System prompt tuned for concise, portable shell commands
- README with installation and usage instructions
