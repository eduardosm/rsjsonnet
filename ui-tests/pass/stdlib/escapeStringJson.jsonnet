std.assertEqual(std.escapeStringJson(""), @'""') &&
std.assertEqual(std.escapeStringJson("string"), @'"string"') &&
std.assertEqual(std.escapeStringJson(" \\ \" "), @'" \\ \" "') &&
std.assertEqual(std.escapeStringJson(" \t \r \n "), @'" \t \r \n "') &&
std.assertEqual(std.escapeStringJson(" \u0000 \u000F "), @'" \u0000 \u000f "') &&
std.assertEqual(std.escapeStringJson(" \u0019 \u001A \u001F "), @'" \u0019 \u001a \u001f "') &&

std.assertEqual(std.escapeStringJson({ a: 1 }), @'"{\"a\": 1}"') &&

std.all([
  std.assertEqual(std.parseJson(std.escapeStringJson(std.char(c))), std.char(c))
  for c in std.range(0, 159)
]) &&

true
