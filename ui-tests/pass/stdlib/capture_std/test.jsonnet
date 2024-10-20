local mystd = import "imports/mystd.libsonnet";
local mystd_func = import "imports/mystd_func.libsonnet";

std.assertEqual(std.thisFile, "test.jsonnet") &&
std.assertEqual(mystd.thisFile, "imports/mystd.libsonnet") &&
std.assertEqual(mystd_func("").thisFile, "imports/mystd_func.libsonnet") &&

std.assertEqual(std.type(true), "boolean") &&
std.assertEqual(mystd.type(true), "BOOLEAN") &&
std.assertEqual(mystd_func(" 1").type(true), "BOOLEAN 1") &&
std.assertEqual(mystd_func(" 2").type(true), "BOOLEAN 2") &&

true
