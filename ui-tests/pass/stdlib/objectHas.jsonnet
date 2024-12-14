local test(obj, field, not_all, all) =
  std.assertEqual(std.objectHas(obj, field), not_all) &&
  std.assertEqual(std.objectHasAll(obj, field), all) &&
  std.assertEqual(std.objectHasEx(obj, field, false), not_all) &&
  std.assertEqual(std.objectHasEx(obj, field, true), all);

test({}, "a", false, false) &&
test({ a: 1 }, "a", true, true) &&
test({ a: 1 }, "b", false, false) &&
test({ a:: 1 }, "a", false, true) &&
test({ a:: 1 }, "b", false, false) &&
test({ a::: 1 }, "a", true, true) &&
test({ a::: 1 }, "b", false, false) &&

std.all([
  local has = std.objectHas(obj, "a");
  local hasAll = std.objectHasAll(obj, "a");

  test(obj + {}, "a", has, hasAll) &&
  test(obj + {}, "b", false, false) &&
  test({} + obj, "a", has, hasAll) &&
  test({} + obj, "b", false, false) &&
  true

  for obj in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj + { a: 1 }, "a", std.objectHas(obj, "a"), true) &&
  test(obj + { a: 1 }, "b", false, false) &&
  test(obj + { a:: 1 }, "a", false, true) &&
  test(obj + { a:: 1 }, "b", false, false) &&
  test(obj + { a::: 1 }, "a", true, true) &&
  test(obj + { a::: 1 }, "b", false, false) &&
  true

  for obj in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  local has = std.objectHas(obj1 + obj2, "a");
  local hasAll = std.objectHasAll(obj1 + obj2, "a");

  test(obj1 + obj2 + {}, "a", has, hasAll) &&
  test(obj1 + obj2 + {}, "b", false, false) &&
  test(obj1 + {} + obj2, "a", has, hasAll) &&
  test(obj1 + {} + obj2, "b", false, false) &&
  test({} + obj1 + obj2, "a", has, hasAll) &&
  test({} + obj1 + obj2, "b", false, false) &&
  true

  for obj1 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj1 + obj2 + { a: 1 }, "a", std.objectHas(obj1 + obj2, "a"), true) &&
  test(obj1 + obj2 + { a: 1 }, "b", false, false) &&
  test(obj1 + obj2 + { a:: 1 }, "a", false, true) &&
  test(obj1 + obj2 + { a:: 1 }, "b", false, false) &&
  test(obj1 + obj2 + { a::: 1 }, "a", true, true) &&
  test(obj1 + obj2 + { a::: 1 }, "b", false, false) &&
  true

  for obj1 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

true
