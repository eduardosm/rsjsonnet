std.assertEqual(std.type(null), "null") &&
std.assertEqual(std.type(true), "boolean") &&
std.assertEqual(std.type(false), "boolean") &&
std.assertEqual(std.type(0), "number") &&
std.assertEqual(std.type("str"), "string") &&
std.assertEqual(std.type([]), "array") &&
std.assertEqual(std.type([1, 2]), "array") &&
std.assertEqual(std.type({}), "object") &&
std.assertEqual(std.type({ a: 1, b: 2 }), "object") &&
std.assertEqual(std.type(function() 0), "function") &&
std.assertEqual(std.type(std.length), "function") &&

std.assertEqual(std.isNumber(null), false) &&
std.assertEqual(std.isNumber(true), false) &&
std.assertEqual(std.isNumber(0), true) &&
std.assertEqual(std.isNumber("str"), false) &&
std.assertEqual(std.isNumber([1, 2]), false) &&
std.assertEqual(std.isNumber({ a: 1, b: 2 }), false) &&
std.assertEqual(std.isNumber(function() 0), false) &&
std.assertEqual(std.isNumber(std.length), false) &&

std.assertEqual(std.isString(null), false) &&
std.assertEqual(std.isString(true), false) &&
std.assertEqual(std.isString(0), false) &&
std.assertEqual(std.isString("str"), true) &&
std.assertEqual(std.isString([1, 2]), false) &&
std.assertEqual(std.isString({ a: 1, b: 2 }), false) &&
std.assertEqual(std.isString(function() 0), false) &&
std.assertEqual(std.isString(std.length), false) &&

std.assertEqual(std.isArray(null), false) &&
std.assertEqual(std.isArray(true), false) &&
std.assertEqual(std.isArray(0), false) &&
std.assertEqual(std.isArray("str"), false) &&
std.assertEqual(std.isArray([1, 2]), true) &&
std.assertEqual(std.isArray({ a: 1, b: 2 }), false) &&
std.assertEqual(std.isArray(function() 0), false) &&
std.assertEqual(std.isArray(std.length), false) &&

std.assertEqual(std.isObject(null), false) &&
std.assertEqual(std.isObject(true), false) &&
std.assertEqual(std.isObject(0), false) &&
std.assertEqual(std.isObject("str"), false) &&
std.assertEqual(std.isObject([1, 2]), false) &&
std.assertEqual(std.isObject({ a: 1, b: 2 }), true) &&
std.assertEqual(std.isObject(function() 0), false) &&
std.assertEqual(std.isObject(std.length), false) &&

std.assertEqual(std.isFunction(null), false) &&
std.assertEqual(std.isFunction(true), false) &&
std.assertEqual(std.isFunction(0), false) &&
std.assertEqual(std.isFunction("str"), false) &&
std.assertEqual(std.isFunction([1, 2]), false) &&
std.assertEqual(std.isFunction({ a: 1, b: 2 }), false) &&
std.assertEqual(std.isFunction(function() 0), true) &&
std.assertEqual(std.isFunction(std.length), true) &&

// Object asserts are not checked
std.assertEqual(std.type({ assert false, a: 1 }), "object") &&
std.assertEqual(std.isObject({ assert false, a: 1 }), true) &&

true
