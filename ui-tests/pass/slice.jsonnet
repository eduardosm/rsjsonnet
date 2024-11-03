local test(indexable, index, end, step, expected) =
  std.assertEqual(indexable[index:end:step], expected) &&
  std.assertEqual(std.slice(indexable, index, end, step), expected);

// string slice

test("string", null, null, null, "string") &&

test("string", 0, null, null, "string") &&
test("string", 1, null, null, "tring") &&
test("string", 2, null, null, "ring") &&

test("string", null, 4, null, "stri") &&
test("string", null, 5, null, "strin") &&
test("string", null, 6, null, "string") &&

test("string", null, null, 1, "string") &&
test("string", null, null, 2, "srn") &&
test("string", null, null, 3, "si") &&

test("string", 0, 6, null, "string") &&
test("string", 1, 5, null, "trin") &&
test("string", 2, 4, null, "ri") &&

test("string", 0, null, 1, "string") &&
test("string", 1, null, 2, "tig") &&
test("string", 1, null, 3, "tn") &&

test("string", null, 6, 1, "string") &&
test("string", null, 5, 2, "srn") &&
test("string", null, 4, 2, "sr") &&

test("string", 0, 6, 1, "string") &&
test("string", 1, 6, 2, "tig") &&
test("string", 1, 5, 2, "ti") &&

std.assertEqual("string"[::], "string") &&
std.assertEqual("string"[1::], "tring") &&
std.assertEqual("string"[:5:], "strin") &&
std.assertEqual("string"[::2], "srn") &&

test("🧶🧺🧲🧢", 1, 3, null, "🧺🧲") &&
test("🧶🧺🧲🧢", 0, 3, 2, "🧶🧲") &&

// array slice

local arr = ['a', 'r', 'r', 'a', 'y', '.'];

test(arr, null, null, null, arr) &&

test(arr, 0, null, null, arr) &&
test(arr, 1, null, null, ['r', 'r', 'a', 'y', '.']) &&
test(arr, 2, null, null, ['r', 'a', 'y', '.']) &&

test(arr, null, 4, null, ['a', 'r', 'r', 'a']) &&
test(arr, null, 5, null, ['a', 'r', 'r', 'a', 'y']) &&
test(arr, null, 6, null, arr) &&

test(arr, null, null, 1, arr) &&
test(arr, null, null, 2, ['a', 'r', 'y']) &&
test(arr, null, null, 3, ['a', 'a']) &&

test(arr, 0, 6, null, arr) &&
test(arr, 1, 5, null, ['r', 'r', 'a', 'y']) &&
test(arr, 2, 4, null, ['r', 'a']) &&

test(arr, 0, null, 1, arr) &&
test(arr, 1, null, 2, ['r', 'a', '.']) &&
test(arr, 1, null, 3, ['r', 'y']) &&

test(arr, null, 6, 1, arr) &&
test(arr, null, 5, 2, ['a', 'r', 'y']) &&
test(arr, null, 4, 2, ['a', 'r']) &&

test(arr, 0, 6, 1, arr) &&
test(arr, 1, 6, 2, ['r', 'a', '.']) &&
test(arr, 1, 5, 2, ['r', 'a']) &&

std.assertEqual(arr[::], ['a', 'r', 'r', 'a', 'y', '.']) &&
std.assertEqual(arr[1::], ['r', 'r', 'a', 'y', '.']) &&
std.assertEqual(arr[:5:], ['a', 'r', 'r', 'a', 'y']) &&
std.assertEqual(arr[::2], ['a', 'r', 'y']) &&

true
