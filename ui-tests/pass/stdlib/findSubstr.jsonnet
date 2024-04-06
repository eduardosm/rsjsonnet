std.assertEqual(std.findSubstr("sub", ""), []) &&
std.assertEqual(std.findSubstr("a", "aa"), [0, 1]) &&
std.assertEqual(std.findSubstr("a", "ab"), [0]) &&
std.assertEqual(std.findSubstr("b", "ab"), [1]) &&
std.assertEqual(std.findSubstr("", "string"), []) &&
std.assertEqual(std.findSubstr("aa", "aabaa"), [0, 3]) &&
std.assertEqual(std.findSubstr("ab", "aabaa"), [1]) &&

true
