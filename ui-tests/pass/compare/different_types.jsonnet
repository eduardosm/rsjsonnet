local test(a, b) =
  std.assertEqual(a == b, false) &&
  std.assertEqual(b == a, false) &&
  std.assertEqual(a != b, true) &&
  std.assertEqual(b != a, true) &&
  std.assertEqual(std.equals(a, b), false) &&
  std.assertEqual(std.equals(b, a), false) &&
  std.assertEqual(std.primitiveEquals(a, b), false) &&
  std.assertEqual(std.primitiveEquals(b, a), false);

test(null, true) &&
test(null, 1.5) &&
test(null, "string") &&
test(null, [1, 2, 3]) &&
test(null, { a: 1, b: 2 }) &&
test(null, function() null) &&

test(true, 1.5) &&
test(true, "string") &&
test(true, [1, 2, 3]) &&
test(true, { a: 1, b: 2 }) &&
test(true, function() null) &&

test(1.5, "string") &&
test(1.5, [1, 2, 3]) &&
test(1.5, { a: 1, b: 2 }) &&
test(1.5, function() null) &&

test("string", [1, 2, 3]) &&
test("string", { a: 1, b: 2 }) &&
test("string", function() null) &&

test([1, 2, 3], { a: 1, b: 2 }) &&
test([1, 2, 3], function() null) &&

test({ a: 1, b: 2 }, function() null) &&

true
