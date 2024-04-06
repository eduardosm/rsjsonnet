std.assertEqual(std.uniq([]), []) &&
std.assertEqual(std.uniq([1]), [1]) &&
std.assertEqual(std.uniq([1, 2, 3, 4]), [1, 2, 3, 4]) &&
std.assertEqual(std.uniq([4, 3, 2, 1]), [4, 3, 2, 1]) &&
std.assertEqual(std.uniq([1, 1, 2, 2]), [1, 2]) &&
std.assertEqual(std.uniq([1, 2, 2, 1]), [1, 2, 1]) &&
std.assertEqual(std.uniq([1, 1, 2, 1, 1]), [1, 2, 1]) &&

std.assertEqual(std.uniq([], keyF=function(x) error "err"), []) &&

std.assertEqual(std.uniq([[1, 10]], function(v) v[0]), [[1, 10]]) &&
std.assertEqual(std.uniq([[1, 10], [2, 11]], function(v) v[0]), [[1, 10], [2, 11]]) &&
std.assertEqual(std.uniq(
  [[1, 10], [2, 11], [2, 12], [3, 13]], function(v) v[0]),
  [[1, 10], [2, 11], [3, 13]],
) &&
std.assertEqual(std.uniq(
  [[1, 10], [1, 11], [2, 12], [3, 13]], function(v) v[0]),
  [[1, 10], [2, 12], [3, 13]],
) &&
std.assertEqual(std.uniq(
  [[1, 10], [2, 11], [3, 12], [3, 13]], function(v) v[0]),
  [[1, 10], [2, 11], [3, 12]],
) &&

std.assertEqual(
  std.uniq(["one", "two", "three", "four", "six", "seven"], keyF=std.length),
  ["one", "three", "four", "six", "seven"],
) &&

true
