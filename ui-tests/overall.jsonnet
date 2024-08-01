# This is meant to be a test to check if a Jsonnet interpreter works
# overall, without testing every corner case of every feature.

local importedFunc = import "pass/import/lib/func.libjsonnet";

local a = 1, b = 2;

{
  binOps: {
    add: {
      number: [
        1.5 + 2.25,
        3 + 4,
      ],
      string: [
        "a" + "b",
        "c" + "d",
      ],
    },
    and: {
      bool: [
        false && false,
        false && true,
        true && false,
        true && true,
      ],
      integer: [
        0 & 1,
        3 & 6,
      ],
    },
  },

  unOps: {
    not: [
      !false,
      !true,
    ],
    neg: [
      -1,
      -2,
    ],
  },

  importedFunc: [
    importedFunc(2, 3),
    importedFunc(4, 5)
  ],

  "if": [
    if a == b then 55 else 33,
    if a != b then 77 else 88,
  ],

  importStr: importstr "pass/import/lib/utf8.txt",
  importBin: importbin "pass/import/lib/non_utf8.bin",

  arrayComp: [x + y for x in [1, 2, 3] for y in [x * 10, 40, 50, 60] if x != 2],
  objectComp: { ["x=" + x]: x * 10 for x in [1, 2, 3] if x != 2 },

  std: {
    object: std,

    all: [
      std.all([true, true]),
      std.all([true, false]),
      std.all([false, false]),
    ],
    any: [
      std.any([true, true]),
      std.any([true, false]),
      std.any([false, false]),
    ],

    asciiLower: std.asciiLower("1 aBcDeFgHiJkLmNoPqRsTuVwXyZ 2"),
    asciiUpper: std.asciiUpper("1 aBcDeFgHiJkLmNoPqRsTuVwXyZ 2"),

    char: std.char(128175),
    codepoint: std.codepoint("🍏"),

    decodeUTF8: std.decodeUTF8([32, 240, 159, 143, 128, 32]),
    encodeUTF8: std.encodeUTF8(" 🧶 "),

    stringChars: std.stringChars("👀🐢💤🛷"),

    startsWith: [
      std.startsWith("abc", "ab"),
      std.startsWith("abc", "ba"),
    ],
    endsWith: [
      std.endsWith("abc", "bc"),
      std.endsWith("abc", "cb"),
    ],

    format: std.format("a = %.3f, b = %.4f", [1.25, 2.5]),

    foldl: std.foldl(function(x, y) [x, y], [1, 2], 'x'),
    foldr: std.foldr(function(x, y) [x, y], [1, 2], 'x'),

    sort: std.sort([3, 1, 2, 1]),

    set: std.set([1, 2, 3, 2, 1]),

    md5: std.md5("some string"),

    parseJson: std.parseJson(|||
      {
        "a": 1,
        "b": [2, 3, 4]
      }
    |||),
    parseYaml: std.parseYaml(|||
      a: 1
      b:
        - 2
        - 3
        - 4
    |||),
  }
}
