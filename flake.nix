{
  description = "jellyfin terminal ui in nice";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-rust-build = {
      url = "github:RobinMarchart/nix-rust-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      nix-rust-build,
      rust-overlay,
      ...
    }:
    let
      lib = nixpkgs.lib;
      eachSystem =
        f:
        let
          forSystem = system: builtins.mapAttrs (name: val: { ${system} = val; }) (f system);
          sets = map forSystem lib.systems.flakeExposed;
        in
        builtins.foldl' lib.attrsets.recursiveUpdate { } sets;
    in
    (
      eachSystem (
        system:
        let
          overlays = [
            rust-overlay.overlays.default
          ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          rust-build = nix-rust-build.rust-build-from-pkgs pkgs;
          jellyhaj = pkgs.callPackage ./jellyhaj.nix { inherit rust-build; };
          writer = pkgs.callPackage ./checkFile { inherit jellyhaj; };
          inherit (writer) writeConfig writeKeybinds writeEffects;
          test-server = pkgs.callPackage ./jellyhaj-test-server { };

        in
        {
          formatter = pkgs.nixfmt-tree;
          packages = {
            default = jellyhaj;
            inherit jellyhaj;
          }
          // (if pkgs.stdenv.hostPlatform.isLinux then { inherit test-server; } else { });
          checks = {
            inherit jellyhaj;
            config = writeConfig (fromTOML (builtins.readFile ./config/config.toml));
            keybinds = writeKeybinds (fromTOML (builtins.readFile ./config/keybinds.toml));
            effects = writeEffects (fromTOML (builtins.readFile ./config/effects.toml));
          };
          apps = {
            default = {
              type = "app";
              program = "${jellyhaj}/bin/jellyhaj";
              meta = jellyhaj.meta;
            };
          };
          devShells = {
            default =
              let
                llvm = pkgs.llvmPackages_22;
              in
              (pkgs.mkShell.override { stdenv = llvm.stdenv; }) {
                nativeBuildInputs = [
                  llvm.bintools
                  pkgs.cargo-nextest
                  pkgs.cargo-audit
                  pkgs.cargo-expand
                  pkgs.cargo-llvm-lines
                  pkgs.rust-bin.nightly.latest.rust-analyzer
                  (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
                  pkgs.rustPlatform.bindgenHook
                  pkgs.sqlx-cli
                  pkgs.pkg-config
                  pkgs.sqlite-interactive
                  pkgs.tokio-console
                  pkgs.rust-bindgen
                ];
                buildInputs = [
                  (pkgs.mpv-unwrapped.overrideAttrs { separateDebugInfo = true; })
                  pkgs.sqlite
                  pkgs.chafa
                  pkgs.glib
                  pkgs.aws-lc.dev
                ];
                DATABASE_URL = "sqlite://db.sqlite";
              };
          };
        }
      )
      // (
        let
          jellyhaj = final: prev: {
            jellyhaj = final.callPackage ./jellyhaj.nix { };
          };
        in
        {
          overlays = {
            inherit jellyhaj;
            default = jellyhaj;
            rust-build = nix-rust-build.overlays.default;
          };
          hmModules = {
            default = import ./hm-module.nix nix-rust-build.rust-build-from-pkgs;
          };
        }
      )
    );
}
