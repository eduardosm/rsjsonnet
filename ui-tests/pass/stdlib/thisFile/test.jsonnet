std.assertEqual(std.thisFile, "test.jsonnet") &&
std.assertEqual(import "imports/a.libsonnet", "imports/a.libsonnet") &&
std.assertEqual(import "imports/b.libsonnet", "imports/a.libsonnet") &&

true
