std.assertEqual(std.manifestIni({ sections: {} }), "") &&
std.assertEqual(std.manifestIni({ main: {}, sections: {} }), "") &&

std.assertEqual(std.manifestIni(
  {
    main: {
      a: null,
      b: true,
      c: false,
      d: 1.5,
      e: "string",
      f: [],
      g: [1, 2],
      h: { a: 1 },
    },
    sections: {},
  }),
  |||
    a = null
    b = true
    c = false
    d = 1.5
    e = string
    g = 1
    g = 2
    h = {"a": 1}
  |||,
) &&

std.assertEqual(std.manifestIni(
  {
    sections: {
      sec1: {
        a: 1,
        b: 2,
      },
      sec2: {
        c: 3,
        d: 4,
      },
    },
  }),
  |||
    [sec1]
    a = 1
    b = 2
    [sec2]
    c = 3
    d = 4
  |||,
) &&

std.assertEqual(std.manifestIni(
  {
    sections: {
      sec1: {},
      sec2: {},
    },
  }),
  |||
    [sec1]
    [sec2]
  |||,
) &&

std.assertEqual(std.manifestIni(
  {
    main: {
      m1: "x",
      m2: "y",
    },
    sections: {
      sec1: {
        a: 1,
        b: 2,
      },
      sec2: {
        c: 3,
        d: 4,
      },
    },
  }),
  |||
    m1 = x
    m2 = y
    [sec1]
    a = 1
    b = 2
    [sec2]
    c = 3
    d = 4
  |||,
) &&

true
