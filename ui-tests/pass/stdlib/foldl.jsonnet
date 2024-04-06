std.assertEqual(std.foldl(function(x, y) [x, y], [], 'x'), 'x') &&
std.assertEqual(std.foldl(function(x, y) [x, y], [1], 'x'), ['x', 1]) &&
std.assertEqual(std.foldl(function(x, y) [x, y], [1, 2], 'x'), [['x', 1], 2]) &&
std.assertEqual(std.foldl(function(x, y) [x, y], [1, 2, 3], 'x'), [[['x', 1], 2], 3]) &&

true
