# Yaak — Product Gap Analysis

> Reviewed: 2026-04-06 | Current version: 0.0.6

Yaak is a CLI tool that translates natural language into bash commands using LLMs. This document identifies missing functionality that would meaningfully improve the product for its target users.

---

## Priority 1 — High Impact, Core UX

### 1. Command History & Recall
**Gap:** Yaak is entirely stateless. Every invocation is independent with no memory of past commands.

**Why it matters:** Users frequently want to re-run or modify recent commands. Without history, they must re-type descriptions or rely on shell history (which only captures the `yaak ...` invocation, not the generated command).

**Suggestion:**
- Persist generated commands to `~/.local/share/yaak/history.jsonl` (description, command, timestamp, provider)
- Add `yaak --history` / `-H` to list recent generations
- Add `yaak --last` / `-l` to re-execute the most recent command
- Add `yaak --search <term>` to fuzzy-search past commands

---

### 2. Streaming Output
**Gap:** The LLM response is fetched as a single blocking request. For slower models or providers, the user sees "Thinking..." with no feedback until the full response arrives.

**Why it matters:** Streaming is table-stakes for LLM-based tools. Long waits with no progress feel broken, especially for explain mode which returns multi-paragraph responses.

**Suggestion:**
- Use streaming (`stream: true` for OpenAI, `stream: true` for Anthropic) with incremental terminal rendering
- Show tokens as they arrive for explain mode
- Show a spinner or elapsed timer for generate mode

---

### 3. Shell Completions (Broken/Missing)
**Gap:** The v0.0.1 changelog mentions `yaak --completions <shell>` for bash/zsh/fish, but this flag does not exist in the current `Args` struct or CLI definition.

**Why it matters:** Shell completions are listed as a shipped feature but appear to have been lost during the modular refactor in v0.0.6.

**Suggestion:**
- Re-add `--completions <shell>` using `clap_complete` to generate completion scripts
- Document installation in README

---

### 4. Retry & Timeout Configuration
**Gap:** No configurable timeout or retry logic. The `reqwest::blocking::Client` uses default timeouts. If the API is slow or down, the tool hangs indefinitely.

**Why it matters:** Local LLM providers (Ollama, vLLM) can be slow. Cloud providers can have outages. Users need control.

**Suggestion:**
- Add `timeout` config option (default: 30s)
- Add `--timeout` CLI flag
- Implement a single retry with backoff on transient failures (5xx, timeout)

---

### 5. Multi-command / Pipeline Mode
**Gap:** Yaak generates a single command per invocation. Complex tasks requiring multiple steps get jammed into a single `&&`-chained line that's hard to review.

**Why it matters:** Real-world tasks often require multi-step workflows (e.g., "set up a Python virtualenv, install requirements, and run tests").

**Suggestion:**
- Allow the LLM to return numbered steps
- Display each step for individual confirmation
- Execute sequentially, stopping on failure
- Add `--steps` flag to enable this mode

---

## Priority 2 — Important Gaps

### 6. Clipboard Integration
**Gap:** No way to copy the generated command to clipboard without executing it.

**Why it matters:** Many users want to inspect, modify, or paste commands into another terminal/script rather than execute immediately.

**Suggestion:**
- Add `--copy` / `-C` flag to copy to clipboard (via `pbcopy`/`xclip`/`wl-copy`)
- Add "Copy" as a third option alongside Execute/Abort in the confirmation prompt

---

### 7. Custom System Prompt / Persona Override
**Gap:** System prompts are hardcoded. Users cannot customize the LLM's behavior, persona, or constraints.

**Why it matters:** Power users may want domain-specific prompts (e.g., "prefer Docker commands", "use nix tools", "output PowerShell").

**Suggestion:**
- Add `system_prompt` field in `config.toml`
- Add `--system-prompt` CLI flag
- Support `~/.config/yaak/prompt.txt` file override

---

### 8. Temperature / Creativity Control
**Gap:** Temperature is hardcoded to `0.0`. Users cannot adjust it.

**Why it matters:** Zero temperature is good for deterministic commands, but explain mode and creative use cases benefit from higher temperature.

**Suggestion:**
- Add `--temperature` / `-t` flag (default: 0.0 for generate, 0.3 for explain)
- Add `temperature` to config file

---

### 9. Non-Bash Shell Support
**Gap:** While Yaak detects `$SHELL` and mentions it in the system prompt, the destructive command detection, command extraction, and execution logic are all bash-centric. PowerShell, fish, and nushell have different syntax.

**Why it matters:** Windows users (without WSL), fish users, and nushell users will get commands that don't work in their shell.

**Suggestion:**
- Adapt destructive command patterns per shell
- Test command extraction with fish/PowerShell output formats
- Add `--shell` flag to override detection
- Update system prompt to be more explicit about shell syntax requirements

---

### 10. Dry-Run / Preview Mode
**Gap:** No way to generate a command and print it without any interactive prompt.

**Why it matters:** Useful for scripting, piping output, and CI/automation contexts. Also useful for `yaak "..." | pbcopy` workflows.

