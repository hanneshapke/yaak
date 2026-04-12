{
  description = "yaak — translate natural language to bash commands via an OpenAI-compatible LLM";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Point the build at the repository root. When consumed as
        # `github:hanneshapke/yaak`, users can override `src` to build a
        # specific revision.
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

          # Needed by rustls + reqwest on some systems; harmless otherwise.
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
            maintainers = [ ];
            platforms = platforms.unix;
          };
        };
      in
      {
        packages = {
          inherit yaak;
          default = yaak;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = yaak;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ yaak ];
          packages = with pkgs; [ cargo rustc rustfmt clippy rust-analyzer ];
        };
      });
}
