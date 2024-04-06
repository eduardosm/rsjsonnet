// Begin with some sanity checks
std.assertEqual(0 == 0, true) &&
std.assertEqual(0 != 0, false) &&
std.assertEqual(0 < 0, false) &&
std.assertEqual(0 <= 0, true) &&
std.assertEqual(0 > 0, false) &&
std.assertEqual(0 >= 0, true) &&

std.assertEqual(1 == 0, false) &&
std.assertEqual(1 != 0, true) &&
std.assertEqual(1 < 0, false) &&
std.assertEqual(1 <= 0, false) &&
std.assertEqual(1 > 0, true) &&
std.assertEqual(1 >= 0, true) &&

std.assertEqual(-1 == 0, false) &&
std.assertEqual(-1 != 0, true) &&
std.assertEqual(-1 < 0, true) &&
std.assertEqual(-1 <= 0, true) &&
std.assertEqual(-1 > 0, false) &&
std.assertEqual(-1 >= 0, false) &&

// Actual tests
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

test(1.5, 1.5, 0) &&
test(1.5, 2.25, -1) &&
test(2.25, 1.5, 1) &&
test(1.5, -1.5, 1) &&
test(-1.5, 1.5, -1) &&
test(0, -0, 0) &&
test(-0, 0, 0) &&

true
