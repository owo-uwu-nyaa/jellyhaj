
def main [] {
  let file = $env.XDG_CACHE_HOME | path join jellyhaj.sqlite
  open $file | get jellyfin_socket_events | each {|v| $v | update val  ($v.val | from json) }
}
