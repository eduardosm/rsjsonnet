std.assertEqual(std.toString(std.parseInt("00")), "0") &&
std.assertEqual(std.toString(std.parseInt("-00")), "-0") &&

std.assertEqual(std.parseInt("0"), 0) &&
std.assertEqual(std.parseInt("-0"), 0) &&

std.assertEqual(std.parseInt("1"), 1) &&
std.assertEqual(std.parseInt("-1"), -1) &&

std.assertEqual(std.parseInt("01"), 1) &&
std.assertEqual(std.parseInt("-01"), -1) &&

std.assertEqual(std.parseInt("123456789"), 123456789) &&
std.assertEqual(std.parseInt("-123456789"), -123456789) &&

local assertApprox(actual, expected) =
  if expected == 0 then
    std.assertEqual(actual, expected)
  else if (std.abs(actual - expected) / std.abs(expected)) < 1e-15 then
    true
  else
    error "assertApprox failed: " + actual + " != " + expected;

assertApprox(
  std.parseInt("123456789123456789123456789123456789123456789123456789123456789123456789"),
  1.2345678912345678e+71,
) &&

true
