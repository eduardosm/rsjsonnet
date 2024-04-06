std.assertEqual(std.foldr(function(x, y) [x, y], [], 'x'), 'x') &&
std.assertEqual(std.foldr(function(x, y) [x, y], [1], 'x'), [1, 'x']) &&
std.assertEqual(std.foldr(function(x, y) [x, y], [1, 2], 'x'), [1, [2, 'x']]) &&
std.assertEqual(std.foldr(function(x, y) [x, y], [1, 2, 3], 'x'), [1, [2, [3, 'x']]]) &&

true
