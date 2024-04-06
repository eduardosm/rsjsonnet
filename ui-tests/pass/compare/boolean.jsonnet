local test(a, b, eq) =
  std.assertEqual(a == b, eq) &&
  std.assertEqual(a != b, !eq) &&
  std.assertEqual(std.equals(a, b), eq) &&
  std.assertEqual(std.primitiveEquals(a, b), eq);

test(true, true, true) &&
test(false, false, true) &&
test(true, false, false) &&
test(false, true, false) &&

true
