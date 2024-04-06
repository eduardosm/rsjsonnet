local test(a, b, cmp) =
  std.assertEqual(a == b, cmp == 0) &&
  std.assertEqual(a != b, cmp != 0) &&
  std.assertEqual(a < b, cmp < 0) &&
  std.assertEqual(a <= b, cmp <= 0) &&
  std.assertEqual(a > b, cmp > 0) &&
  std.assertEqual(a >= b, cmp >= 0) &&
  std.assertEqual(std.equals(a, b), cmp == 0) &&
  std.assertEqual(std.__compare(a, b), cmp) &&
  std.assertEqual(std.__compare_array(a, b), cmp);

test(['a', 'r', 'r', 'a', 'y'], ['a', 'r', 'r', 'a', 'y'], 0) &&
test(['a', 'r', 'r'], ['a', 'r', 'r', 'a', 'y'], -1) &&
test(['a', 'r', 'r', 'a', 'y'], ['a', 'r', 'r'], 1) &&
test(['a', 'r', 'r', 'a', 'y'], ['A', 'r', 'r', 'a', 'y'], 1) &&
test(['A', 'r', 'r', 'a', 'y'], ['a', 'r', 'r', 'a', 'y'], -1) &&
test(['a', 'r', 'r', 'a', 'y'], ['a', 'r', 'R', 'a', 'y'], 1) &&
test(['a', 'r', 'R', 'a', 'y'], ['a', 'r', 'r', 'a', 'y'], -1) &&
test(['a', 'r', 'r', 'a', 'y'], ['a', 'r', 'r', 'a', 'Y'], 1) &&
test(['a', 'r', 'r', 'a', 'Y'], ['a', 'r', 'r', 'a', 'y'], -1) &&
test(['a', 'r', 'R'], ['a', 'r', 'r', 'a', 'y'], -1) &&
test(['a', 'r', 'r', 'a', 'y'], ['a', 'r', 'R'], 1) &&
test(['a', 'r', 'r'], ['a', 'r', 'R', 'a', 'y'], 1) &&
test(['a', 'r', 'R', 'a', 'y'], ['a', 'r', 'r'], -1) &&

test([1, error "err"], [1], 1) &&
test([1], [1, error "err"], -1) &&
test([1, error "err"], [2], -1) &&
test([1], [2, error "err"], -1) &&
test([1, error "err"], [2, 3], -1) &&
test([1, 2], [2, error "err"], -1) &&

true
