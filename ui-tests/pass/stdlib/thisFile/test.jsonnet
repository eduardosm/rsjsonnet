std.assertEqual(std.thisFile, "test.jsonnet") &&
std.assertEqual(import "imports/a.libjsonnet", "imports/a.libjsonnet") &&
std.assertEqual(import "imports/b.libjsonnet", "imports/a.libjsonnet") &&

true
