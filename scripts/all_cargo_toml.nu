def all_cargo_toml_inner () {
  ls $in | let files
  $files | where $it.type == "dir" | each {|e|  $e.name | all_cargo_toml_inner} | flatten | let recursive
  $files | where ($it.name | path basename) == "Cargo.toml" | append $recursive
}

def all_cargo_toml () {
  ls | where $it.type == "dir" | where ($it.name | path basename) != "target" | each {|e| $e.name | all_cargo_toml_inner} | flatten | each {|e|  $e.name}
}

def transform_all_members (modifier: closure) {
  all_cargo_toml | each {|f| open $f | do $modifier | save -f $f}
}

def test_transform_all_members (example: path, modifier: closure) {
  open $example | do $modifier
}
