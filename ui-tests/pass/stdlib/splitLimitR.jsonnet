std.assertEqual(std.splitLimitR("1,2,3", ",", -1), ["1", "2", "3"]) &&
std.assertEqual(std.splitLimitR("1,2,3", ",", 0), ["1,2,3"]) &&
std.assertEqual(std.splitLimitR("1,2,3", ",", 1), ["1,2", "3"]) &&
std.assertEqual(std.splitLimitR("1,2,3", ",", 2), ["1", "2", "3"]) &&
std.assertEqual(std.splitLimitR("1,2,3", ",", 3), ["1", "2", "3"]) &&
std.assertEqual(std.splitLimitR("1,2,3", ",", 4), ["1", "2", "3"]) &&

std.assertEqual(std.splitLimitR(",1,2,3", ",", -1), ["", "1", "2", "3"]) &&
std.assertEqual(std.splitLimitR(",1,2,3", ",", 0), [",1,2,3"]) &&
std.assertEqual(std.splitLimitR(",1,2,3", ",", 1), [",1,2", "3"]) &&
std.assertEqual(std.splitLimitR(",1,2,3", ",", 2), [",1", "2", "3"]) &&
std.assertEqual(std.splitLimitR(",1,2,3", ",", 3), ["", "1", "2", "3"]) &&
std.assertEqual(std.splitLimitR(",1,2,3", ",", 4), ["", "1", "2", "3"]) &&
std.assertEqual(std.splitLimitR(",1,2,3", ",", 5), ["", "1", "2", "3"]) &&

true
