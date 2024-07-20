std.parseYaml(|||
  x: &anchor
    a: 1
    b: 2
  y: *anchor
|||)
