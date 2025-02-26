std.assertEqual(std.maxArray([], onEmpty="was" + " empty"), "was empty") &&
std.assertEqual(std.maxArray([1]), 1) &&
std.assertEqual(std.maxArray([1], onEmpty=error "was empty"), 1) &&
std.assertEqual(std.maxArray([1, 2]), 2) &&
std.assertEqual(std.maxArray([2, 1]), 2) &&
std.assertEqual(std.maxArray([1, 2, 3]), 3) &&
std.assertEqual(std.maxArray([3, 2, 1]), 3) &&
std.assertEqual(std.maxArray([1, 3, 2]), 3) &&
std.assertEqual(std.maxArray(["a3", "b2", "c1"], keyF=function(v) v[1]), "a3") &&
std.assertEqual(std.maxArray(["c1", "b2", "a3"], keyF=function(v) v[1]), "a3") &&
std.assertEqual(std.maxArray(["seven", "eight", "nine"], keyF=std.length), "seven") &&
std.assertEqual(std.maxArray(["nine", "eight", "seven"], keyF=std.length), "eight") &&

true
