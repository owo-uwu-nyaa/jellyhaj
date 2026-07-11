{
  description = "jellyfin terminal ui in nice";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      systems,
      ...
    }:
    let
      lib = nixpkgs.lib;
      eachSystem =
        f:
        let
          forSystem = system: builtins.mapAttrs (name: val: { ${system} = val; }) (f system);
          sets = map forSystem (import systems);
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
          jellyhaj = pkgs.callPackage ./jellyhaj.nix { };
          test-server =
            if lib.systems.inspect.predicates.isLinux system then
              {
                test-server = pkgs.callPackage ./jellyhaj-test-server { };
              }
            else
              { };
        in
        {
          formatter = pkgs.nixfmt-tree;
          packages = {
            default = jellyhaj;
            inherit jellyhaj;
          }
          // test-server;
          checks = { inherit jellyhaj; };
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
                ];
                buildInputs = [
                  pkgs.mpv-unwrapped
                  pkgs.sqlite
                  pkgs.chafa
                  pkgs.glib
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
          inherit self;
          overlays = {
            inherit jellyhaj;
            default = jellyhaj;
          };
          hmModules = {
            default = import ./hm-module.nix;
          };
        }
      )
    );
}
