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
  writeShellScript,
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
  mkFiltered =
    path:
    builtins.path {
      path = src;
      filter =
        let
          parents-gen =
            path:
            if path == "." then
              [ ]
            else
              let
                path' = dirOf path;
              in
              (parents-gen path') ++ [ "${src}/${path}" ];
          parents = parents-gen path;
          base = "${src}/${path}";
        in
        path: type: (builtins.any (p: p == path) parents) || (lib.hasPrefix base path);
    };
  sqlx-fake-cargo = writeShellScript "sqlx-fake-cargo" ''echo '{"workspace_root": "${mkFiltered ".sqlx"}"}' '';
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
      xtask = {
        postPatch = "ln -s ${mkFiltered "src/args.rs"}/src ..";
      };
      config = {
        postPatch = "ln -s ${mkFiltered "migrations"}/migrations ..";
        CARGO = sqlx-fake-cargo;
      };
      jellyhaj-event-listener = {
        CARGO = sqlx-fake-cargo;
      };
      jellyhaj-image = {
        CARGO = sqlx-fake-cargo;
      };
      jellyhaj-login-view = {
        CARGO = sqlx-fake-cargo;
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