**Suggestion:**
- Add `--dry-run` / `-n` flag that prints the command to stdout with no prompt
- When stdout is not a TTY, default to dry-run mode (like `grep --color=auto`)

---

### 11. Contextual Awareness (Working Directory)
**Gap:** The system prompt tells the LLM the OS and shell, but not the current directory, available files, or installed tools.

**Why it matters:** "List the CSV files" should generate `ls *.csv` only if CSV files exist. "Build the project" should detect Makefile vs Cargo.toml vs package.json.

**Suggestion:**
- Add `--context` flag to include `ls` output or `git status` in the user message
- Auto-detect project type (Cargo.toml, package.json, Makefile, etc.) and include as context
- Keep this opt-in to avoid leaking sensitive filenames

---

## Priority 3 — Nice to Have

### 12. Saved Command Templates / Aliases
**Gap:** No way to save frequently-used descriptions as shortcuts.

**Suggestion:**
- Support `[aliases]` section in config: `deploy = "build and push Docker image to registry"`
- Run via `yaak @deploy`

---

### 13. Output Formatting Options
**Gap:** Explain mode outputs colored terminal text only. No structured output format.

**Suggestion:**
- Add `--format json` for machine-readable output
- Add `--format plain` for uncolored text (useful in pipes)
- Detect non-TTY stdout and auto-disable colors

---

### 14. Token Usage / Cost Tracking
**Gap:** No visibility into API token consumption or cost.

**Suggestion:**
- Show token count after each request (opt-in via `--verbose` or `-v`)
- Track cumulative usage in history file

---

### 15. Offline / Cache Mode
**Gap:** Every invocation requires an API call, even for identical descriptions.

**Suggestion:**
- Cache description->command mappings locally
- Add `--cache` flag to reuse cached results for identical descriptions
- Add `--no-cache` to force fresh generation

---

### 16. Multi-turn Conversation / Refinement
**Gap:** No way to refine a generated command. If the LLM produces something close but not right, the user must start over with a more detailed description.

**Suggestion:**
- Add `--refine` / `-R` flag that includes the last generated command as context
- Or implement a simple REPL mode: `yaak --interactive` / `-i`

---

### 17. Plugin / Hook System
**Gap:** No extensibility mechanism. Users cannot add pre/post-execution hooks or custom command transformers.

**Suggestion:**
- Support `pre_execute` and `post_execute` hook scripts in config
- Allow custom command validators beyond the built-in destructive check

---

### 18. Proxy Support
**Gap:** No explicit proxy configuration. While `reqwest` may respect `HTTP_PROXY`/`HTTPS_PROXY` environment variables, this is not documented or configurable.

**Suggestion:**
- Document env var proxy support
- Add `proxy` field in config for explicit configuration
- Support authenticated proxies

---

### 19. Destructive Command Allow-List
**Gap:** The safety blocklist is all-or-nothing. There's no way to override it for users who intentionally want to run `rm` commands.

**Suggestion:**
- Add `--force` / `-f` flag to bypass safety checks (with a prominent warning)
- Add `allow_destructive` config option
- Or: instead of blocking, require double-confirmation for destructive commands

---

### 20. Man Page Generation
**Gap:** No man page. `man yaak` returns nothing.

**Suggestion:**
- Generate man page from clap using `clap_mangen`
- Include in release artifacts and Homebrew formula

---

## Summary Table

| # | Feature | Priority | Effort | Impact |
|---|---------|----------|--------|--------|
| 1 | Command History & Recall | P1 | Medium | High |
| 2 | Streaming Output | P1 | Medium | High |
| 3 | Shell Completions (fix regression) | P1 | Low | Medium |
| 4 | Retry & Timeout Config | P1 | Low | Medium |
| 5 | Multi-command Pipeline Mode | P1 | High | High |
| 6 | Clipboard Integration | P2 | Low | Medium |
| 7 | Custom System Prompt | P2 | Low | Medium |
| 8 | Temperature Control | P2 | Low | Low |
| 9 | Non-Bash Shell Support | P2 | Medium | Medium |
| 10 | Dry-Run / Preview Mode | P2 | Low | Medium |
| 11 | Contextual Awareness | P2 | Medium | High |
| 12 | Saved Aliases/Templates | P3 | Medium | Medium |
| 13 | Output Formatting (JSON/plain) | P3 | Low | Low |
| 14 | Token Usage Tracking | P3 | Low | Low |
| 15 | Offline Cache | P3 | Medium | Medium |
| 16 | Multi-turn Refinement | P3 | High | High |
| 17 | Plugin/Hook System | P3 | High | Medium |
| 18 | Proxy Support | P3 | Low | Low |
| 19 | Destructive Command Allow-List | P3 | Low | Medium |
| 20 | Man Page | P3 | Low | Low |

---

## Recommended Next Steps

1. **Fix the shell completions regression** (#3) — low effort, already documented as shipped
2. **Add command history** (#1) — highest user-value feature for a v0.1.0 milestone
3. **Add streaming** (#2) — critical UX improvement, especially for explain mode
4. **Add `--dry-run` and clipboard** (#10, #6) — small additions that unlock scripting workflows
5. **Add timeout config** (#4) — prevents the tool from hanging on unresponsive providers
