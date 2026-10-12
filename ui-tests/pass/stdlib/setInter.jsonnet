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

# Keys that cannot be ordered are accepted when they are equal
std.assertEqual(std.setInter([null], [null]), [null]) &&
std.assertEqual(std.setInter([true], [true]), [true]) &&
std.assertEqual(std.setInter([{ a: 1 }], [{ a: 1 }]), [{ a: 1 }]) &&
std.assertEqual(std.setInter([[null]], [[null]]), [[null]]) &&

true
