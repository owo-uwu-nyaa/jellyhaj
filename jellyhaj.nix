{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  mpv-unwrapped,
  sqlite,
  versionCheckHook,
  runCommand,
  remarshal,
  withMpris ? stdenv.isLinux, # enable media player dbus interface
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
    ./libmpv-rs/test-data
  ];
  src = lib.fileset.toSource {
    root = ./.;
    inherit fileset;
  };
  jellyhaj =
    let
      checkKeybinds =
        keybinds:
        runCommand "keybinds.toml"
          {
            nativeBuildInputs = [
              remarshal
              jellyhaj
            ];
            value = builtins.toJSON keybinds;
            passAsFile = [ "value" ];
          }
          ''
            json2toml "$valuePath" "$out"
            jellyhaj check-keybinds "$out"
          '';
      checkEffects =
        effects:
        runCommand "effects.toml"
          {
            nativeBuildInputs = [
              remarshal
              jellyhaj
            ];
            value = builtins.toJSON effects;
            passAsFile = [ "value" ];
          }
          ''
            json2toml "$valuePath" "$out"
            jellyhaj check-effects "$out"
          '';
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
        pkg-config
      ];
      buildInputs = [
        sqlite
        mpv-unwrapped
      ];
      passthru = {
        inherit checkKeybinds checkEffects;
      };
      postBuild = lib.optionalString stdenv.hostPlatform.isLinux ''
        install -Dm644 $src/jellyhaj.desktop $out/share/applications/jellyhaj.desktop       
      '';
      nativeInstallCheckInputs = [ versionCheckHook ];
      versionCheckProgramArg = "--version";
      doInstallCheck = true;
      checkFlags = [
        #some tests need internet access
        "--skip=tests::properties"
        "--skip=tests::node_map"
        "--skip=tests::events"
      ];
      cargoTestFlags = [ "--workspace" ];
      cargoBuildFlags = lib.optional withTools "--workspace";
      buildFeatures = lib.optional withMpris "mpris";
      separateDebugInfo = true;
      meta = {
        description = "Terminal client for Jellyfin reimplementing parts of the web ui";
        license = lib.licenses.mit;
        sourceProvenance = [ lib.sourceTypes.fromSource ];
        mainProgram = "jellyhaj";
        homepage = "https://github.com/owo-uwu-nyaa/jellyhaj";
      };
    };
in
jellyhaj
