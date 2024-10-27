std.assertEqual({ a: 1 }, { a: 1 }) &&
std.assertEqual({ "a": 1 }, { a: 1 }) &&
std.assertEqual({ @"a": 1 }, { a: 1 }) &&
std.assertEqual({ ["a"]: 1 }, { a: 1 }) &&
std.assertEqual({ [null]: 1 }, {}) &&
std.assertEqual({ ["a"]: 1, [null]: 2 }, { a: 1 }) &&
std.assertEqual({ ["a"]: 1, [null]: 2, ["c"]: 3}, { a: 1, c: 3 }) &&

std.assertEqual({ a: 1, b: 2 }.a, 1) &&
std.assertEqual({ a: 1, b: 2 }["a"], 1) &&
std.assertEqual({ a: 1, b: 2 }.b, 2) &&
std.assertEqual({ a: 1, b: 2 }["b"], 2) &&
std.assertEqual({ a: 1, b:: 2 }.b, 2) &&
std.assertEqual({ a: 1, b:: 2 }["b"], 2) &&
std.assertEqual({ a: 1, b: error "err" }.a, 1) &&
std.assertEqual({ a: 1, b: error "err" }["a"], 1) &&
std.assertEqual({ a: 1, b:: error "err" }, { a: 1 }) &&

std.assertEqual({ a: 1, b: self.a }, { a: 1, b: 1 }) &&
std.assertEqual({ a: 1, b: $.a }, { a: 1, b: 1 }) &&
std.assertEqual(
  { a: 1, b: self.a, c: $.a, sub: { a: 2, b: self.a, c: $.a } },
  { a: 1, b: 1, c: 1, sub: { a: 2, b: 2, c: 1 } },
) &&

std.assertEqual({ local x = 1, a: x }, { a: 1 }) &&
std.assertEqual({ local x = self.a, a: 1, b: x }, { a: 1, b: 1 }) &&
std.assertEqual({ local x = error "err" }, {}) &&

std.assertEqual({ assert true }, {}) &&
std.assertEqual({ assert true, a: 1 }, { a: 1 }) &&
std.assertEqual({ assert self.a, a: true }, { a: true }) &&
std.assertEqual({ assert x, local x = true, a: 1, b: x }, { a: 1, b: true }) &&

std.assertEqual("a" in { a: 1 }, true) &&
std.assertEqual("b" in { a: 1 }, false) &&

local obj = {} + {};
std.assertEqual(obj, {}) &&
std.assertEqual(std.objectFields(obj), []) &&
std.assertEqual(std.objectFieldsAll(obj), []) &&

std.assertEqual({} + { a: 1 }, { a: 1 }) &&
std.assertEqual({} + { a+: 1 }, { a: 1 }) &&
std.assertEqual({ a: 1 } + {}, { a: 1 }) &&
std.assertEqual({ a+: 1 } + {}, { a: 1 }) &&

std.assertEqual({ a: 1 } { b: 2 }, { a: 1, b: 2 }) &&
std.assertEqual({ a: 1 } + { b: 2 }, { a: 1, b: 2 }) &&
std.assertEqual({ a: 1, b: 2 } + { a: 3 }, { a: 3, b: 2 }) &&
std.assertEqual({ a: 1 } + { a: 2, b: 3 }, { a: 2, b: 3 }) &&

std.assertEqual({ a+: 1 }, { a: 1 }) &&
std.assertEqual({ a+: 1 } + { a: 20 }, { a: 20 }) &&
std.assertEqual({ a: 1 } + { a+: 20 }, { a: 21 }) &&
std.assertEqual({ a+: 1 } + { a+: 20 }, { a: 21 }) &&
std.assertEqual({ a: 1 } + { ["a"]+: 20 }, { a: 21 }) &&

std.assertEqual(({ a: 1 } + { a: 10 }) + ({ a+: 100 } + { a+: 1000 }), { a: 1110 }) &&
std.assertEqual(({ a: 1 } + { a+: 10 }) + ({ a+: 100 } + { a+: 1000 }), { a: 1111 }) &&

std.assertEqual({ a: self.b } + { b: 1 }, { a: 1, b: 1 }) &&
std.assertEqual({ a: self.b, b: 1 } + { b: 2 }, { a: 2, b: 2 }) &&
std.assertEqual(({ a: self.b, b: 1 } + { b: 2 }) + ({ b: 3 } + { b: 4 }), { a: 4, b: 4 }) &&

std.assertEqual({ local x = self.a, a: 1, b: x + 10 } + { a: 2 }, { a: 2, b: 12 }) &&

std.assertEqual({ assert self.a, a: false } + { a: true }, { a: true }) &&
std.assertEqual({ assert self.a, a: false } + { a: true, b: super.a }, { a: true, b: false }) &&

std.assertEqual(local obj = { a: 1 }; obj + obj, { a: 1 }) &&
std.assertEqual(local obj = { a: super.a };  { a: 1 } + (obj + obj), { a: 1 }) &&
std.assertEqual(local obj = { a: super.a + 10 };  { a: 1 } + (obj + obj), { a: 21 }) &&
std.assertEqual(
  local obj = { a: super.a, assert super.a == 1 }; { a: 1 } + (obj + obj),
  { a: 1 },
) &&

std.assertEqual({ a: 1 } + { a: 2, b: self.a, c: super.a }, { a: 2, b: 2, c: 1 }) &&
std.assertEqual({ a: 1 } + { a: 2, b: self["a"], c: super["a"] }, { a: 2, b: 2, c: 1 }) &&

std.assertEqual({ a: "a" in super }, { a: false }) &&
std.assertEqual({ a: "b" in super }, { a: false }) &&
std.assertEqual({ a: "b" in super, b: 2 }, { a: false, b: 2 }) &&
std.assertEqual({} + { a: "a" in super }, { a: false }) &&
std.assertEqual({} + { a: "b" in super }, { a: false }) &&
std.assertEqual({} + { a: "b" in super, b: 2 }, { a: false, b: 2 }) &&
std.assertEqual({ a: 1 } + { a: "a" in super }, { a: true }) &&
std.assertEqual({ a: 1 } + { b: "a" in super }, { a: 1, b: true }) &&
std.assertEqual({ a: 1 } + { a: 2, b: "a" in super }, { a: 2, b: true }) &&

std.assertEqual(
  { ["n_" + x]: "v_" + x for x in ["a", "b"] },
  { n_a: "v_a", n_b: "v_b" },
) &&
std.assertEqual(
  { [x[0]]: x[1] for x in [["a", 1], [null, 2], ["c", 3]] },
  { a: 1, c: 3 },
) &&
std.assertEqual(
  { local x2 = x, ["n_" + x]: "v_" + x2 for x in ["a", "b"] },
  { n_a: "v_a", n_b: "v_b" },
) &&
std.assertEqual(
  { local a = "a", [x]: x + "_" + a + b, local b = "b" for x in ["1", "2"] },
  { "1": "1_ab", "2": "2_ab" },
) &&
std.assertEqual(
  local pre = "n_"; { local pre = "v_", [pre + x]: pre + x for x in ["a", "b"] },
  { n_a: "v_a", n_b: "v_b" },
) &&

true
