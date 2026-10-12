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

# Keys that cannot be ordered are accepted when they are equal
std.assertEqual(std.setUnion([null], [null]), [null]) &&
std.assertEqual(std.setUnion([true], [true]), [true]) &&
std.assertEqual(std.setUnion([{ a: 1 }], [{ a: 1 }]), [{ a: 1 }]) &&
std.assertEqual(std.setUnion([[null]], [[null]]), [[null]]) &&

true
