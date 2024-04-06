std.assertEqual(std.escapeStringJson(""), @'""') &&
std.assertEqual(std.escapeStringJson("string"), @'"string"') &&
std.assertEqual(std.escapeStringJson(" \\ \" "), @'" \\ \" "') &&
std.assertEqual(std.escapeStringJson(" \t \r \n "), @'" \t \r \n "') &&
std.assertEqual(std.escapeStringJson(" \u0000 \u000F "), @'" \u0000 \u000f "') &&

std.assertEqual(std.escapeStringJson({ a: 1 }), @'"{\"a\": 1}"') &&

true
