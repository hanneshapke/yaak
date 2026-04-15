---
title: "7 yaak Demos That Show What It Can Really Do"
date: 2026-04-15
slug: seven-yaak-demos
description: "A visual tour through yaak's best features — from interactive setup to context-aware commands and instant cache hits."
author: yaak team
---

*You can read documentation all day, but nothing beats seeing a tool in action. I put together seven quick demos that showcase yaak's most powerful features — the ones that turn it from "neat idea" into "I use this ten times a day."*

Here's what I built.

---

## 1. Interactive setup wizard — zero friction onboarding

![yaak config wizard](/demos/output/01-wizard.gif)

First-time setup shouldn't feel like work. Run `yaak --config` and you get an interactive wizard that walks you through:

- **Language selection** — pick from eight supported languages
- **Model provider** — Anthropic, Google Gemini, OpenAI, Ollama, Groq, and more
- **Model configuration** — choose your model and set preferences

No hunting through docs. No editing YAML files. Just answer a few prompts and start using yaak immediately.

```
$ yaak --config
```

The wizard detects existing config and lets you update individual settings without starting over. Perfect for when you want to switch models or try a different provider.

---

## 2. Kill processes like a human would ask

![yaak kill port](/demos/output/02-kill-port.gif)

"What's using port 3000?" is the kind of thing I used to Google, copy a `lsof` command, run it, get a PID, then remember the `kill` syntax. With yaak, I just ask:

```
$ yaak find what's using port 3000
  Command: lsof -ti:3000 | xargs kill -9
  Execute? [y/N]
```

yaak streams the command, waits for confirmation, and you're done. No context switching, no Stack Overflow, no remembering flags.

This is the headline feature — plain English to executable shell commands in seconds.

---

## 3. Spin up services without memorizing Docker flags

![yaak docker postgres](/demos/output/03-docker-postgres.gif)

Docker commands are powerful but verbose. Starting a Postgres container with custom password and port? That's a wall of flags you definitely don't have memorized:

```
$ yaak run postgres in docker with password mysecret on port 5433
  Command: docker run --name postgres -e POSTGRES_PASSWORD=mysecret -p 5433:5432 -d postgres
  Execute? [y/N]
```

yaak translates your intent into the right `docker run` command with all the flags in the right order. Works for any service — Redis, MongoDB, Nginx, whatever you need.

---

## 4. Git log queries that actually make sense

![yaak git log](/demos/output/04-git-log.gif)

Git has incredible power buried under flags you can never remember. Want a brief summary of commits from the last two days?

```
$ yaak show commits from the last 2 days. show a brief summary
  Command: git log --since="2 days ago" --oneline --no-merges
  Execute? [y/N]
```

yaak knows `--since`, `--oneline`, `--no-merges`, and hundreds of other git flags. You just describe what you want to see.

---

## 5. Project-aware commands with --context

![yaak context build](/demos/output/05-context-build.gif)

Pass `--context` and yaak reads your project structure — `Cargo.toml`, `package.json`, `go.mod`, `Dockerfile`, git branch, directory layout — and generates commands that understand what kind of project you're in:

```
$ yaak --context build and test the project
  Command: cargo build && cargo test
  Execute? [y/N]
```

No need to specify "use cargo" or "this is a Rust project." yaak detects it and does the right thing automatically. Works for npm, Go, Python, Docker, and more.

Context is opt-in by design. Simple queries stay fast, but when you need project awareness, it's there.

---

## 6. Explain scary commands before you run them

![yaak explain](/demos/output/06-explain.gif)

Found a Stack Overflow answer with a command you don't fully trust? Use `--explain` to break it down:

```
$ yaak --explain 'find . -name "*.log" -mtime +30 -delete'
```

yaak walks through each part of the command, explains what it does, flags destructive operations, and tells you exactly what will happen before you execute anything.

Perfect for:
- Learning new commands
- Code review
- Debugging shell scripts
- Making sure you don't accidentally nuke production logs

---

## 7. Cache results for instant replay

![yaak cache](/demos/output/07-cache.gif)

How many times have I asked the same question? Weather forecasts, file conversions, database queries — some things I look up over and over. With `--cache`, yaak remembers:

```
$ yaak get the weather forecasts for San Francisco, Tokyo, and London. save as json.
  [15 seconds of API calls and processing]

$ yaak --cache get the weather forecasts for San Francisco, Tokyo, and London. save as json.
  [instant response from cache]
```

The first run (without `--cache`) fetches data from weather APIs and processes it. The second run with `--cache`? Instant replay from the cached result. Cache lookup uses exact match first, then fuzzy BM25 matching, so minor variations still hit the cache.

Great for expensive operations like multi-city weather comparisons, video processing, or any API-heavy task I repeat throughout the day.

---

## Try them yourself

Install yaak and try these commands yourself:

```
$ curl -fsSL https://getyaak.ai/install.sh | bash
```

See the [installation docs](https://getyaak.ai/docs/install) for other options.

Set `YAAK_API_KEY` (or run `yaak --config` for the wizard), and you're ready to go.

*yaak is open source under the Apache-2.0 license. → [getyaak.ai](https://getyaak.ai) · [GitHub](https://github.com/hanneshapke/yaak)*