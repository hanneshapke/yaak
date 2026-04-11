# Non-flake entry point. Lets users do:
#
#   nix-build packaging/nix
#   nix-shell -p '(import ./packaging/nix {}).yaak'
#
# without enabling flakes. Flake users should prefer `flake.nix`.

{ pkgs ? import <nixpkgs> { } }:

let
  yaakSrc = ../..;
  manifest = (pkgs.lib.importTOML "${yaakSrc}/Cargo.toml").package;

  yaak = pkgs.rustPlatform.buildRustPackage {
    pname = manifest.name;
    version = manifest.version;

    src = yaakSrc;

    cargoLock = {
      lockFile = "${yaakSrc}/Cargo.lock";
    };

    nativeBuildInputs = [ pkgs.pkg-config pkgs.installShellFiles ];

    buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
      pkgs.darwin.apple_sdk.frameworks.Security
      pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
    ];

    postInstall = ''
      installShellCompletion --cmd yaak \
        --bash <($out/bin/yaak --completions bash) \
        --zsh  <($out/bin/yaak --completions zsh)  \
        --fish <($out/bin/yaak --completions fish)
    '';

    meta = with pkgs.lib; {
      description = manifest.description;
      homepage = "https://github.com/hanneshapke/yaak";
      license = licenses.asl20;
      mainProgram = "yaak";
      platforms = platforms.unix;
    };
  };
in
{
  inherit yaak;
}
