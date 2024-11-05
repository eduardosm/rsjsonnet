std.assertEqual(
  std.flatMap(function(x) error "err", []),
  [],
) &&
std.assertEqual(
  std.flatMap(function(x) [], [1, 2, 3]),
  [],
) &&
std.assertEqual(
  std.flatMap(function(x) std.repeat([std.toString(x)], x), [2, 1, 0, 3]),
  ["2", "2", "1", "3", "3", "3"],
) &&

std.assertEqual(
  std.flatMap(function(x) error "err", ""),
  "",
) &&
std.assertEqual(
  std.flatMap(function(x) "", "abcd"),
  "",
) &&
std.assertEqual(
  std.flatMap(function(x) std.repeat(x, std.parseInt(x)), "3012"),
  "333122",
) &&
std.assertEqual(
  std.flatMap(function(x) ":" + x, "🧶🧺🧲🧢"),
  ":🧶:🧺:🧲:🧢",
) &&
std.assertEqual(
  std.flatMap(function(x) if x == "_" then null else x, "__1_2_3__4_"),
  "1234",
) &&

true
