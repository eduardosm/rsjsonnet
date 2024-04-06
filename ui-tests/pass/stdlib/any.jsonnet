std.assertEqual(std.any([]), false) &&
std.assertEqual(std.any([false]), false) &&
std.assertEqual(std.any([false, false]), false) &&
std.assertEqual(std.any([false, false, false]), false) &&
std.assertEqual(std.any([false, false, false, false]), false) &&

std.assertEqual(std.any([true]), true) &&
std.assertEqual(std.any([true, true]), true) &&
std.assertEqual(std.any([true, true, true]), true) &&
std.assertEqual(std.any([true, true, true, true]), true) &&

std.assertEqual(std.any([true, false, false]), true) &&
std.assertEqual(std.any([false, true, false]), true) &&
std.assertEqual(std.any([false, false, true]), true) &&

true
