local f(arg1, arg2) =
  std.assertEqual(arg1, "string arg") &&
  std.assertEqual(arg2, { a: 1, b: 2 }) &&
  true;

f
