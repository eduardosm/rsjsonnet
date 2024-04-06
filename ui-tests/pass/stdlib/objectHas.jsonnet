local test(obj, field, not_all, all) =
  std.assertEqual(std.objectHas(obj, field), not_all) &&
  std.assertEqual(std.objectHasAll(obj, field), all) &&
  std.assertEqual(std.objectHasEx(obj, field, false), not_all) &&
  std.assertEqual(std.objectHasEx(obj, field, true), all);

test({}, "a", false, false) &&
test({ a: 1 }, "a", true, true) &&
test({ a:: 1 }, "a", false, true) &&

true
