std.assertEqual(std.setDiff([], []), []) &&
std.assertEqual(std.setDiff([1, 2, 3], []), [1, 2, 3]) &&
std.assertEqual(std.setDiff([], [1, 2, 3]), []) &&
std.assertEqual(std.setDiff([1, 2, 3], [1]), [2, 3]) &&
std.assertEqual(std.setDiff([1, 2, 3], [2]), [1, 3]) &&
std.assertEqual(std.setDiff([1, 2, 3], [3]), [1, 2]) &&
std.assertEqual(std.setDiff([1], [1, 2, 3]), []) &&
std.assertEqual(std.setDiff([2], [1, 2, 3]), []) &&
std.assertEqual(std.setDiff([3], [1, 2, 3]), []) &&
std.assertEqual(std.setDiff([1, 2, 3], [1, 3]), [2]) &&
std.assertEqual(std.setDiff([1, 3], [1, 2, 3]), []) &&
std.assertEqual(std.setDiff([1, 2], [1, 3]), [2]) &&
std.assertEqual(std.setDiff([1, 3], [1, 2]), [3]) &&

std.assertEqual(std.setDiff(["one", "four", "three"], ["two", "three"], keyF=std.length), ["four"]) &&

# Keys that cannot be ordered are accepted when they are equal
std.assertEqual(std.setDiff([null], [null]), []) &&
std.assertEqual(std.setDiff([true], [true]), []) &&
std.assertEqual(std.setDiff([{ a: 1 }], [{ a: 1 }]), []) &&
std.assertEqual(std.setDiff([[null]], [[null]]), []) &&

true
