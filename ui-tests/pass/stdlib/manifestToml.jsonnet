std.assertEqual(std.manifestToml({}), "") &&

std.assertEqual(
  std.manifestToml({
    b: true,
    c: false,
    d: 1.5,
    e: "string",
    f: [],
    g: {},
    h: {},
  }) + "\n",
  |||
    b = true
    c = false
    d = 1.5
    e = "string"
    f = []

    [g]

    [h]
  |||,
) &&

std.assertEqual(
  std.manifestToml({
    array: [1, 2, 3, { a: 4, b: 5, c: [6, 7] }],
    object1: {
      field1: "a",
      field2: "b",
    },
    object2: {
      value: 1,
      array: [
        { a: 1, b: 2 },
        { a: 3, b: 4 },
      ]
    }
  }) + "\n",
  |||
    array = [
      1,
      2,
      3,
      { a = 4, b = 5, c = [ 6, 7 ] }
    ]

    [object1]
      field1 = "a"
      field2 = "b"

    [object2]
      value = 1

      [[object2.array]]
        a = 1
        b = 2

      [[object2.array]]
        a = 3
        b = 4
  |||,
) &&

std.assertEqual(
  std.manifestTomlEx({
    array: [1, 2, 3, { a: 4, b: 5, c: [6, 7] }],
    object1: {
      field1: "a",
      field2: "b",
    },
    object2: {
      value: 1,
      array: [
        { a: 1, b: 2 },
        { a: 3, b: 4 },
      ]
    }
  }, "    ") + "\n",
  |||
    array = [
        1,
        2,
        3,
        { a = 4, b = 5, c = [ 6, 7 ] }
    ]

    [object1]
        field1 = "a"
        field2 = "b"

    [object2]
        value = 1

        [[object2.array]]
            a = 1
            b = 2

        [[object2.array]]
            a = 3
            b = 4
  |||,
) &&

std.assertEqual(
  std.manifestToml({
    "some\"key": "some\"string",
    "some\"object": {
      "some\"array": [
        { a: 1, b: 2 },
        { a: 3, b: 4 },
      ]
    }
  }) + "\n",
  |||
    "some\"key" = "some\"string"

    ["some\"object"]


      [["some\"object"."some\"array"]]
        a = 1
        b = 2

      [["some\"object"."some\"array"]]
        a = 3
        b = 4
  |||,
) &&

true
