std.assertEqual(std.lines([]), "") &&
std.assertEqual(std.lines([null]), "") &&
std.assertEqual(std.lines(["a"]), "a\n") &&
std.assertEqual(std.lines(["a", null]), "a\n") &&
std.assertEqual(std.lines(["a", "b"]), "a\nb\n") &&
std.assertEqual(std.lines(["a", "b", "c"]), "a\nb\nc\n") &&
std.assertEqual(std.lines(["a", null, "b"]), "a\nb\n") &&

true
