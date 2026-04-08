# yaak — Yet Another AI for the Kommandozeile

Translate natural language into bash commands using any OpenAI-compatible LLM.

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

# Homebrew (macOS/Linux)
brew tap hanneshapke/yaak
brew install yaak

# Or via Cargo
cargo install yaak
```

## Quick setup

Run the interactive config wizard:

```
$ yaak --config
yaak configuration wizard
────────────────────────────────────────

? Select your API provider:
  1. OpenAI
  2. Anthropic
❯ 3. Ollama
  4. Groq
  5. Together AI
  6. OpenRouter
  7. LM Studio
  8. vLLM
  9. LocalAI

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
```

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
| `--cache`               |       | Reuse cached result for identical queries |
| `--no-cache`            |       | Force fresh API call, ignore cache       |
| `--history`             | `-H`  | Show recent command history              |
| `--last`                | `-l`  | Re-execute the most recent command       |
| `--search KEYWORD`      | `-s`  | Search command history by keyword        |
| `--version`             | `-v`  | Print version number                     |
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

# Reuse a cached result (no API call)
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

yaak blocks destructive commands (`rm`, `dd`, `mkfs`, `shred`, etc.) from being executed, including `sudo` variants and piped/chained sequences.

## Compatible providers

yaak auto-detects the provider from the API base URL and uses the appropriate request format.

**Anthropic (native Messages API):**

- **Anthropic** — `https://api.anthropic.com/v1`

```bash
yaak -u https://api.anthropic.com/v1 -k sk-ant-... find all large log files
```

**OpenAI-compatible** (`/v1/chat/completions`):

- **OpenAI** — `https://api.openai.com/v1`
- **Ollama** — `http://localhost:11434/v1` (no key needed)
- **Groq** — `https://api.groq.com/openai/v1`
- **Together AI** — `https://api.together.xyz/v1`
- **OpenRouter** — `https://openrouter.ai/api/v1`
- **LM Studio** — `http://localhost:1234/v1`
- **vLLM / LocalAI** — your local URL

## License

Apache-2.0
