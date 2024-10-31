std.assertEqual(1.5 + 1.75, 3.25) &&
std.assertEqual(1.5 - 1.25, 0.25) &&
std.assertEqual(1.5 + (-1.25), 0.25) &&
std.assertEqual(1.5 * 1.75, 2.625) &&
std.assertEqual(1.5 / 2, 0.75) &&
std.assertEqual(1.5 / 2, 0.75) &&

std.assertEqual(5.5 % 2, 1.5) &&
std.assertEqual(5.5 % (-2), 1.5) &&
std.assertEqual((-5.5) % 2, -1.5) &&
std.assertEqual((-5.5) % (-2), -1.5) &&

std.assertEqual(std.mod(5.5, 2), 1.5) &&
std.assertEqual(std.mod(5.5, -2), 1.5) &&
std.assertEqual(std.mod(-5.5, 2), -1.5) &&
std.assertEqual(std.mod(-5.5, -2), -1.5) &&

std.assertEqual(5 << 4, 80) &&
std.assertEqual(5 >> 2, 1) &&

std.assertEqual(6 & 3, 2) &&
std.assertEqual(6 | 3, 7) &&
std.assertEqual(6 ^ 3, 5) &&

std.assertEqual(0, -0) &&
std.assertEqual(-0, 0) &&
std.assertEqual(-(-1.5), 1.5) &&
std.assertEqual(+1.5, 1.5) &&
std.assertEqual(~1, -2) &&
std.assertEqual(~(-1), 0) &&

true
