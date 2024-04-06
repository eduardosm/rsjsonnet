std.assertEqual(std.sort([]), []) &&

std.assertEqual(std.sort([], keyF=std.length), []) &&
std.assertEqual(
  std.sort(["one", "two", "three", "four", "six", "seven"], keyF=std.length),
  ["one", "two", "six", "four", "three", "seven"],
) &&

std.assertEqual(std.sort([2, 1, 2]), [1, 2, 2]) &&
std.assertEqual(std.sort([5, 0, 5, 7]), [0, 5, 5, 7]) &&
std.assertEqual(std.sort([0, 7, 2, 6, 1]), [0, 1, 2, 6, 7]) &&
std.assertEqual(std.sort([2, 8, 1, 6, 8, 3]), [1, 2, 3, 6, 8, 8]) &&
std.assertEqual(std.sort([9, 9, 5, 2, 7, 8, 1]), [1, 2, 5, 7, 8, 9, 9]) &&
std.assertEqual(std.sort([8, 0, 3, 2, 1, 7, 1, 6]), [0, 1, 1, 2, 3, 6, 7, 8]) &&
std.assertEqual(std.sort([5, 2, 0, 6, 0, 9, 9, 4, 9]), [0, 0, 2, 4, 5, 6, 9, 9, 9]) &&
std.assertEqual(std.sort([4, 4, 2, 7, 2, 5, 3, 9, 8, 9]), [2, 2, 3, 4, 4, 5, 7, 8, 9, 9]) &&

std.assertEqual(
  std.sort([[5, 1], [3, 2], [1, 3], [6, 4], [8, 4], [7, 6], [2, 7], [9, 8]], function(x) x[0]),
  [[1, 3], [2, 7], [3, 2], [5, 1], [6, 4], [7, 6], [8, 4], [9, 8]],
) &&

std.all([
  std.assertEqual(std.sort(std.repeat([1], n)), std.repeat([1], n)) &&
  std.assertEqual(std.sort(std.range(1, n)), std.range(1, n)) &&
  std.assertEqual(std.sort(std.reverse(std.range(1, n))), std.range(1, n)) &&
  std.assertEqual(std.sort(std.range(1, n), keyF=function(x) -x), std.reverse(std.range(1, n))) &&
  true

  for n in std.range(1, 150)
]) &&

true
