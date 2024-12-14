std.assertEqual(std.escapeStringBash(""), "''") &&
std.assertEqual(std.escapeStringBash("string"), "'string'") &&
std.assertEqual(std.escapeStringBash("\""), "'\"'") &&
std.assertEqual(std.escapeStringBash("a\"b"), "'a\"b'") &&
std.assertEqual(std.escapeStringBash("a'b"), "'a'\"'\"'b'") &&

std.assertEqual(std.escapeStringBash({ a: "'" }), '\'{"a": "\'"\'"\'"}\'') &&

true
