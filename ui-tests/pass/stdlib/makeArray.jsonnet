std.assertEqual(std.makeArray(0, function(x) error "err"), []) &&
std.assertEqual(std.makeArray(5, function(x) x), [0, 1, 2, 3, 4]) &&
std.assertEqual(std.makeArray(5, function(x) (x + 1) * 10), [10, 20, 30, 40, 50]) &&
std.assertEqual(std.makeArray(3, std.type), ["number", "number", "number"]) &&
std.assertEqual(std.makeArray(5, std.toString), ["0", "1", "2", "3", "4"]) &&

// Each element is lazily evaluated.
std.assertEqual(std.makeArray(3, function(p) 1 / p)[1:], [1, 0.5]) &&

true
