std.parseYaml(|||
  a: &alias 1
  ---
  b: 2
  *alias : 3
|||)
