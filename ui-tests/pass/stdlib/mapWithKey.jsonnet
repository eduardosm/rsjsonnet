std.assertEqual(std.mapWithKey(function(k, v) error "err", {}), {}) &&
std.assertEqual(
  std.mapWithKey(function(k, v) k + ":" + v, { a: 1, b: 2 }),
  { a: "a:1", b: "b:2" },
) &&
std.assertEqual(
  std.mapWithKey(std.format, { "%i": 10, "%x": 11 }),
  { "%i": "10", "%x": "b" },
) &&
std.assertEqual(
  std.objectFieldsAll(std.mapWithKey(function(k, v) k + ":" + v, { a: 1, b: 2, c:: 3 })),
  ["a", "b"],
) &&

true
