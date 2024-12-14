//@args: --ext-str str_var="string variable" --ext-code code_var="{ a: 0.5 } { a+: 0.5, b: 2 }"

std.assertEqual(std.extVar("str_var"), "string variable") &&
std.assertEqual(std.extVar("code_var"), { a: 1, b: 2 }) &&

true
