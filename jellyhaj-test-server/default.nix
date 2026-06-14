{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jellyhaj-test-server;
  inherit (lib)
    mkEnableOption
    mkOption
    types
    mkMerge
    mkIf
    mkDefault
    ;
  files = import ./files.nix {
    inherit pkgs;
    inherit (cfg) port;
  };
in
{
  options.jellyhaj-test-server = {
    enable = mkEnableOption "enable jellyhaj test server";
    port = mkOption {
      type = types.int;
      default = 8096;
      description = "jellyfin server port";
      example = 8000;
    };
  };
  config = mkMerge [
    { jellyhaj-test-server.enable = mkDefault true; }
    (mkIf cfg.enable {
      containers.jellyhaj-test-server = {
        ephemeral = true;
        privateUsers = "pick";
        restartIfChanged = true;
        config = { ... }: {
          environment.enableAllTerminfo = true;
          system.stateVersion = "26.11";
          services.jellyfin = {
            enable = true;
            forceEncodingConfig = true;
          };
          systemd = {
            tmpfiles.rules = [ "L+ /var/lib/jellyfin/config/network.xml - - - - ${files}/network.xml" ];
            services.setup-jellyfin = {
              after = [ "jellyfin.service" ];
              wantedBy = ["multi-user.target"];
              path = [pkgs.bash pkgs.curl];
              serviceConfig = {
                Type="simple";
                ExecStart = "${files}/setup-jellyfin";
              };
            };
          };
        };
      };

    })
  ];
}
