std.assertEqual(std.strReplace("", "a", "b"), "") &&
std.assertEqual(std.strReplace("a", "b", "c"), "a") &&
std.assertEqual(std.strReplace("a", "a", "b"), "b") &&
std.assertEqual(std.strReplace("1a2a3a4", "a", "b"), "1b2b3b4") &&
std.assertEqual(std.strReplace("1aa2aa3", "aa", "bb"), "1bb2bb3") &&
std.assertEqual(std.strReplace("💯👎💯", "👎", "👍"), "💯👍💯") &&

true
