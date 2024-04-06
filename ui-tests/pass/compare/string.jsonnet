local test(a, b, cmp) =
  std.assertEqual(a == b, cmp == 0) &&
  std.assertEqual(a != b, cmp != 0) &&
  std.assertEqual(a < b, cmp < 0) &&
  std.assertEqual(a <= b, cmp <= 0) &&
  std.assertEqual(a > b, cmp > 0) &&
  std.assertEqual(a >= b, cmp >= 0) &&
  std.assertEqual(std.equals(a, b), cmp == 0) &&
  std.assertEqual(std.primitiveEquals(a, b), cmp == 0) &&
  std.assertEqual(std.__compare(a, b), cmp);

test("string", "string", 0) &&
test("str", "string", -1) &&
test("string", "str", 1) &&
test("string", "String", 1) &&
test("String", "string", -1) &&
test("string", "strIng", 1) &&
test("strIng", "string", -1) &&
test("string", "stR", 1) &&
test("stR", "string", -1) &&
test("stRing", "str", -1) &&
test("str", "stRing", 1) &&

true
