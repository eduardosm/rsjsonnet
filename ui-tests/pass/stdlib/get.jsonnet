std.assertEqual(std.get({}, "a"), null) &&
std.assertEqual(std.get({}, "a", "?"), "?") &&
std.assertEqual(std.get({}, "a", "?", true), "?") &&
std.assertEqual(std.get({}, "a", "?", false), "?") &&

std.assertEqual(std.get({ a: 1 }, "a"), 1) &&
std.assertEqual(std.get({ a: 1 }, "a", "?"), 1) &&
std.assertEqual(std.get({ a: 1 }, "a", "?", true), 1) &&
std.assertEqual(std.get({ a: 1 }, "a", "?", false), 1) &&

std.assertEqual(std.get({ a: 1 }, "b"), null) &&
std.assertEqual(std.get({ a: 1 }, "b", "?"), "?") &&
std.assertEqual(std.get({ a: 1 }, "b", "?", true), "?") &&
std.assertEqual(std.get({ a: 1 }, "b", "?", false), "?") &&

std.assertEqual(std.get({ a:: 1 }, "a"), 1) &&
std.assertEqual(std.get({ a:: 1 }, "a", "?"), 1) &&
std.assertEqual(std.get({ a:: 1 }, "a", "?", true), 1) &&
std.assertEqual(std.get({ a:: 1 }, "a", "?", false), "?") &&

std.assertEqual(std.get({ a:: 1 }, "b"), null) &&
std.assertEqual(std.get({ a:: 1 }, "b", "?"), "?") &&
std.assertEqual(std.get({ a:: 1 }, "b", "?", true), "?") &&
std.assertEqual(std.get({ a:: 1 }, "b", "?", false), "?") &&

true
