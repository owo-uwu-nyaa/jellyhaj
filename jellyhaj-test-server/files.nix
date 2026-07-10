{
  runCommand,
  lib,
  jellyfin,
  nushell,
}:
let
  set = lib.fileset.unions [
    ./group
    ./passwd
    ./run.nu
    ./network.xml
    ./nsswitch.conf
  ];
  src = lib.fileset.toSource {
    root = ./.;
    fileset = set;
  };
in
runCommand "jellyhaj-test-server-files" { } ''
  mkdir $out
  mkdir $out/etc
  mkdir $out/var
  mkdir $out/var/empty
  mkdir $out/bin
  mkdir $out/template

  cd $out/etc
  ln -s ${src}/group
  ln -s ${src}/passwd
  ln -s ${src}/nsswitch.conf
  cd $out/template
  ln -s ${src}/network.xml
  cd $out/bin
  ln -s ${src}/run.nu
  ln -s ${lib.getExe jellyfin}
  ln -s ${lib.getExe nushell}
''
