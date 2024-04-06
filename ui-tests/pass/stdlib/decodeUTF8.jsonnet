std.assertEqual(std.decodeUTF8([]), "") &&
std.assertEqual(std.decodeUTF8([49, 50, 51]), "123") &&
std.assertEqual(std.decodeUTF8([207, 128]), "\u03C0") &&
std.assertEqual(std.decodeUTF8([240, 144, 133, 150]), "\uD800\uDD56") &&
std.assertEqual(std.decodeUTF8([255]), "\uFFFD") &&

true
