std.assertEqual(std.substr("string", 0, 6), "string") &&
std.assertEqual(std.substr("string", 0, 7), "string") &&
std.assertEqual(std.substr("string", 0, 5), "strin") &&
std.assertEqual(std.substr("string", 1, 6), "tring") &&
std.assertEqual(std.substr("string", 1, 7), "tring") &&
std.assertEqual(std.substr("string", 10, 10), "") &&
std.assertEqual(std.substr("🧶🧺🧲🧢", 1, 2), "🧺🧲") &&

true
