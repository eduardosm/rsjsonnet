std.assertEqual(std.setUnion([], []), []) &&
std.assertEqual(std.setUnion([1, 2, 3], []), [1, 2, 3]) &&
std.assertEqual(std.setUnion([], [1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1, 2, 3], [1]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1, 2, 3], [2]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1, 2, 3], [3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1], [1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([2], [1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([3], [1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1, 2, 3], [1, 3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1, 3], [1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1, 2], [1, 3]), [1, 2, 3]) &&
std.assertEqual(std.setUnion([1, 3], [1, 2]), [1, 2, 3]) &&

# Values from the first set win
std.assertEqual(
  std.setUnion([[1, 1]], [[1, 2]], function(x) x[0]),
  [[1, 1]]
) &&
std.assertEqual(
  std.setUnion([[1, 1], [3, 1]], [[2, 1], [3, 3]], function(x) x[0]),
  [[1, 1], [2, 1], [3, 1]],
) &&

true
