local test(a, b, eq) =
  std.assertEqual(a == b, eq) &&
  std.assertEqual(a != b, !eq) &&
  std.assertEqual(std.equals(a, b), eq);

test({}, {}, true) &&
test({ a: 1 }, { a: 1 }, true) &&
test({ a: 1 }, { a: 1, b:: 2 }, true) &&
test({ a: 1, b: 2 }, { a: 1, b: 2 }, true) &&
test({ a: 1, b: 2, c: 3 }, { a: 1, b: 2, c: 3 }, true) &&

test({ a: 1, b:: 2 }, { a: 1 }, true) &&
test({ a: 1, b:: 2 }, { a: 1, b:: 3 }, true) &&

test({}, { a: 1 }, false) &&
test({ a: 1 }, {}, false) &&
test({ a: 1 }, { a: 2 }, false) &&
test({ a: 1 }, { a: 1, b: 2 }, false) &&
test({ a: 1, b: 2 }, { a: 1 }, false) &&
test({ a: 1, b: 2 }, { a: 1, b: 3 }, false) &&
test({ a: 1, b: 2, c: 3 }, { a: 1, b: 2, c: 4 }, false) &&

true
