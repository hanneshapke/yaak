---
title: "10 Things yaak Can Do (That You Probably Didn't Know)"
date: 2026-04-11
slug: 10-things-yaak-can-do
description: "A whirlwind tour of yaak's most useful tricks — from fuzzy cache lookups to reverse explanations and project-aware context."
author: yaak team
---

*Most people install yaak, type a few questions, and stop there. That's fine — it works great as a "what's the bash syntax for X" tool out of the box. But yaak has a handful of features that turn it from "occasional helper" into "the thing I reach for first."*

Here are ten of them.

## 1. Translate plain English into shell commands

The headline feature. Type what you want, no quoting needed:

```
$ yaak find all rust files modified this week
  Command: find . -name "*.rs" -mtime -7
  Execute? [Y/n]
```

yaak streams the model response, shows you the command, and waits for confirmation before running anything. Three seconds from thought to execution.

## 2. Reverse mode: explain a command you don't understand

Got a scary one-liner from a Stack Overflow answer? Let yaak explain it:

```
$ yaak --reverse find . -name "*.log" -mtime +30 -delete
```

yaak breaks the command down piece by piece, flags any destructive operations, and tells you exactly what will happen before you run it. Great for code review, learning, and not blowing up production.

## 3. Cache results so you stop re-asking

How many times have you Googled the same `ffmpeg` command? With `--cache`, yaak remembers:

```
$ yaak --cache convert mov to mp4 with h264
```

The next time you ask the same question — or even a fuzzy variant — yaak returns the cached result instantly. Cache lookup uses exact match first, then BM25 fuzzy matching, so "convert .mov to .mp4" will still hit the cache from "convert mov to mp4 with h264."

## 4. Browse and re-run history

Every command yaak generates can be saved to history:

```
$ yaak --history          # browse past prompts and commands
$ yaak --last             # re-run the last generated command
$ yaak --search ffmpeg    # search history by keyword
```

It's a personal command knowledge base that grows with you.

## 5. Project-aware context

Pass `--context` and yaak detects what kind of project you're in — Cargo, npm, Go, Docker, Python, and more — along with your git branch and file structure. Suddenly "build the project" or "run the tests" do the right thing automatically:

```
$ yaak --context build the project
  Command: cargo build --release
```

Context is opt-in by design. Simple queries stay fast, but when you need it, it's there.

## 6. Bring your own model — any model

yaak isn't tied to a single provider. Native support for Anthropic and Google Gemini, plus any endpoint that speaks the OpenAI chat completions protocol:

- Anthropic
- Google Gemini
- OpenAI
- Ollama (local)
- Groq
- Together AI
- OpenRouter
- LM Studio (local)
- vLLM (self-hosted)
- LocalAI (self-hosted)

Run `yaak --config`, pick your provider, paste your key, done. Switching providers later is one config file away.

## 7. Use yaak in eight languages

yaak speaks English, Deutsch, Español, Français, Português, 中文, 日本語, and 한국어. Auto-detects from your system locale, or set it explicitly:

```
$ yaak --language de finde alle rust dateien dieser woche
$ yaak --language es buscar archivos rust modificados esta semana
```

The CLI prompts, error messages, and all interaction surfaces are localized — not just the model input.

## 8. Safety guards for destructive commands

yaak refuses to auto-execute commands containing `rm`, `dd`, `mkfs`, `shred`, and their privilege-escalation variants (`sudo`, `doas`, `pkexec` — including flags and `env` wrappers). Even with `-y` (auto-execute), destructive commands always require explicit confirmation. Non-destructive commands that use `sudo` or other escalation tools get a separate privilege warning so you always know when a command will run as root. You can review every generated command before it runs, every time.

It's the safety net you didn't know you wanted until the first time it stops a runaway `rm -rf`.

## 9. Copy to clipboard instead of executing

Sometimes you don't want to run the command — you want to paste it somewhere else. A script, a Slack message, a colleague's terminal. Use `-C`:

```
$ yaak -C tar a directory with gzip and exclude node_modules
  Command copied to clipboard.
```

No execution, no prompts. Just the command, ready to paste.

## 10. Refine a command without starting over

Not quite what you wanted? Pick "Refine" at the prompt and add a follow-up:

```
$ yaak find large files
  Command: find . -type f -size +10M
  ? What next?: Refine
  > only show the top 5 sorted by size
  Command: find . -type f -size +10M -exec du -h {} + | sort -rh | head -5
```

yaak keeps the conversation context so you can iterate without re-typing the original prompt. It's a cheap, fast way to nail down the exact command you need.

-----

## A 10-second install

```
$ brew install hanneshapke/yaak/yaak
# or
$ curl -fsSL https://getyaak.ai/install.sh | bash
```

Set `YAAK_API_KEY` (or run `yaak --config` for the wizard), and start typing.

*yaak is open source under the Apache-2.0 license. → [getyaak.ai](https://getyaak.ai) · [GitHub](https://github.com/hanneshapke/yaak)*
