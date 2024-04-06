std.assertEqual(std.parseHex("0"), 0) &&
std.assertEqual(std.parseHex("1"), 1) &&
std.assertEqual(std.parseHex("01"), 1) &&
std.assertEqual(std.parseHex("123456789"), 4886718345) &&
std.assertEqual(std.parseHex("abcdef"), 11259375) &&
std.assertEqual(std.parseHex("ABCDEF"), 11259375) &&

local assertApprox(actual, expected) =
  if expected == 0 then
    std.assertEqual(actual, expected)
  else if (std.abs(actual - expected) / std.abs(expected)) < 1e-15 then
    true
  else
    error "assertApprox failed: " + actual + " != " + expected;

assertApprox(
  std.parseHex("123456789ABCDEF123456789ABCDEF123456789ABCDEF123456789ABCDEF"),
  1.2564245793979622e+71,
) &&

true
