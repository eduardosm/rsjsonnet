std.assertEqual(if true then "x", "x") &&
std.assertEqual(if false then "x", null) &&

std.assertEqual(if true then "x" else "y", "x") &&
std.assertEqual(if false then "x" else "y", "y") &&

std.assertEqual(if false then error "err", null) &&
std.assertEqual(if true then "x" else error "err", "x") &&
std.assertEqual(if false then error "err" else "y", "y") &&

true
