std.assertEqual(std.flattenArrays([]), []) &&
std.assertEqual(std.flattenArrays([[]]), []) &&
std.assertEqual(std.flattenArrays([[], [], []]), []) &&
std.assertEqual(std.flattenArrays([[1, 2]]), [1, 2]) &&
std.assertEqual(std.flattenArrays([[1, 2], ["a", "b"]]), [1, 2, "a", "b"]) &&
std.assertEqual(std.flattenArrays([[[], []], [[1, 2], ["a", "b"]]]), [[], [], [1, 2], ["a", "b"]]) &&

true
