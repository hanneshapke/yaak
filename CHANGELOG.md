# Changelog

All notable changes to yaak will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
