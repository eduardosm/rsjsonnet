std.assertEqual(std.trim(""), "") &&
std.assertEqual(std.trim(" \t\n\r\f\u0085\u00A0"), "") &&
std.assertEqual(std.trim(" \t\n\r\f\u0085\u00A0string"), "string") &&
std.assertEqual(std.trim("string \t\n\r\f\u0085\u00A0"), "string") &&
std.assertEqual(std.trim("\n string\t\r"), "string") &&

true
