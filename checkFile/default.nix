{
  lib,
  nushell,
  jellyhaj,
  stdenv,
  debug ? false,
}:
let
  script = builtins.path { path = ./check.nu; };
  inherit (lib) getExe;
  writeFile =
    out: checkCmd: data:
    derivation {
      name = out;
      system = stdenv.buildPlatform.system;
      builder = getExe nushell;
      args = [
        "-n"
        script
      ];
      jellyhaj = getExe jellyhaj;
      inherit data debug;
      check_cmd = checkCmd;
      __structuredAttrs = true;
    };
in
{
  writeConfig = writeFile "config.toml" "check-config";
  writeKeybinds = writeFile "keybinds.toml" "check-keybinds";
  writeEffects = writeFile "effects.toml" "check-effects";
}
