std.parseYaml(|||
  a: &alias 1
  ---
  b: *alias
|||)
