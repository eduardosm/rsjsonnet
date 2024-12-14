//@exit-code: 0

local f1(x) = std.trace("trace 1", x);
local f2 = [ function(x) std.trace("trace 2", x) ];
local obj = { local f3(x) = std.trace("trace 3", x), field: f3(3) };

[
    f1(1),
    f2[0](2),
    obj.field,
]
