std.assertEqual(std.mergePatch({ a: [1, 2] }, { a: [3, null] }), { a: [3, null] }) &&
std.assertEqual(std.mergePatch({ a: 1 }, 1.5), 1.5) &&
std.assertEqual(std.mergePatch({ a: 1 }, { a: 2 }), { a: 2 }) &&
std.assertEqual(std.mergePatch({ a: 1 }, { a: [] }), { a: [] }) &&
std.assertEqual(std.mergePatch({ a: 1 }, { a: {} }), { a: {} }) &&
std.assertEqual(std.mergePatch({ a: 1 }, { a: null }), {}) &&
std.assertEqual(std.mergePatch({ a: { x: 1, y: 2 }, b: -1 }, { a: { x: 3 } }), { a: { x: 3, y: 2 }, b: -1 }) &&
std.assertEqual(std.mergePatch({ a: { x: 1, y: 2 }, b: -1 }, { a: { x: null, y: null } }), { a: {}, b: -1 }) &&
std.assertEqual(std.mergePatch({ a: null }, {}), { a: null }) &&
std.assertEqual(std.mergePatch({ a: null }, { a: null }), {}) &&
std.assertEqual(std.mergePatch({ a: null }, { b: null }), { a: null }) &&
std.assertEqual(std.mergePatch(false, true), true) &&
std.assertEqual(std.mergePatch(false, {}), {}) &&
std.assertEqual(std.mergePatch(false, { a: 1 }), { a: 1 }) &&
std.assertEqual(std.mergePatch(false, { a: null }), {}) &&

// Examples from RFC 7396 (https://datatracker.ietf.org/doc/html/rfc7396)
std.assertEqual(std.mergePatch({"a":"b"}, {"a":"c"}), {"a":"c"}) &&
std.assertEqual(std.mergePatch({"a":"b"}, {"b":"c"}), {"a":"b","b":"c"}) &&
std.assertEqual(std.mergePatch({"a":"b"}, {"a":null}), {}) &&
std.assertEqual(std.mergePatch({"a":"b","b":"c"}, {"a":null}), {"b":"c"}) &&
std.assertEqual(std.mergePatch({"a":["b"]}, {"a":"c"}), {"a":"c"}) &&
std.assertEqual(std.mergePatch({"a":"c"}, {"a":["b"]}), {"a":["b"]}) &&
std.assertEqual(std.mergePatch({"a":{"b":"c"}}, {"a":{"b":"d","c":null}}), {"a":{"b":"d"}}) &&
std.assertEqual(std.mergePatch({"a":[{"b":"c"}]}, {"a":[1]}), {"a":[1]}) &&
std.assertEqual(std.mergePatch(["a","b"], ["c","d"]), ["c","d"]) &&
std.assertEqual(std.mergePatch({"a":"b"}, ["c"]), ["c"]) &&
std.assertEqual(std.mergePatch({"a":"foo"}, null), null) &&
std.assertEqual(std.mergePatch({"a":"foo"}, "bar"), "bar") &&
std.assertEqual(std.mergePatch({"e":null}, {"a":1}), {"e":null,"a":1}) &&
std.assertEqual(std.mergePatch([1,2], {"a":"b","c":null}), {"a":"b"}) &&
std.assertEqual(std.mergePatch({}, {"a":{"bb":{"ccc":null}}}), {"a":{"bb":{}}}) &&

true
