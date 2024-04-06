std.assertEqual(std.setInter([], []), []) &&
std.assertEqual(std.setInter([1, 2, 3], [ ]), []) &&
std.assertEqual(std.setInter([], [1, 2, 3]), []) &&
std.assertEqual(std.setInter([1, 2, 3], [1]), [1]) &&
std.assertEqual(std.setInter([1, 2, 3], [2]), [2]) &&
std.assertEqual(std.setInter([1, 2, 3], [3]), [3]) &&
std.assertEqual(std.setInter([1], [1, 2, 3]), [1]) &&
std.assertEqual(std.setInter([2], [1, 2, 3]), [2]) &&
std.assertEqual(std.setInter([3], [1, 2, 3]), [3]) &&
std.assertEqual(std.setInter([1, 2, 3], [1, 3]), [1, 3]) &&
std.assertEqual(std.setInter([1, 3], [1, 2, 3]), [1, 3]) &&
std.assertEqual(std.setInter([1, 2], [1, 3]), [1]) &&
std.assertEqual(std.setInter([1, 3], [1, 2]), [1]) &&

std.assertEqual(std.setInter(["one", "four", "three"], ["two", "three"], keyF=std.length), ["one", "three"]) &&

true
