{ pkgs, port }:
pkgs.runCommand "jellyhaj-test-server-files" {} ''
  mkdir $out
  substitute ${./network.xml} $out/network.xml --replace-fail "@port@" ${port}
  substitute ${./setup-jellyfin.sh} $out/setup-jellyfin --replace-fail "@port@" ${port}
  chmod +x $out/setup-jellyfin
''
