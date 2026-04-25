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
in
{
  options.programs.jellyhaj = {
    enable = mkEnableOption "enable jellyhaj tui";
    package = mkOption {
      type = types.package;
      default = jellyhaj;
      description = "package with jellyhaj";
    };
    config = {
      mpv_profile = mkOption {
        type = types.enum [
          "fast"
          "high-quality"
          "default"
        ];
        default = "default";
        description = "mpv profile to inherit from";
      };
      hwdec = mkOption {
        type = types.str;
        default = "auto-safe";
        description = "hardware decoding";
      };
      mpv_log_level = mkOption {
        type = types.enum [
          "no"
          "fatal"
          "error"
          "warn"
          "info"
          "v"
          "debug"
          "trace"
        ];
        default = "info";
        description = "mpv log level, separate from general log level";
      };
      login_file = mkOption {
        type = types.path;
        default = "${config.xdg.configHome}/jellyhaj/login.toml";
        description = "login file";
      };
      keybinds_file = mkOption {
        type = types.nullOr types.path;
        default = null;
      };
      effects_file = mkOption {
        type = types.nullOr types.path;
        default = null;
      };
      mpv_config_file = mkOption {
        type = types.nullOr types.path;
        default = null;
      };
      entry_image_with = mkOption {
        type = lib.types.ints.u16;
        default = 32;
      };
      concurrent_jellyfin_connections = mkOption {
        type = lib.types.ints.u8;
        default = 2;
      };
      fetch_timeout = mkOption {
        type = lib.types.ints.u16;
        default = 15;
      };
      store_access_token = mkOption {
        type = lib.types.bool;
        default = false;
        description = "store the access token in the cache";
      };
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
    default = "${config.xdg.configHome}/jellyhaj/keybinds.toml";
  };
  config = mkMerge [
    (mkIf cfg.enable {
      home.packages = [ cfg.package ];
      xdg.configFile = {
        "jellyhaj/config.toml".source = pkgs.writers.writeTOML "config.toml" (
          filterAttrs (_: v: !isNull v) cfg.config
        );
      };
    })
    (mkIf (!isNull cfg.keybinds) {
      programs.jellyhaj.config.keybinds_file = jellyhaj.checkKeybinds cfg.keybinds;
    })
    (mkIf (!isNull cfg.effects) {
      programs.jellyhaj.config.effects_file = jellyhaj.checkEffects cfg.effects;
    })
  ];
}
