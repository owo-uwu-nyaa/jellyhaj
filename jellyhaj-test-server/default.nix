{
  jellyfin,
  nushell,
  dockerTools,
  runCommand,
  lib,
  port ? 8000,
}:
let
  port_s = toString port;
  files = import ./files.nix {
    inherit
      runCommand
      lib
      jellyfin
      nushell
      ;
  };
in
dockerTools.streamLayeredImage {
  name = "jellyhaj-test-server";
  config = {
    User = "jellyfin:jellyfin";
    ExposedPorts = {
      "${port_s}/tcp" = { };
    };
    Env = [
      "PORT=${port_s}"
    ];
    Cmd = [
      "/bin/nu"
      "/bin/run.nu"
    ];
  };
  contents = [
    files
    dockerTools.usrBinEnv
    dockerTools.binSh
    dockerTools.caCertificates
  ];
  fakeRootCommands = ''
    mkdir -p /var/lib/jellyfin
    chown 100:100 /var/lib/jellyfin
    mkdir -p /.home
    chown 100:100 /.home
    mkdir -p /var/cache/jellyfin
    chown 100:100 /var/cache/jellyfin
    mkdir -p /tmp
    chown 100:100 /tmp
  '';
  enableFakechroot = true;
}
