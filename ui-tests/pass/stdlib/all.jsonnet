std.assertEqual(std.all([]), true) &&
std.assertEqual(std.all([true]), true) &&
std.assertEqual(std.all([true, true]), true) &&
std.assertEqual(std.all([true, true, true]), true) &&
std.assertEqual(std.all([true, true, true, true]), true) &&

std.assertEqual(std.all([false]), false) &&
std.assertEqual(std.all([false, false]), false) &&
std.assertEqual(std.all([false, false, false]), false) &&
std.assertEqual(std.all([false, false, false, false]), false) &&

std.assertEqual(std.all([false, true, true]), false) &&
std.assertEqual(std.all([true, false, true]), false) &&
std.assertEqual(std.all([true, true, false]), false) &&

true
