{ config, lib, ... }:
let
  cfg = config.jellyhaj-test-server;
  inherit (lib) mkEnableOption mkOption types mkMerge mkIf mkDefault;
in
{
  options.jellyhaj-test-server = {
    enable = mkEnableOption "enable jellyhaj test server";
    port = mkOption {
      type = types.int;
      default = 3000;
      description = "jellyfin server port";
      example = 3500;
    };
  };
  config = mkMerge [
    {jellyhaj-test-server = mkDefault true;}
    (mkIf cfg.enable {
      containers.jellyhaj-test-server = {
        config = let port = cfg.port; in {...} : {
          services.jellyfin = {
            enable = true;
            forceEncodingConfig = true;
          };
        };
      };
      
    })
  ];
}
