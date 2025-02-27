std.assertEqual(std.flattenDeepArray([]), []) &&
std.assertEqual(std.flattenDeepArray("x"), ["x"]) &&
std.assertEqual(std.flattenDeepArray([1, 2, 3]), [1, 2, 3]) &&
std.assertEqual(std.flattenDeepArray([1, ["a", 3], [null]]), [1, "a", 3, null]) &&
std.assertEqual(std.flattenDeepArray([["x", 2], 3, 4]), ["x", 2, 3, 4]) &&
std.assertEqual(std.flattenDeepArray([1, [], [2, 3]]), [1, 2, 3]) &&
std.assertEqual(std.flattenDeepArray([[], 1, [2, 3]]), [1, 2, 3]) &&
std.assertEqual(std.flattenDeepArray(
  ["a", ["b", [1, 2, [], [[], 3, [4]]]],
  ["c", "d"]]), ["a", "b", 1, 2, 3, 4, "c", "d"],
) &&

true
