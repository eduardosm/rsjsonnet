std.assertEqual(std.parseOctal("0"), 0) &&
std.assertEqual(std.parseOctal("1"), 1) &&
std.assertEqual(std.parseOctal("01"), 1) &&
std.assertEqual(std.parseOctal("1234567"), 342391) &&

local assertApprox(actual, expected) =
  if expected == 0 then
    std.assertEqual(actual, expected)
  else if (std.abs(actual - expected) / std.abs(expected)) < 1e-15 then
    true
  else
    error "assertApprox failed: " + actual + " != " + expected;

assertApprox(
  std.parseOctal("123456712345671234567123456712345671234567123456712345671234567"),
  1.281037429041102e+56,
) &&

true
