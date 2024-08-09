std.assertEqual(std.findSubstr("", ""), []) &&

std.assertEqual(std.findSubstr("sub", ""), []) &&
std.assertEqual(std.findSubstr("a", "aa"), [0, 1]) &&
std.assertEqual(std.findSubstr("a", "ab"), [0]) &&
std.assertEqual(std.findSubstr("b", "ab"), [1]) &&
std.assertEqual(std.findSubstr("", "string"), []) &&
std.assertEqual(std.findSubstr("aa", "aabaa"), [0, 3]) &&
std.assertEqual(std.findSubstr("ab", "aabaa"), [1]) &&
std.assertEqual(std.findSubstr("aa", "aaa"), [0, 1]) &&

std.assertEqual(std.findSubstr("😎", "😎"), [0]) &&
std.assertEqual(std.findSubstr("😎", "😎 😎"), [0, 2]) &&
std.assertEqual(std.findSubstr("😎", "😎🙈😎"), [0, 2]) &&
std.assertEqual(std.findSubstr("b", "a😎b"), [2]) &&
std.assertEqual(std.findSubstr("😎😎", "😎😎😎"), [0, 1]) &&

true
