std.assertEqual(std.member([], "a"), false) &&
std.assertEqual(std.member(["a", "b", "c"], "a"), true) &&
std.assertEqual(std.member(["a", "b", "c"], "b"), true) &&
std.assertEqual(std.member(["a", "b", "c"], "c"), true) &&
std.assertEqual(std.member(["a", "b", "c"], "d"), false) &&

std.assertEqual(std.member("", "a"), false) &&
std.assertEqual(std.member("abc", "a"), true) &&
std.assertEqual(std.member("abc", "b"), true) &&
std.assertEqual(std.member("abc", "c"), true) &&
std.assertEqual(std.member("abc", "d"), false) &&
std.assertEqual(std.member("💯👍💯", "👍"), true) &&
std.assertEqual(std.member("💯👍💯", "👎"), false) &&

std.assertEqual(std.member([], error "err"), false) &&

true
