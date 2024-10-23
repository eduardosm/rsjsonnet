local test(obj, not_all, all) =
  std.assertEqual(std.objectFields(obj), not_all) &&
  std.assertEqual(std.objectFieldsAll(obj), all) &&
  std.assertEqual(std.objectFieldsEx(obj, false), not_all) &&
  std.assertEqual(std.objectFieldsEx(obj, true), all);

test({}, [], []) &&
test({ a: 1 }, ["a"], ["a"]) &&
test({ a: 1, b: 2 }, ["a", "b"], ["a", "b"]) &&
test({ a: 1, b:: 2 }, ["a"], ["a", "b"]) &&
test({ a: 1, b::: 2 }, ["a", "b"], ["a", "b"]) &&

std.all([
  local fields = std.objectFields(obj);
  local fieldsAll = std.objectFieldsAll(obj);

  test(obj + {}, fields, fieldsAll) &&
  test({} + obj, fields, fieldsAll) &&
  true

  for obj in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj + { a: 1 }, std.objectFields(obj), ["a"]) &&
  test(obj + { a:: 1 }, [], ["a"]) &&
  test(obj + { a::: 1 }, ["a"], ["a"]) &&
  true

  for obj in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  local fields = std.objectFields(obj1 + obj2);
  local fieldsAll = std.objectFieldsAll(obj1 + obj2);

  test(obj1 + obj2 + {}, fields, fieldsAll) &&
  test(obj1 + {} + obj2, fields, fieldsAll) &&
  test({} + obj1 + obj2, fields, fieldsAll) &&
  true

  for obj1 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj1 + obj2 + { a: 1 }, std.objectFields(obj1 + obj2), ["a"]) &&
  test(obj1 + obj2 + { a:: 1 }, [], ["a"]) &&
  test(obj1 + obj2 + { a::: 1 }, ["a"], ["a"]) &&
  true

  for obj1 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

true
