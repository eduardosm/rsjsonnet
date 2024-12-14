//@args: --max-trace 5
//@exit-code: 0

local f(x) =
  if x == 0 then
    std.trace("some trace", true)
  else f(x - 1);

f(10)
