# Packaging

This directory contains package definitions for distributing yaak through
several package managers in addition to Homebrew.

| Manager  | Platform    | Files                                       |
|----------|-------------|---------------------------------------------|
| AUR      | Arch Linux  | [`aur/yaak/PKGBUILD`](aur/yaak/PKGBUILD), [`aur/yaak-bin/PKGBUILD`](aur/yaak-bin/PKGBUILD) |
| Nix      | Linux/macOS | [`nix/flake.nix`](nix/flake.nix), [`nix/default.nix`](nix/default.nix) |
| Scoop    | Windows     | [`scoop/yaak.json`](scoop/yaak.json)        |
| Homebrew | macOS/Linux | published at [`hanneshapke/homebrew-yaak`](https://github.com/hanneshapke/homebrew-yaak) |

## AUR (Arch Linux)

Two packages are provided:

- **`yaak`** — builds from source using `cargo`. Install build-time dependencies
  and run `makepkg -si` inside `packaging/aur/yaak`.
- **`yaak-bin`** — downloads the upstream prebuilt Linux `x86_64` tarball from
  GitHub Releases and verifies it against `SHA256SUMS.txt`.

To publish to the AUR, push the `PKGBUILD` (together with a generated
`.SRCINFO`) to the respective AUR git repositories (`aur.archlinux.org/yaak.git`
and `aur.archlinux.org/yaak-bin.git`).

```sh
cd packaging/aur/yaak
makepkg --printsrcinfo > .SRCINFO
makepkg -si
```

## Nix

```sh
# From a flake-enabled system
nix run github:hanneshapke/yaak -- --help
nix profile install github:hanneshapke/yaak

# Or use the flake locally
cd packaging/nix
nix build
./result/bin/yaak --help
```

The flake exposes `packages.<system>.yaak` (also the default package) for
`x86_64-linux`, `aarch64-linux`, `x86_64-darwin`, and `aarch64-darwin`. A
`default.nix` shim is provided for non-flake users.

## Scoop (Windows)

```powershell
# Add the bucket (one-time)
scoop bucket add yaak https://github.com/hanneshapke/scoop-yaak

# Install
scoop install yaak
```

The manifest in [`scoop/yaak.json`](scoop/yaak.json) points at the Windows
prebuilt binary attached to each GitHub release and uses Scoop's built-in
`autoupdate` mechanism so future releases are picked up automatically.

## Keeping packages in sync with releases

Each package pins a version and either a `sha256`/`sha512` hash or a
`sha256sums.txt` URL. When you cut a new release:

1. `cargo set-version <new>` (or edit `Cargo.toml`).
2. Tag and push — CI publishes Linux/macOS/Windows archives and
   `SHA256SUMS.txt` to the GitHub release.
3. Update `pkgver` / `version` in each manifest under `packaging/` and refresh
   hashes:
   - AUR: `updpkgsums` inside each PKGBUILD directory, regenerate `.SRCINFO`.
   - Nix: update `version` in `flake.nix`; `cargoHash`/`hash` will be reported
     by `nix build` on first run (set to `lib.fakeHash` temporarily).
   - Scoop: `scoop update yaak` or let the `autoupdate` block in the manifest
     compute new hashes from the release's `SHA256SUMS.txt`.
