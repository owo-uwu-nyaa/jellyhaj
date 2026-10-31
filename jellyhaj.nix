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
  versionCheckHook,
  installShellFiles,
  withMpris ? stdenv.hostPlatform.isLinux, # enable media player dbus interface
  withJournald ? stdenv.hostPlatform.isLinux,
  withTools ? false, # add developement tools
}:
let
  fileset = lib.fileset.unions [
    (lib.fileset.fileFilter (
      file: file.hasExt "rs" || file.name == "Cargo.toml" || file.name == "Cargo.lock"
    ) ./.)
    ./.sqlx
    ./config/config.toml
    ./config/keybinds.toml
    ./config/effects.toml
    ./migrations
    ./jellyhaj.desktop
    ./widgets/form-derive-impl/test-files
    ./mpv-sys/Readme.md
  ];
  src = lib.fileset.toSource {
    root = ./.;
    inherit fileset;
  };
in
rustPlatform.buildRustPackage {
  pname = "jellyhaj";
  version = (fromTOML (builtins.readFile ./Cargo.toml)).package.version;
  inherit src;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  nativeBuildInputs = [
    rustPlatform.bindgenHook
    installShellFiles
    pkg-config
  ];
  buildInputs = [
    sqlite
    mpv-unwrapped
    chafa
    glib
    aws-lc.dev
  ];
  postBuild = ''
    echo Generating jellyhaj completions
    mkdir completion
    cargo run --package xtask -- print-completions completion bash zsh fish nushell
    echo Finished generating jellyhaj completions
  '';
  postInstall = ''
    echo installing shell completion
    installShellCompletion completion/*
  ''
  + (lib.optionalString stdenv.hostPlatform.isLinux ''
    echo installing desptop file
    install -Dm644 $src/jellyhaj.desktop $out/share/applications/jellyhaj.desktop       
  '');
  nativeInstallCheckInputs = [ versionCheckHook ];
  versionCheckProgramArg = "--version";
  doInstallCheck = true;
  cargoTestFlags = [ "--workspace" ];
  cargoBuildFlags = lib.optional withTools "--workspace";
  buildFeatures = (lib.optional withMpris "mpris") ++ (lib.optional withJournald "journald");
  separateDebugInfo = true;
  __structuredAttrs = true;
  meta = {
    description = "Terminal client for Jellyfin reimplementing parts of the web ui";
    license = lib.licenses.mit;
    sourceProvenance = [ lib.sourceTypes.fromSource ];
    mainProgram = "jellyhaj";
    homepage = "https://github.com/owo-uwu-nyaa/jellyhaj";
  };
}
