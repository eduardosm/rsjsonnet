local test(obj, not_all, all) =
  std.assertEqual(std.objectFields(obj), not_all) &&
  std.assertEqual(std.objectFieldsAll(obj), all) &&
  std.assertEqual(std.objectFieldsEx(obj, false), not_all) &&
  std.assertEqual(std.objectFieldsEx(obj, true), all);

test({}, [], []) &&
test({ a: 1 }, ["a"], ["a"]) &&
test({ a: 1, b: 2 }, ["a", "b"], ["a", "b"]) &&
test({ a: 1, b:: 2 }, ["a"], ["a", "b"]) &&

true
