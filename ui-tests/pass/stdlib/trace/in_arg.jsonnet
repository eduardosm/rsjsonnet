//@exit-code: 0

local f(x) = x;
local f1() = f(std.trace("some trace", "some value"));
local f2() = f(std.trace("some trace", "some value")) tailstrict;

[f1(), f2()]
