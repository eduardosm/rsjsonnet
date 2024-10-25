std.assertEqual(std.find(null, []), []) &&
std.assertEqual(std.find(null, [null]), [0]) &&
std.assertEqual(std.find(null, [null, 1, null]), [0, 2]) &&
std.assertEqual(std.find(null, [1, null, null]), [1, 2]) &&
std.assertEqual(std.find(1, [4, 3, 2, 1, 0]), [3]) &&

std.assertEqual(std.find(error "err", []), []) &&

true
