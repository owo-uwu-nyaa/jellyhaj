
let port = $env.PORT? | default "8000"

let headers = {authorization: 'MediaBrowser Client="nu",Device="nu",DeviceId="42",Version="1"'}

def write_config [] {
  mkdir /var/lib/jellyfin/config
  open --raw /template/network.xml | str replace -a "@port@" $port | save /var/lib/jellyfin/config/network.xml
}

def exec_jellyfin [] {
  exec /bin/jellyfin --configdir /var/lib/jellyfin/config --cachedir /var/cache/jellyfin --logdir /var/lib/jellyfin/log
}

def do_check_status [meta] {
  let status = $meta.http_response.status
  if $status != 200 and $status != 204 {error make {msg: $"failed with status ($status | into string)"}}
}

def check_status [] {
  metadata access {|m| do_check_status $m}
}

def send_lang_config [] {
  print "[setup] setting language"
  let lang_config = {
    MetadataCountryCode: "US",
    PreferredMetadataLanguage: "en",
    ServerName: "jellyhaj-test-server",
    UICulture: "en-US"
  }
  http post -t application/json -H $headers $"http://localhost:($port)/Startup/Configuration" $lang_config | check_status
}

def send_user_config [] {
  print "[setup] setting user"
  let user_config = {
    Name: "jellyfin",
    Password: "jellyfin"
  }
  http post -t application/json -H $headers $"http://localhost:($port)/Startup/User" $user_config | check_status
}

def send_network_config [] {
  print "[setup] setting network"
  let network_config = {
    EnableRemoteAccess: false
  }
  http post -t application/json -H $headers $"http://localhost:($port)/Startup/RemoteAccess" $network_config | check_status
}

def send_complete_setup [] {
  print "[setup] completing setup"
  http post -H $headers $"http://localhost:($port)/Startup/Complete" "" | check_status
}

def setup [] {
  if not ("/var/lib/jellyfin/.setup-complete" | path exists) {
    run-external /bin/sh "-c" "/bin/nu /bin/run.nu setup&"
  }
}

def main [] {
  write_config
  setup 
  exec_jellyfin
}

def "main setup" [] {
  sleep 15sec
  send_lang_config
  sleep 1sec
  send_user_config 
  sleep 1sec
  send_network_config
  sleep 1sec
  send_complete_setup
  "" | save /var/lib/jellyfin/.setup-complete
}














