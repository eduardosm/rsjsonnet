std.assertEqual(std.encodeUTF8(""), []) &&
std.assertEqual(std.encodeUTF8("123"), [49, 50, 51]) &&
std.assertEqual(std.encodeUTF8("\u03C0"), [207, 128]) &&
std.assertEqual(std.encodeUTF8("\uD800\uDD56"), [240, 144, 133, 150]) &&

true
