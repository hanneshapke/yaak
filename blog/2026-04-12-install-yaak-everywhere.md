---
title: "Install yaak Everywhere: AUR, Nix, Scoop, .deb, and More"
date: 2026-04-12
slug: install-yaak-everywhere
description: "yaak is now available on AUR for Arch Linux, Nix for declarative systems, Scoop for Windows, and as a .deb for Debian/Ubuntu — plus Homebrew, Cargo, and curl. Seven ways to install, all automated."
author: yaak team
---

*The number-one request since launch wasn't a feature. It was: "How do I install this on my machine?"*

yaak started with two install paths: `cargo install yaak` and a Homebrew tap. That covers macOS developers and anyone comfortable with the Rust toolchain — but it left Arch users reaching for `makepkg`, Nix users wondering where the flake was, Debian admins wanting a `.deb`, and Windows users staring at a shell script that assumed `/bin/bash` existed.

Fair enough. Today yaak ships to **seven distribution channels**, all updated automatically the moment we tag a release. Here's the full list.

## curl — the universal one-liner

Still the fastest path from zero to running. Downloads the latest binary, verifies the SHA256 checksum, and installs to `~/.local/bin`:

```bash
curl -fsSL https://getyaak.ai/install.sh | bash
```

Works on any macOS or Linux box with `curl` and `tar`. Pin a version or redirect the install:

```bash
YAAK_VERSION=v0.1.3 curl -fsSL https://getyaak.ai/install.sh | bash
YAAK_INSTALL_DIR=/usr/local/bin curl -fsSL https://getyaak.ai/install.sh | bash
```

No package manager opinions. No runtime dependencies. Just a binary in your PATH.

## Homebrew — for the macOS/Linux crowd

```bash
brew install hanneshapke/yaak/yaak
```

Tracks releases via a custom tap. `brew upgrade yaak` does what you'd expect. This has been around since day one and it's still the smoothest install on macOS.

## AUR — because Arch users were asking

Two packages, because the Arch community has strong opinions about how software should arrive:

```bash
# Build from source — the way Tux intended
yay -S yaak-cli

# Or grab the prebuilt binary — faster, same result
yay -S yaak-cli-bin
```

(The packages are named `yaak-cli` because `yaak` was already taken on the AUR by an unrelated project. Both packages install the `yaak` binary — the `-cli` is just the AUR package name.)

The source package (`yaak-cli`) downloads the release tarball and runs `cargo build --release` on your machine. The binary package (`yaak-cli-bin`) grabs the prebuilt x86_64 Linux binary from GitHub Releases. Both install shell completions for bash, zsh, and fish.

Works with `yay`, `paru`, `pikaur`, or plain `makepkg -si` if you're a purist.

The PKGBUILDs are maintained in the yaak repo and pushed to the AUR automatically on every release — version bumps, hash updates, `.SRCINFO` generation, all handled by CI.

## Nix — declarative and reproducible

```bash
# Try it without installing anything permanently
nix run github:hanneshapke/yaak -- find all log files larger than 100MB

# Or install it to your profile
nix profile install github:hanneshapke/yaak
```

The flake exposes `packages.<system>.yaak` for x86_64 and aarch64 on both Linux and macOS. Shell completions come via `installShellFiles`. A `default.nix` shim is included for anyone not using flakes yet.

Here's what's nice about the Nix setup: the flake reads `version` directly from `Cargo.toml` and pins dependencies via `Cargo.lock`. There's no separate version file to bump. Every tagged release is immediately available — no hash-updating ritual, no waiting for a nixpkgs PR to merge.

NixOS users can add the flake as an input to their system configuration and drop `yaak` into `environment.systemPackages`. Home Manager works too.

## Debian / Ubuntu — a proper `.deb`

```bash
curl -LO https://github.com/hanneshapke/yaak/releases/latest/download/yaak_0.1.3_amd64.deb
sudo dpkg -i yaak_0.1.3_amd64.deb
```

A `.deb` package ships with every GitHub release, built via `cargo-deb` as part of the CI pipeline. It includes the binary, license, and shell completions for bash, zsh, and fish. Works on Debian, Ubuntu, Pop!_OS, Linux Mint, and anything else that speaks `dpkg`.

No PPA or APT repository yet — for now it's a direct download from the release page. We're looking at hosting a proper APT repo so `sudo apt install yaak` and `apt upgrade` work out of the box (see "What's next" below).

## Scoop — Windows gets a seat at the table

```powershell
# Add the bucket (one-time)
scoop bucket add yaak https://github.com/hanneshapke/scoop-yaak

# Install
scoop install yaak
```

The Scoop manifest tracks the prebuilt Windows x86_64 binary. `scoop update yaak` picks up new releases. The manifest is generated entirely by CI — no manual hash copying.

## Cargo — build it yourself

```bash
cargo install yaak
```

The original install method. Builds from source using whatever Rust toolchain you have. Works on any platform Rust supports, including architectures where we don't ship prebuilt binaries.

## The release pipeline

We didn't want "add a package manager" to mean "add a manual step to every release." So all seven channels are fully automated. When we push a version tag, CI does the following without human intervention:

1. Builds release binaries for Linux (x86_64), macOS (x86_64 + aarch64), and Windows (x86_64).
2. Builds a `.deb` package with `cargo-deb`, shell completions included.
3. Publishes everything to GitHub Releases with SHA256 checksums.
4. Pushes the crate to crates.io.
5. Updates the Homebrew formula in the tap repo.
6. Patches the AUR PKGBUILDs with fresh version and hashes, generates `.SRCINFO`, pushes to both AUR repos.
7. Generates a Scoop manifest with the Windows zip hash and pushes to the bucket repo.
8. Verifies the Nix flake still builds.

Tag, push, walk away. Seven distribution channels, zero manual steps.

## What's next

A few things still on the list:

- **APT repository** — so Debian/Ubuntu users can `sudo apt install yaak` and get updates via `apt upgrade` instead of downloading each `.deb` by hand.
- **nixpkgs upstream** — so yaak is available via `nix-env -iA nixpkgs.yaak` without adding a flake input.
- **winget** — the other Windows package manager.
- **Alpine APK** — for Alpine-based Docker images and lightweight server installs.
- **Snapcraft / Flatpak** — for Linux users who prefer sandboxed packages.

If there's a package manager you want yaak on, [open an issue](https://github.com/hanneshapke/yaak/issues). We'll get to it.

## Get started

No matter which install method you pick, the next step is the same:

```bash
yaak --config
```

Pick your language, choose a provider, enter your API key, and you're done. The whole setup takes about thirty seconds — which, come to think of it, is also how long it takes to forget the `tar` flags for the last time.

-----

*yaak is open source under the Apache-2.0 license. Install it from your favorite package manager, set an API key, and stop googling bash syntax.*

*-> [getyaak.ai](https://getyaak.ai) . [GitHub](https://github.com/hanneshapke/yaak)*
