
export def open-events [p: path] {
  open  -r $p | from json | group-by --to-table MessageType | each {|e|{type: $e.MessageType, val: $e.items.Data}} | transpose -ird
}
