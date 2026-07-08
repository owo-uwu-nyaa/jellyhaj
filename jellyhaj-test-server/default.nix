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
            services = {
              setup-jellyfin-pre = {
                before = [ "jellyfin.service" ];
                wantedBy = [ "jellyfin.service" ];
                serviceConfig = {
                  Type = "oneshot";
                };
                script = ''
                  cp ${files}/network.xml /var/lib/jellyfin/config/network.xml
                  chmod 0660 /var/lib/jellyfin/config/network.xml 
                  chown jellyfin:jellyfin /var/lib/jellyfin/config/network.xml 
                '';
                path = [ pkgs.coreutils ];
                enableStrictShellChecks = true;
                unitConfig = {
                  ConditionPathExists = "!var/lib/jellyfin/.setup-complete";
                };
              };
              setup-jellyfin-post = {
                after = [ "jellyfin.service" ];
                wantedBy = [ "jellyfin.service" ];
                path = [
                  pkgs.bash
                  pkgs.curl
                ];
                serviceConfig = {
                  Type = "oneshot";
                  ExecStart = "${files}/setup-jellyfin";
                };
                unitConfig = {
                  ConditionPathExists = "!var/lib/jellyfin/.setup-complete";
                };
              };
            };
          };
        };
      };

    })
  ];
}
