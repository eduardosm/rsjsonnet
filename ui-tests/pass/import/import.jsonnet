std.assertEqual((import "lib/func.libsonnet")(2, 3), [1, 2, 5]) &&
std.assertEqual((import "lib/indirect.libsonnet")(2, 3), [1, 2, 5]) &&

std.assertEqual((local x = 2; import "lib/func.libsonnet")(2, 3), [1, 2, 5]) &&

std.assertEqual(importstr "lib/utf8.txt", "This is some UTF-8 text 🙂\n") &&

std.assertEqual(importbin "lib/non_utf8.bin", [1, 0, 255]) &&

true
