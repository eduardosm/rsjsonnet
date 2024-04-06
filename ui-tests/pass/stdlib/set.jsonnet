std.assertEqual(std.set([]), []) &&
std.assertEqual(std.set([1]), [1]) &&
std.assertEqual(std.set([1, 1]), [1]) &&
std.assertEqual(std.set([1, 2]), [1, 2]) &&
std.assertEqual(std.set([2, 1]), [1, 2]) &&
std.assertEqual(std.set([1, 2, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.set([1, 1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.set([1, 2, 3, 3]), [1, 2, 3]) &&
std.assertEqual(std.set([3, 1, 1, 2]), [1, 2, 3]) &&
std.assertEqual(std.set([2, 3, 1, 2]), [1, 2, 3]) &&
std.assertEqual(std.set([1, 1, 1, 1]), [1]) &&

std.assertEqual(std.set([], keyF=function(x) error "err"), []) &&
std.assertEqual(
  std.set(["one", "two", "three", "four", "six", "seven"], keyF=std.length),
  ["one", "four", "three"],
) &&

true
