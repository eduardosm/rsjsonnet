std.assertEqual(std.minArray([], onEmpty="was" + " empty"), "was empty") &&
std.assertEqual(std.minArray([1]), 1) &&
std.assertEqual(std.minArray([1], onEmpty=error "was empty"), 1) &&
std.assertEqual(std.minArray([1, 2]), 1) &&
std.assertEqual(std.minArray([2, 1]), 1) &&
std.assertEqual(std.minArray([1, 2, 3]), 1) &&
std.assertEqual(std.minArray([3, 2, 1]), 1) &&
std.assertEqual(std.minArray([2, 1, 3]), 1) &&
std.assertEqual(std.minArray(["one", "two", "three"], keyF=std.length), "one") &&
std.assertEqual(std.minArray(["three", "two", "one"], keyF=std.length), "two") &&

true
