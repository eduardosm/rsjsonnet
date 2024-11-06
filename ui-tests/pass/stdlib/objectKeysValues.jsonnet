local test(obj, not_all, all) =
  std.assertEqual(std.objectKeysValues(obj), not_all) &&
  std.assertEqual(std.objectKeysValuesAll(obj), all);

local mkKv(key, value) = { key: key, value: value };

test({}, [], []) &&
test({ a: 1 }, [mkKv("a", 1)], [mkKv("a", 1)]) &&
test({ a: 1, b: 2 }, [mkKv("a", 1), mkKv("b", 2)], [mkKv("a", 1), mkKv("b", 2)]) &&
test({ a: 1, b:: 2 }, [mkKv("a", 1)], [mkKv("a", 1), mkKv("b", 2)]) &&
test({ a: 1, b::: 2 }, [mkKv("a", 1), mkKv("b", 2)], [mkKv("a", 1), mkKv("b", 2)]) &&

std.all([
  local kv = std.objectKeysValues(obj);
  local kvAll = std.objectKeysValuesAll(obj);

  test(obj + {}, kv, kvAll) &&
  test({} + obj, kv, kvAll) &&
  true

  for obj in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj + { a: 1 }, std.objectKeysValues(obj), [mkKv("a", 1)]) &&
  test(obj + { a:: 1 }, [], [mkKv("a", 1)]) &&
  test(obj + { a::: 1 }, [mkKv("a", 1)], [mkKv("a", 1)]) &&
  true

  for obj in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  local kv = std.objectKeysValues(obj1 + obj2);
  local kvAll = std.objectKeysValuesAll(obj1 + obj2);

  test(obj1 + obj2 + {}, kv, kvAll) &&
  test(obj1 + {} + obj2, kv, kvAll) &&
  test({} + obj1 + obj2, kv, kvAll) &&
  true

  for obj1 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{}, { a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

std.all([
  test(obj1 + obj2 + { a: 1 }, std.objectKeysValues(obj1 + obj2), [mkKv("a", 1)]) &&
  test(obj1 + obj2 + { a:: 1 }, [], [mkKv("a", 1)]) &&
  test(obj1 + obj2 + { a::: 1 }, [mkKv("a", 1)], [mkKv("a", 1)]) &&
  true

  for obj1 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
  for obj2 in [{ a: 1 }, { a:: 1 }, { a::: 1 }]
]) &&

true
