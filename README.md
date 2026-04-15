# yaak — Yet Another AI for the Kommandozeile

[![Crates.io](https://img.shields.io/crates/v/yaak.svg)](https://crates.io/crates/yaak)
[![License](https://img.shields.io/crates/l/yaak.svg)](https://github.com/hanneshapke/yaak/blob/main/LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/hanneshapke/yaak.svg?style=social)](https://github.com/hanneshapke/yaak/stargazers)

[![Homebrew](https://img.shields.io/badge/homebrew-hanneshapke%2Fyaak-orange)](https://github.com/hanneshapke/homebrew-yaak)
[![AUR](https://img.shields.io/aur/version/yaak-cli)](https://aur.archlinux.org/packages/yaak-cli)
[![Nix](https://img.shields.io/badge/nix-flake-5277C3)](https://github.com/hanneshapke/yaak#nix--nixos-flakes)
[![Scoop](https://img.shields.io/badge/scoop-yaak-blue)](https://github.com/hanneshapke/scoop-yaak)

Translate natural language into bash commands using any OpenAI-compatible LLM. Supports 8 languages.

```
$ yaak list all rust files larger than 1MB
  Thinking ....
  Command: find . -name "*.rs" -size +1M -ls
? What next?: Execute / Refine / Copy / Abort
```

## Install

```bash
# Quick install (macOS / Linux)
curl -fsSL https://getyaak.ai/install.sh | bash

# Homebrew (macOS / Linux)
brew install hanneshapke/yaak/yaak

# Arch Linux (AUR) — build from source or prebuilt binary
yay -S yaak-cli          # or: paru -S yaak-cli
yay -S yaak-cli-bin      # prebuilt release binary

# Nix / NixOS (flakes)
nix profile install github:hanneshapke/yaak
# or one-shot:
nix run github:hanneshapke/yaak -- --help

# Debian / Ubuntu
curl -LO https://github.com/hanneshapke/yaak/releases/latest/download/yaak_0.1.3_amd64.deb
sudo dpkg -i yaak_0.1.3_amd64.deb

# Windows (Scoop)
scoop bucket add yaak https://github.com/hanneshapke/scoop-yaak
scoop install yaak

# Or via Cargo
cargo install yaak
```

Package definitions for AUR, Nix and Scoop live under
[`packaging/`](packaging/README.md). A `.deb` package is attached to each
[GitHub release](https://github.com/hanneshapke/yaak/releases).

## Quick setup

Run the interactive config wizard:

```
$ yaak --config
yaak configuration wizard
────────────────────────────────────────

? Select your language:
  1. English    5. Português
  2. Deutsch    6. 中文
  3. Español    7. 日本語
❯ 4. Français   8. 한국어

? Select your API provider:
  1. OpenAI
  2. Anthropic
  3. Google Gemini
❯ 4. Ollama
  5. Groq
  6. Together AI
  7. OpenRouter
  8. LM Studio
  9. vLLM
  10. LocalAI

? Select a model: gemma3:4b

✓ Config written to ~/.config/yaak/config.toml
```

## Configuration

yaak resolves settings in this priority order: **CLI flags → environment variables → config file → defaults**.

### Config file

Run `yaak --config` to generate the config interactively, or create it manually:

```bash
mkdir -p ~/.config/yaak
cp config.example.toml ~/.config/yaak/config.toml
```

On macOS, yaak also checks `~/Library/Application Support/yaak/config.toml`.

### Environment variables

```bash
export YAAK_API_BASE="https://api.openai.com/v1"
export YAAK_API_KEY="sk-..."
export YAAK_MODEL="gpt-4o-mini"
export YAAK_LANGUAGE="en"  # en, de, es, fr, pt, zh, ja, ko
```

## Multi-language support

yaak supports 8 languages for its UI (prompts, errors, menus, labels):

| Code | Language   |
|------|------------|
| `en` | English    |
| `de` | Deutsch    |
| `es` | Español    |
| `fr` | Français   |
| `pt` | Português  |
| `zh` | 中文        |
| `ja` | 日本語      |
| `ko` | 한국어      |

Language is resolved in this order: `--language` flag → `YAAK_LANGUAGE` env var → `language` in config file → system locale → English.

```bash
# Set via CLI flag
yaak -L de list files in current directory

# Set via environment variable
export YAAK_LANGUAGE=es

# Set via config file
# Add to ~/.config/yaak/config.toml:
#   language = "fr"

# The config wizard includes a language selection step
yaak --config
```

When a non-English language is selected, yaak also instructs the LLM to respond with explanations in that language (in `--explain` mode), while keeping the structured output format intact.

## Usage

```
yaak <description of what you want to do>
```

### Options

| Flag                    | Short | Description                              |
|-------------------------|-------|------------------------------------------|
| `--config`              | `-c`  | Interactive configuration wizard         |
| `--api-base URL`        | `-u`  | API base URL                             |
| `--api-key KEY`         | `-k`  | API key                                  |
| `--model NAME`          | `-m`  | Model name                               |
| `--yes`                 | `-y`  | Skip confirmation prompt                 |
| `--reverse`             | `-r`  | Explain a command instead of generating  |
| `--explain`             | `-e`  | Alias for --reverse                      |
| `--copy`                | `-C`  | Copy generated command to clipboard      |
| `--context`             |       | Include project type and file listing in prompt |
| `--cache`               |       | Reuse cached result (exact or BM25 fuzzy match) |
| `--no-cache`            |       | Force fresh API call, ignore cache       |
| `--history`             | `-H`  | Show recent command history              |
| `--last`                | `-l`  | Re-execute the most recent command       |
| `--search KEYWORD`      | `-s`  | Search command history by keyword        |
| `--language LANG`       | `-L`  | UI language: en, de, es, fr, pt, zh, ja, ko |
| `--version`             | `-v`  | Print version number                     |
| `--update`              | `-U`  | Update yaak to the latest version        |
| `--feedback`            |       | Open the feedback page in your browser   |
| `--completions SHELL`   |       | Generate shell completions (bash/zsh/fish)|

### Examples

```bash
# Basic usage
yaak find all PDFs modified in the last week

# Use a local Ollama model
yaak -u http://localhost:11434/v1 -m llama3 show disk usage by directory

# Pipe-friendly (skip confirmation)
yaak -y count lines of code in src/

# Copy to clipboard instead of executing
yaak --copy compress all png files in this directory

# Explain a command
yaak --explain 'find . -name "*.log" -mtime +30 -delete'

# View command history
yaak --history

# Re-run the last generated command
yaak --last

# Search history
yaak --search "docker"

# Include project context for smarter results
yaak --context build the project

# Reuse a cached result (exact or fuzzy BM25 match, no API call)
yaak --cache list all docker containers
```

### Shell completions

Generate completions for your shell:

```bash
# Bash
yaak --completions bash > ~/.bash_completion.d/yaak

# Zsh
yaak --completions zsh > ~/.zfunc/_yaak

# Fish
yaak --completions fish > ~/.config/fish/completions/yaak.fish
```

### Safety

yaak blocks destructive commands (`rm`, `dd`, `mkfs`, `shred`, etc.) from being executed, including privilege-escalation variants (`sudo`, `doas`, `pkexec` — with flags, `env` wrappers, etc.) and piped/chained sequences. Commands that use privilege escalation without being destructive trigger a separate confirmation prompt warning about elevated privileges.

## Compatible providers

yaak auto-detects the provider from the API base URL and uses the appropriate request format.

**Anthropic (native Messages API):**

- **Anthropic** — `https://api.anthropic.com/v1`

```bash
yaak -u https://api.anthropic.com/v1 -k sk-ant-... find all large log files
```

**Google Gemini (native Gemini API):**

- **Google Gemini** — `https://generativelanguage.googleapis.com/v1beta`

```bash
yaak -u https://generativelanguage.googleapis.com/v1beta -k AIza... find all large log files
```

**OpenAI-compatible** (`/v1/chat/completions`):

- **OpenAI** — `https://api.openai.com/v1`
- **Ollama** — `http://localhost:11434/v1` (no key needed)
- **Groq** — `https://api.groq.com/openai/v1`
- **Together AI** — `https://api.together.xyz/v1`
- **OpenRouter** — `https://openrouter.ai/api/v1`
- **Doubleword** — `https://api.doubleword.ai/v1`
- **LM Studio** — `http://localhost:1234/v1`
- **vLLM / LocalAI** — your local URL

## License

Apache-2.0
