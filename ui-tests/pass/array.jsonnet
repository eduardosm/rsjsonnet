std.assertEqual([] + [], []) &&
std.assertEqual([] + [1, 2], [1, 2]) &&
std.assertEqual([1, 2] + [], [1, 2]) &&
std.assertEqual([1, 2] + [3, 4], [1, 2, 3, 4]) &&

local arr = ["a", "b", "c"];
std.assertEqual(arr[0], "a") &&
std.assertEqual(arr[1], "b") &&
std.assertEqual(arr[2], "c") &&

local arr = [1, [2, 3, []]];
std.assertEqual(arr[0], 1) &&
std.assertEqual(arr[1], [2, 3, []]) &&
std.assertEqual(arr[1][0], 2) &&
std.assertEqual(arr[1][1], 3) &&
std.assertEqual(arr[1][2], []) &&

std.assertEqual([error "err", "x"][1], "x") &&

std.assertEqual([x + 100 for x in [1, 2, 3]], [101, 102, 103]) &&
std.assertEqual([x + 100 for x in [1, 2, 3, 4] if x % 2 == 0], [102, 104]) &&
std.assertEqual(
  [[x, y] for x in [1, 2] for y in [10 + x, 20 + x]],
  [[1, 11], [1, 21], [2, 12], [2, 22]],
) &&
std.assertEqual(
  [[x, y] for x in [1, 2, 3] if x != 2 for y in [10 + x, 20 + x]],
  [[1, 11], [1, 21], [3, 13], [3, 23]],
) &&
std.assertEqual(
  [[x, y] for x in [1, 2] for y in [10, 20, 30][x:]],
  [[1, 20], [1, 30], [2, 30]],
) &&

true
