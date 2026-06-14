{ pkgs, port }:
pkgs.runCommand "jellyhaj-test-server-files" {} ''
  mkdir $out
  substitute ${./network.xml} $out/network.xml --replace-fail "@port@" ${toString port}
  substitute ${./setup-jellyfin.sh} $out/setup-jellyfin --replace-fail "@port@" ${toString port}
  chmod +x $out/setup-jellyfin
''
