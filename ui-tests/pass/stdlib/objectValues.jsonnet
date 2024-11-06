local test(obj, not_all, all) =
  std.assertEqual(std.objectValues(obj), not_all) &&
  std.assertEqual(std.objectValuesAll(obj), all);

test({}, [], []) &&
test({ a: 1 }, [1], [1]) &&
test({ a: 1, b: 2 }, [1, 2], [1, 2]) &&
test({ a: 1, b:: 2 }, [1], [1, 2]) &&
test({ a: 1, b::: 2 }, [1, 2], [1, 2]) &&

std.all([
  local values = std.objectValues(obj);
  local valuesAll = std.objectValuesAll(obj);

  test(obj + {}, values, valuesAll) &&
  test({} + obj, values, valuesAll) &&
  true

  for obj in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj + { a: 1 }, std.objectValues(obj), [1]) &&
  test(obj + { a:: 1 }, [], [1]) &&
  test(obj + { a::: 1 }, [1], [1]) &&
  true

  for obj in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  local values = std.objectValues(obj1 + obj2);
  local valuesAll = std.objectValuesAll(obj1 + obj2);

  test(obj1 + obj2 + {}, values, valuesAll) &&
  test(obj1 + {} + obj2, values, valuesAll) &&
  test({} + obj1 + obj2, values, valuesAll) &&
  true

  for obj1 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj1 + obj2 + { a: 1 }, std.objectValues(obj1 + obj2), [1]) &&
  test(obj1 + obj2 + { a:: 1 }, [], [1]) &&
  test(obj1 + obj2 + { a::: 1 }, [1], [1]) &&
  true

  for obj1 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

true
