std.assertEqual(std.endsWith("string", "abc"), false) &&
std.assertEqual(std.endsWith("string", ""), true) &&
std.assertEqual(std.endsWith("string", "ing"), true) &&
std.assertEqual(std.endsWith("string", "string"), true) &&
std.assertEqual(std.endsWith("string", "1string"), false) &&
std.assertEqual(std.endsWith("", "string"), false) &&

true
