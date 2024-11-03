std.assertEqual("" + "string", "string") &&
std.assertEqual("string" + "", "string") &&
std.assertEqual("str" + "ing", "string") &&

std.assertEqual("string"[0], "s") &&
std.assertEqual("string"[1], "t") &&
std.assertEqual("string"[2], "r") &&
std.assertEqual("string"[3], "i") &&
std.assertEqual("string"[4], "n") &&
std.assertEqual("string"[5], "g") &&

std.assertEqual("\uEEEE\uFFFF"[0], "\uEEEE") &&
std.assertEqual("\uEEEE\uFFFF"[1], "\uFFFF") &&
std.assertEqual("🧶🧺🧲🧢"[0], "🧶") &&
std.assertEqual("🧶🧺🧲🧢"[1], "🧺") &&
std.assertEqual("🧶🧺🧲🧢"[2], "🧲") &&
std.assertEqual("🧶🧺🧲🧢"[3], "🧢") &&

std.assertEqual(null + "_", "null_") &&
std.assertEqual(true + "_", "true_") &&
std.assertEqual(false + "_", "false_") &&
std.assertEqual(1.5 + "_", "1.5_") &&
std.assertEqual([] + "_", "[ ]_") &&
std.assertEqual({} + "_", "{ }_") &&

std.assertEqual("_" + null, "_null") &&
std.assertEqual("_" + true, "_true") &&
std.assertEqual("_" + false, "_false") &&
std.assertEqual("_" + 1.5, "_1.5") &&
std.assertEqual("_" + [], "_[ ]") &&
std.assertEqual("_" + {}, "_{ }") &&

true
