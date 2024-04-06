local f() = 0;
std.assertEqual(f(), 0) &&

local f(x) = [x];
std.assertEqual(f(1), [1]) &&
std.assertEqual(f(x=1), [1]) &&

local f(x, y) = [x, y];
std.assertEqual(f(1, 2), [1, 2]) &&
std.assertEqual(f(1, y=2), [1, 2]) &&
std.assertEqual(f(x=1, y=2), [1, 2]) &&
std.assertEqual(f(y=1, x=2), [2, 1]) &&

local f(x=1, y) = [x, y];
std.assertEqual(f(2, 3), [2, 3]) &&
std.assertEqual(f(2, y=3), [2, 3]) &&
std.assertEqual(f(x=2, y=3), [2, 3]) &&
std.assertEqual(f(y=2, x=3), [3, 2]) &&
std.assertEqual(f(y=2), [1, 2]) &&

local f(x, y=2) = [x, y];
std.assertEqual(f(3, 4), [3, 4]) &&
std.assertEqual(f(3, y=4), [3, 4]) &&
std.assertEqual(f(1), [1, 2]) &&
std.assertEqual(f(x=1), [1, 2]) &&
std.assertEqual(f(x=1, y=2), [1, 2]) &&
std.assertEqual(f(y=1, x=2), [2, 1]) &&

local f(x, y=x) = [x, y];
std.assertEqual(f(1, 2), [1, 2]) &&
std.assertEqual(f(1, y=2), [1, 2]) &&
std.assertEqual(f(1), [1, 1]) &&
std.assertEqual(f(x=1), [1, 1]) &&
std.assertEqual(f(x=1, y=2), [1, 2]) &&
std.assertEqual(f(y=1, x=2), [2, 1]) &&

local f(x=y, y) = [x, y];
std.assertEqual(f(1, 2), [1, 2]) &&
std.assertEqual(f(1, y=2), [1, 2]) &&
std.assertEqual(f(x=1, y=2), [1, 2]) &&
std.assertEqual(f(y=1, x=2), [2, 1]) &&
std.assertEqual(f(y=1), [1, 1]) &&

local f(x=y, y=1) = [x, y];
std.assertEqual(f(), [1, 1]) &&
std.assertEqual(f(2), [2, 1]) &&
std.assertEqual(f(2, 3), [2, 3]) &&
std.assertEqual(f(y=2), [2, 2]) &&
std.assertEqual(f(x=2, y=3), [2, 3]) &&
std.assertEqual(f(y=2, x=3), [3, 2]) &&

local f(x=[1, y[0]], y=[2, x[0]]) = [x, y];
std.assertEqual(f(), [[1, 2], [2, 1]]) &&
std.assertEqual(f(x=[3, 4]), [[3, 4], [2, 3]]) &&
std.assertEqual(f(y=[3, 4]), [[1, 3], [3, 4]]) &&
std.assertEqual(f(x=[3, 4], y=[5, 6]), [[3, 4], [5, 6]]) &&

std.assertEqual(local f = (local a = 1; function() [a]); local a = 2; f(), [1]) &&
std.assertEqual(local f = (local a = 1; function(x=a) [x]); local a = 2; f(), [1]) &&
std.assertEqual(local f = (local a = 1; function(x=a) [x]); local a = 2; f(a), [2]) &&

std.assertEqual(local a = 1; local f = function() [a]; local a = 2; f(), [1]) &&
std.assertEqual(local a = 1; local f = function(x=a) [x]; local a = 2; f(), [1]) &&
std.assertEqual(local a = 1; local f = function(x=a) [x]; local a = 2; f(a), [2]) &&

std.assertEqual(local a = 1, f = function() [a]; local a = 2; f(), [1]) &&
std.assertEqual(local a = 1, f = function(x=a) [x]; local a = 2; f(), [1]) &&
std.assertEqual(local a = 1, f = function(x=a) [x]; local a = 2; f(a), [2]) &&

# Arguments are evaluated lazily
local f(x, y) = [x, x];
std.assertEqual(f(1, error "err"), [1, 1]) &&

local f(x, y=error "err") = [x, x];
std.assertEqual(f(1), [1, 1]) &&

true
