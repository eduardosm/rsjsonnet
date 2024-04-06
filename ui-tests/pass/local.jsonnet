std.assertEqual(local x = 1; 2, 2) &&
std.assertEqual(local x = 1; x, 1) &&
std.assertEqual(local x = 1, y = 2; [x, y], [1, 2]) &&
std.assertEqual(local x = y, y = 1; [x, y], [1, 1]) &&
std.assertEqual(local x = 1; local y = 2; [x, y], [1, 2]) &&
std.assertEqual(local x = 1; local y = x; [x, y], [1, 1]) &&
std.assertEqual(local x = 1, y = x; local x = 2; [x, y], [2, 1]) &&

true
