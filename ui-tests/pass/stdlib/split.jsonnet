std.assertEqual(std.split("", ","), [""]) &&
std.assertEqual(std.split("1,2,3", ","), ["1", "2", "3"]) &&
std.assertEqual(std.split("1,2,3,", ","), ["1", "2", "3", ""]) &&

true
