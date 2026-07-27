let options = open ./.attrs.json
let debug = $options.debug
if $debug {
  $options | table --theme basic --width 100 --expand  | print
}
let out = $options.outputs.out
if $debug {
  print "config:"
  $options.data | table --theme basic --width 100 --expand  | print
}
let log = if $debug {"trace"} else {"warn"}
$options.data | save $out;
let jellyhaj = $options.jellyhaj
let cmd = $options.check_cmd
print $"running ($jellyhaj) ($cmd) ($out)"
with-env {RUST_LOG: $log, RUST_BACKTRACE:"1"} {run-external $jellyhaj $cmd $out}

