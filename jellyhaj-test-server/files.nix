{
  runCommand,
  fetchurl,
  fetchzip,
  lib,
  jellyfin,
  nushell,
}:
let
  tv = fetchurl {
    url = "https://archive.org/download/flash-gordon-tv-show/Flash%20Gordon%20S01%20E01The%20Planet%20of%20Death.mp4";
    hash = "sha256-S2K0wCYT+zuKBswyNBuusbxPA/tWa1bw85Q3Q9b1xPQ=";
  };
  movie = fetchurl {
    url = "https://archive.org/download/CC_1914_02_02_MakingALiving/CC_1914_02_02_MakingALiving_512kb.mp4";
    hash = "sha256-PiI0g4sMaErJ4dc7b6Y5fH3c9bwVqHjb2nP8+JBuArM=";
  };
  music = fetchzip {
    url = "https://archive.org/compress/lp_the-nine-symphonies-of-beethoven_ludwig-van-beethoven-rene-leibowitz-the/formats=OPUS&file=/lp_the-nine-symphonies-of-beethoven_ludwig-van-beethoven-rene-leibowitz-the.zip";
    hash = "sha256-goIsfAboU+GaoaXkBOQoymRVsucf/V2PsdE3+VxCQQQ=";
    stripRoot = false;
  };
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
  mkdir $out/media
  mkdir $out/media/series
  mkdir "$out/media/series/Flash Gordon"
  mkdir $out/media/movies
  mkdir "$out/media/movies/Making A Living"
  mkdir $out/media/music
  mkdir "$out/media/music/The Nine Symphonies Of Beethoven"

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
  cd "$out/media/series/Flash Gordon"
  ln -s ${tv}
  cd "$out/media/movies/Making A Living"
  ln -s ${movie}
  cd "$out/media/music/The Nine Symphonies Of Beethoven"
  ln -s ${music}
''
