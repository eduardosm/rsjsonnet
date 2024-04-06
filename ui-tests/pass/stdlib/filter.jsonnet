std.assertEqual(std.filter(function(x) null, []), []) &&
std.assertEqual(std.filter(function(x) true, [1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.filter(function(x) false, [1, 2, 3]), []) &&
std.assertEqual(
  std.filter(function(x) x[0] == 1, [[1, 1], [2, 2], [1, 2]]),
  [[1, 1], [1, 2]],
) &&
std.assertEqual(
  std.filter(function(x, y=1) x[0] == y, [[1, 1], [2, 2], [1, 2]]),
  [[1, 1], [1, 2]],
) &&

true
