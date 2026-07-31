# requires nushell-plugin-dbus
#

def "jellyhaj find" () {
  dbus list | where $it =~ "jellyhaj" | first
}

def "jellyhaj tracklist" () {
  let dest = jellyhaj find
  dbus get-all --dest=$dest /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.TrackList
}

def "jellyhaj tracklist add" (id: string, after: string, current: bool) {
  let dest = jellyhaj find
  dbus call --dest=$dest /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.TrackList AddTrack $"jellyfin-item:($id)" $after $current
}

def "jellyhaj player" () {
  let dest = jellyhaj find
  dbus get-all --dest=$dest /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player
}

def "jellyhaj media-player" () {
  let dest = jellyhaj find
  dbus get-all --dest=$dest /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2
}


def jellyhaj () {
  if (yn "run jellyhaj") {
    run-external jellyhaj
  }
}

def yn (prompt: string) {
  loop {
    let r = input -n 1 $"($prompt) y/n: "
    if $r == y {
      return true
    }
    if $r == n {
      return false
    }
  }
}
