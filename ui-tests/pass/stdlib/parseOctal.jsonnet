std.assertEqual(std.parseOctal("0"), 0) &&
std.assertEqual(std.parseOctal("1"), 1) &&
std.assertEqual(std.parseOctal("01"), 1) &&
std.assertEqual(std.parseOctal("1234567"), 342391) &&

std.assertEqual(
  std.parseOctal("123456712345671234567123456712345671234567123456712345671234567"),
  1.281037429041102e+56,
) &&

true
