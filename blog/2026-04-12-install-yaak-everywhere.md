---
title: "Install yaak Everywhere: AUR, Nix, Scoop, and Beyond"
date: 2026-04-12
slug: install-yaak-everywhere
description: "yaak is now available on AUR for Arch Linux, Nix for declarative systems, and Scoop for Windows — plus Homebrew, Cargo, and curl. Here's how to install it on every platform."
author: yaak team
---

*The number-one request since launch: "Can I install this on [my platform]?" Today the answer is yes for almost everything.*

yaak started with two install methods: `cargo install yaak` and a Homebrew tap. That covers macOS developers nicely, but it left out Arch users, the Nix crowd, and anyone on Windows. As of today, yaak ships packages for **AUR**, **Nix**, and **Scoop** — all updated automatically on every release.

## The full menu

Here's every way to install yaak, from quickest to most opinionated:

### curl (macOS / Linux)

The one-liner that started it all. Downloads the latest release binary, verifies the SHA256 checksum, and drops it in `~/.local/bin`:

```bash
curl -fsSL https://getyaak.ai/install.sh | bash
```

Pin a version or change the install directory:

```bash
YAAK_VERSION=v0.1.3 curl -fsSL https://getyaak.ai/install.sh | bash
YAAK_INSTALL_DIR=/usr/local/bin curl -fsSL https://getyaak.ai/install.sh | bash
```

### Homebrew (macOS / Linux)

```bash
brew install hanneshapke/yaak/yaak
```

Tracks releases via a custom tap. `brew upgrade yaak` picks up new versions.

### AUR (Arch Linux) -- new

Two packages, depending on how you like your software:

```bash
# Build from source
yay -S yaak

# Prebuilt binary (faster)
yay -S yaak-bin
```

The source package (`yaak`) clones the release tarball and runs `cargo build --release` on your machine. The binary package (`yaak-bin`) downloads the prebuilt x86_64 Linux binary from GitHub Releases. Both install shell completions for bash, zsh, and fish automatically.

Works with any AUR helper — `yay`, `paru`, `pikaur`, or plain `makepkg -si`.

### Nix (Linux / macOS) -- new

For flake users:

```bash
# One-shot — try it without installing
nix run github:hanneshapke/yaak -- find all log files larger than 100MB

# Persistent install
nix profile install github:hanneshapke/yaak
```

The flake exposes `packages.<system>.yaak` for x86_64 and aarch64 on both Linux and macOS. Shell completions are installed via `installShellFiles`. A `default.nix` shim is included for non-flake `nix-build` users.

Because the flake reads `version` directly from `Cargo.toml` and `Cargo.lock` from the repo, there's no separate version bump needed — every tagged release is immediately available.

For NixOS users who manage their system declaratively, you can add the flake as an input to your system configuration and include `yaak` in `environment.systemPackages`.

### Scoop (Windows) -- new

```powershell
# Add the bucket (one-time)
scoop bucket add yaak https://github.com/hanneshapke/scoop-yaak

# Install
scoop install yaak
```

The Scoop manifest points at the prebuilt Windows x86_64 binary from each release. `scoop update yaak` picks up new versions automatically.

### Cargo (any platform)

```bash
cargo install yaak
```

Builds from source using whatever Rust toolchain you have installed. Works anywhere Rust works, including platforms without prebuilt binaries.

## Automated releases

Every install method updates automatically when we tag a new version. The CI pipeline:

1. Builds release binaries for Linux (x86_64), macOS (x86_64 + aarch64), and Windows (x86_64).
2. Publishes them to GitHub Releases with SHA256 checksums.
3. Pushes to crates.io (`cargo publish`).
4. Updates the Homebrew formula in `hanneshapke/homebrew-yaak`.
5. Patches the AUR PKGBUILDs with fresh hashes and pushes to both `yaak` and `yaak-bin` on the AUR.
6. Generates a Scoop manifest with the correct hash and pushes to `hanneshapke/scoop-yaak`.
7. Verifies the Nix flake still builds.

Tag, push, walk away. All six distribution channels update within minutes.

## What's next

A few install methods we're considering:

- **Snapcraft / Flatpak** — for Linux users who prefer sandboxed packages.
- **nixpkgs upstream** — so yaak is available via `nix-env -iA nixpkgs.yaak` without adding a flake input.
- **winget** — the other Windows package manager.
- **Alpine APK / Debian .deb** — for Docker base images and server installs.

If there's a package manager you want yaak on, [open an issue](https://github.com/hanneshapke/yaak/issues) and let us know.

## Get started

No matter which install method you choose, the next step is the same:

```bash
yaak --config
```

Pick your language, choose a provider, enter your API key, and start yaaking.

-----

*yaak is open source under the Apache-2.0 license. Install it with your favorite package manager, set your API key, and stop googling bash syntax.*

*-> [getyaak.ai](https://getyaak.ai) . [GitHub](https://github.com/hanneshapke/yaak)*
