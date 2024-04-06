std.assertEqual(std.join(",", []), "") &&
std.assertEqual(std.join(",", ["1"]), "1") &&
std.assertEqual(std.join(",", ["1", "2"]), "1,2") &&
std.assertEqual(std.join(",", ["1", "2", "3"]), "1,2,3") &&
std.assertEqual(std.join(",", [null]), "") &&
std.assertEqual(std.join(",", [null, "1", "2"]), "1,2") &&
std.assertEqual(std.join(",", ["1", null, "2"]), "1,2") &&
std.assertEqual(std.join(",", ["1", "2", null]), "1,2") &&

std.assertEqual(std.join([0], []), []) &&
std.assertEqual(std.join([0], [[1]]), [1]) &&
std.assertEqual(std.join([0], [[1], [2]]), [1, 0, 2]) &&
std.assertEqual(std.join([0], [[1], [2], [3]]), [1, 0, 2, 0, 3]) &&
std.assertEqual(std.join([0], [null]), []) &&
std.assertEqual(std.join([0], [null, [1], [2]]), [1, 0, 2]) &&
std.assertEqual(std.join([0], [[1], null, [2]]), [1, 0, 2]) &&
std.assertEqual(std.join([0], [[1], [2], null]), [1, 0, 2]) &&

true
