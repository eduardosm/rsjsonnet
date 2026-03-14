std.assertEqual(std.manifestIni({ sections: {} }), "") &&
std.assertEqual(std.manifestIni({ main: {}, sections: {} }), "") &&

std.assertEqual(std.manifestIni(
  {
    main: {
      nu: null,
      b1: true,
      b2: false,
      nr: 1.5,
      s1: "",
      s2: "string",
      a1: [],
      a2: [1, 2],
      o1: {},
      o2: { a: 1 },
    },
    sections: {},
  }),
  |||
    a2 = 1
    a2 = 2
    b1 = true
    b2 = false
    nr = 1.5
    nu = null
    o1 = { }
    o2 = {"a": 1}
    s1 = 
    s2 = string
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
      sec1: {
        a: 1,
        b: 2,
      },
      sec2: {
        c: 3,
        d: 4,
      },
      sec3: {},
    },
  }),
  |||
    [sec1]
    a = 1
    b = 2
    [sec2]
    c = 3
    d = 4
    [sec3]
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
      sec3: {},
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
    [sec3]
  |||,
) &&

std.assertEqual(std.manifestIni(
  {
    main: {
      m1: "x",
      m2: "y",
    },
    sections: {
      sec1: {},
      sec2: {},
    },
  }),
  |||
    m1 = x
    m2 = y
    [sec1]
    [sec2]
  |||,
) &&

true
