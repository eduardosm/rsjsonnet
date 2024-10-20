std.assertEqual(std.escapeStringPython(""), @'""') &&
std.assertEqual(std.escapeStringPython("string"), @'"string"') &&
std.assertEqual(std.escapeStringPython(" \\ \" "), @'" \\ \" "') &&
std.assertEqual(std.escapeStringPython(" \t \r \n "), @'" \t \r \n "') &&
std.assertEqual(std.escapeStringPython(" \u0000 \u000F "), @'" \u0000 \u000f "') &&

std.assertEqual(std.escapeStringPython({ a: 1 }), @'"{\"a\": 1}"') &&

true
