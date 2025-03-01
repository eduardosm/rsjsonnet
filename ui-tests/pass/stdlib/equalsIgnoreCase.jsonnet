std.assertEqual(std.equalsIgnoreCase("", ""), true) &&
std.assertEqual(std.equalsIgnoreCase("", "x"), false) &&
std.assertEqual(std.equalsIgnoreCase("x", ""), false) &&
std.assertEqual(std.equalsIgnoreCase("x", "x"), true) &&
std.assertEqual(std.equalsIgnoreCase("X", "x"), true) &&
std.assertEqual(std.equalsIgnoreCase("X", "y"), false) &&
std.assertEqual(std.equalsIgnoreCase("_aBcDeFgHi_", "_AbCdEfGhI_"), true) &&
std.assertEqual(std.equalsIgnoreCase("_aBcDeFgHi_", "_AbCdXfGhI_"), false) &&

true
