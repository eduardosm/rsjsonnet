std.assertEqual(std.setMember(1, []), false) &&
std.assertEqual(std.setMember(1, [1]), true) &&
std.assertEqual(std.setMember(3, [1, 2, 3, 4, 5]), true) &&
std.assertEqual(std.setMember(3, [1, 2, 4, 5]), false) &&

std.assertEqual(std.setMember("nine", [], keyF=std.length), false) &&
std.assertEqual(std.setMember("nine", ["one", "three"], keyF=std.length), false) &&
std.assertEqual(std.setMember("nine", ["one", "four", "three"], keyF=std.length), true) &&

local n = 100, arr = std.range(1, n)[::2];
std.all([
    std.assertEqual(std.setMember(i, arr), i >= 1 && i <= n && (i % 2) == 1)
    for i in std.range(-200, 200)
]) &&

true
