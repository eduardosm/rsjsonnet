//@args: --max-stack 100

std.assertEqual(std.filterMap(function(x) error "err", function(x) error "err", []), []) &&
std.assertEqual(std.filterMap(function(x) false, function(x) error "err", [4, 1, 8, 7]), []) &&
std.assertEqual(std.filterMap(function(x) x % 2 == 0, function(x) x * 10, [6, 3, 9, 2]), [60, 20]) &&

// More items than the maximum stack size.
std.assertEqual(
  std.filterMap(function(x) x % 2 == 0, function(x) x / 2, std.range(1, 200)),
  std.range(1, 100),
) &&

true
