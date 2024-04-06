std.assertEqual(std.reverse([]), []) &&
std.assertEqual(std.reverse([1]), [1]) &&
std.assertEqual(std.reverse([1, 5]), [5, 1]) &&
std.assertEqual(std.reverse([1, 3, 2]), [2, 3, 1]) &&
std.assertEqual(std.reverse([1, 3, 2, 4]), [4, 2, 3, 1]) &&
std.assertEqual(std.reverse([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]), [9, 8, 7, 6, 5, 4, 3, 2, 1, 0]) &&
std.assertEqual(std.reverse([1, 1, 1, 1]), [1, 1, 1, 1]) &&
std.assertEqual(std.reverse([[1, 2], [3, 4], [5, 6]]), [[5, 6], [3, 4], [1, 2]]) &&

true
