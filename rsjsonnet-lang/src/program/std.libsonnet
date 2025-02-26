// This is the part of the standard library that is written in Jsonnet.

{
  abs(n)::
    assert self.isNumber(n) : "first argument of `std.abs` is expected to be number, got " + self.type(n);
    if n > 0 then n else -n,

  sign(n)::
    assert self.isNumber(n) : "first argument of `std.sign` is expected to be number, got " + self.type(n);
    if n > 0 then
      1
    else if n < 0 then
      -1
    else 0,

  max(a, b)::
    assert self.isNumber(a) : "first argument of `std.max` is expected to be number, got " + self.type(a);
    assert self.isNumber(b) : "second argument of `std.max` is expected to be number, got " + self.type(b);
    if a > b then a else b,

  min(a, b)::
    assert self.isNumber(a) : "first argument of `std.min` is expected to be number, got " + self.type(a);
    assert self.isNumber(b) : "second argument of `std.min` is expected to be number, got " + self.type(b);
    if a < b then a else b,

  clamp(x, minVal, maxVal)::
    assert self.isNumber(x) : "first argument of `std.clamp` is expected to be number, got " + self.type(x);
    assert self.isNumber(minVal) : "second argument of `std.clamp` is expected to be number, got " + self.type(minVal);
    assert self.isNumber(maxVal) : "third argument of `std.clamp` is expected to be number, got " + self.type(maxVal);
    if x < minVal then
      minVal
    else if x > maxVal then
      maxVal
    else
      x,

  round(x):: self.floor(x + 0.5),

  isEmpty(str):: self.length(str) == 0,

  manifestToml(value):: self.manifestTomlEx(value, "  "),

  manifestJson(value):: self.manifestJsonEx(value, "    "),

  manifestJsonMinified(value):: self.manifestJsonEx(value, "", "", ":"),

  lines(arr):: self.join("\n", arr + [""]),

  get(o, f, default=null, inc_hidden=true):: if self.objectHasEx(o, f, inc_hidden) then o[f] else default,

  objectHas(o, f):: self.objectHasEx(o, f, false),
  objectHasAll(o, f):: self.objectHasEx(o, f, true),

  objectFields(o):: self.objectFieldsEx(o, false),
  objectFieldsAll(o):: self.objectFieldsEx(o, true),

  objectValues(o):: [o[key] for key in self.objectFields(o)],
  objectValuesAll(o):: [o[key] for key in self.objectFieldsAll(o)],

  objectKeysValues(o):: [{ key: key, value: o[key] } for key in self.objectFields(o)],
  objectKeysValuesAll(o):: [{ key: key, value: o[key] } for key in self.objectFieldsAll(o)],

  xor(x, y):: x != y,
  xnor(x, y):: x == y,

  resolvePath(f, r)::
    local parts = self.split(f, "/");
    self.join("/", parts[:self.length(parts) - 1] + [r]),

  __array_less(arr1, arr2):: self.__compare_array(arr1, arr2) < 0,
  __array_less_or_equal(arr1, arr2):: self.__compare_array(arr1, arr2) <= 0,
  __array_greater(arr1, arr2):: self.__compare_array(arr1, arr2) > 0,
  __array_greater_or_equal(arr1, arr2):: self.__compare_array(arr1, arr2) >= 0,
}
