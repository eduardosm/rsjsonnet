std.assertEqual(std.count([], null), 0) &&
std.assertEqual(std.count([null], null), 1) &&
std.assertEqual(std.count([null, 1, null], null), 2) &&
std.assertEqual(std.count([1, null, null], null), 2) &&
std.assertEqual(std.count([4, 3, 2, 1, 0], 1), 1) &&

std.assertEqual(std.count([], error "err"), 0) &&

true
