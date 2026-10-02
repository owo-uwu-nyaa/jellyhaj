{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  mpv-unwrapped,
  sqlite,
  chafa,
  glib,
  aws-lc,
  rust-build,
  versionCheckHook,
  installShellFiles,
  withMpris ? stdenv.hostPlatform.isLinux, # enable media player dbus interface
  withJournald ? stdenv.hostPlatform.isLinux,
}:
let
  fileset = lib.fileset.unions [
    (lib.fileset.fileFilter (
      file:
      file.hasExt "rs"
      || file.name == "Cargo.toml"
      || file.name == "Cargo.lock"
      || file.name == "Readme.md"
    ) ./.)
    ./config/config.toml
    ./config/keybinds.toml
    ./config/effects.toml
    ./jellyhaj.desktop
    ./widgets/form-derive-impl/test-files
    ./.sqlx
    ./migrations
  ];
  src = lib.fileset.toSource {
    root = ./.;
    inherit fileset;
  };
  jellyhaj =
    (rust-build.withCrateOverrides {
      mpv-sys = {
        buildInputs = [ mpv-unwrapped ];
        nativeBuildInputs = [
          pkg-config
          rustPlatform.bindgenHook
        ];
      };
      libsqlite3-sys = {
        buildInputs = [ sqlite ];
        nativeBuildInputs = [
          pkg-config
          rustPlatform.bindgenHook
        ];
      };
      aws-lc-sys = {
        buildInputs = [
          aws-lc.dev
        ];
        nativeBuildInputs = [ pkg-config ];
      };
      ratatui-image = {
        buildInputs = [
          chafa
          glib
        ];
        nativeBuildInputs = [ pkg-config ];
      };
      jellyhaj-bin = {
        nativeBuildInputs = [ installShellFiles ];
        postInstall = ''
          echo installing desktop file
          install -Dm644 $src/jellyhaj.desktop $out/share/applications/jellyhaj.desktop       
          echo Generating jellyhaj completions
          mkdir completion
          ${jellyhaj.workspaceMembers.xtask}/bin/xtask print-completions completion bash zsh fish nushell
          installShellCompletion completion/*
          echo Finished generating jellyhaj completions
        '';
        nativeInstallCheckInputs = [ versionCheckHook ];
        versionCheckProgramArg = "--version";
        doInstallCheck = true;
        meta = {
          description = "Terminal client for Jellyfin reimplementing parts of the web ui";
          license = lib.licenses.mit;
          sourceProvenance = [ lib.sourceTypes.fromSource ];
          mainProgram = "jellyhaj";
          homepage = "https://github.com/owo-uwu-nyaa/jellyhaj";
        };
      };
    }).build
      {
        inherit src;
        pname = "jellyfin-tui";
        version = "0.2.1";
        features = (lib.optional withMpris "mpris") ++ (lib.optional withJournald "journald");
      };
in
jellyhaj
