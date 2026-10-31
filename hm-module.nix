{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib)
    mkEnableOption
    mkIf
    mkMerge
    mkOption
    types
    filterAttrs
    ;
  cfg = config.programs.jellyhaj;
  jellyhaj = pkgs.callPackage ./jellyhaj.nix { };
  writer = pkgs.callPackage ./checkFile {
    jellyhaj = cfg.package;
    debug = cfg.debugConfigChecker;
  };
  inherit (writer) writeConfig writeKeybinds writeEffects;
in
{
  options.programs.jellyhaj = {
    enable = mkEnableOption "enable jellyhaj tui";
    package = mkOption {
      type = types.package;
      default = jellyhaj;
      description = "package with jellyhaj";
    };
    debugConfigChecker = mkEnableOption "enable debug printing for the config checker derivations";
    config = mkOption {
      type = types.attrsOf types.anything;
      default = { };
      description = "jellyhaj configuration";
    };
    keybinds = mkOption {
      type = types.nullOr (types.attrsOf types.anything);
      default = null;
      description = "keybind configuration";
    };
    effects = mkOption {
      type = types.nullOr (types.attrsOf types.anything);
      default = null;
      description = "effects configuration";
    };
    login = mkOption {
      type = lib.types.nullOr (
        lib.types.submodule {
          options = {
            server_url = mkOption {
              type = lib.types.str;
            };
            username = mkOption {
              type = lib.types.str;
              default = "";
            };
            password = mkOption {
              type = lib.types.str;
              default = "";
            };
            password_cmd = mkOption {
              type = lib.types.nullOr (lib.types.listOf lib.types.str);
              default = null;
            };
          };
        }
      );
      default = null;
    };
  };
  config = mkMerge [
    (mkIf cfg.enable {
      home.packages = [ cfg.package ];
      xdg.configFile = {
        "jellyhaj/config.toml".source = writeConfig (filterAttrs (_: v: !isNull v) cfg.config);
      };
    })
    (mkIf (!isNull cfg.keybinds) {
      programs.jellyhaj.config.keybinds_file = writeKeybinds cfg.keybinds;
    })
    (mkIf (!isNull cfg.effects) {
      programs.jellyhaj.config.effects_file = writeEffects cfg.effects;
    })
  ];
}
