std.assertEqual(std.repeat("", 0), "") &&
std.assertEqual(std.repeat("", 1), "") &&
std.assertEqual(std.repeat("", 2), "") &&
std.assertEqual(std.repeat("", 1000), "") &&
std.assertEqual(std.repeat("a", 0), "") &&
std.assertEqual(std.repeat("a", 1), "a") &&
std.assertEqual(std.repeat("a", 2), "aa") &&
std.assertEqual(std.repeat("a", 5), "aaaaa") &&
std.assertEqual(std.repeat("ab", 0), "") &&
std.assertEqual(std.repeat("ab", 1), "ab") &&
std.assertEqual(std.repeat("ab", 2), "abab") &&
std.assertEqual(std.repeat("ab", 5), "ababababab") &&

std.assertEqual(std.repeat([], 0), []) &&
std.assertEqual(std.repeat([], 1), []) &&
std.assertEqual(std.repeat([], 2), []) &&
std.assertEqual(std.repeat([], 1000), []) &&
std.assertEqual(std.repeat([1], 0), []) &&
std.assertEqual(std.repeat([1], 1), [1]) &&
std.assertEqual(std.repeat([1], 2), [1, 1]) &&
std.assertEqual(std.repeat([1], 5), [1, 1, 1, 1, 1]) &&
std.assertEqual(std.repeat(["a", "b"], 0), []) &&
std.assertEqual(std.repeat(["a", "b"], 1), ["a", "b"]) &&
std.assertEqual(std.repeat(["a", "b"], 2), ["a", "b", "a", "b"]) &&
std.assertEqual(std.repeat(["a", "b"], 5), ["a", "b", "a", "b", "a", "b", "a", "b", "a", "b"]) &&

true
