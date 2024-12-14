//@args: --tla-str arg1="string arg" --tla-code arg2="{ a: 0.5 } { a+: 0.5, b: 2 }"

local f(arg1, arg2) =
  std.assertEqual(arg1, "string arg") &&
  std.assertEqual(arg2, { a: 1, b: 2 }) &&
  true;

f
