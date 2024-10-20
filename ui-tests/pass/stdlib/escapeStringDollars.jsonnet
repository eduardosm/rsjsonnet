std.assertEqual(std.escapeStringDollars(""), "") &&
std.assertEqual(std.escapeStringDollars("string"), "string") &&
std.assertEqual(std.escapeStringDollars(" $ "), " $$ ") &&
std.assertEqual(std.escapeStringDollars("1$2$3"), "1$$2$$3") &&
std.assertEqual(std.escapeStringDollars("1$$2"), "1$$$$2") &&

std.assertEqual(std.escapeStringDollars({ a: "$" }), @'{"a": "$$"}') &&

true
