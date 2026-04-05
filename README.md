# yaak

Translate natural language into bash commands using any OpenAI-compatible LLM.

```
$ yaak list all rust files larger than 1MB
  Command: find . -name "*.rs" -size +1M -ls
Execute? [Y/n] y
```

## Install

```bash
cargo install --path .
```

## Configuration

yaak resolves settings in this priority order: **CLI flags → environment variables → config file → defaults**.

### Config file

Copy the example and edit it:

```bash
mkdir -p ~/.config/yaak
cp config.example.toml ~/.config/yaak/config.toml
```

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

| Flag              | Short | Description                    |
|-------------------|-------|--------------------------------|
| `--api-base URL`  | `-u`  | API base URL                   |
| `--api-key KEY`   | `-k`  | API key                        |
| `--model NAME`    | `-m`  | Model name                     |
| `--yes`           | `-y`  | Skip confirmation prompt       |

### Examples

```bash
# Basic usage
yaak find all PDFs modified in the last week

# Use a local Ollama model
yaak -u http://localhost:11434/v1 -m llama3 show disk usage by directory

# Pipe-friendly (skip confirmation)
yaak -y count lines of code in src/
```

## Compatible providers

yaak auto-detects the provider from the API base URL and uses the appropriate request format.

**Anthropic (native Messages API):**

- **Anthropic** — `https://api.anthropic.com/v1`

```bash
yaak -u https://api.anthropic.com/v1 -k sk-ant-... find all large log files
```

**OpenAI-compatible** (`/v1/chat/completions`):

- **OpenAI** — `https://api.openai.com/v1`
- **Ollama** — `http://localhost:11434/v1` (no key needed, use `-k none`)
- **Together AI** — `https://api.together.xyz/v1`
- **OpenRouter** — `https://openrouter.ai/api/v1`
- **Groq** — `https://api.groq.com/openai/v1`
- **LM Studio** — `http://localhost:1234/v1`
- **vLLM / LocalAI** — your local URL
