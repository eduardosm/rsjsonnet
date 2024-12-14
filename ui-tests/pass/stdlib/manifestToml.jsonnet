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
    obj: {
      a: 1,
      b: 2,
    },
  }) + "\n",
  |||


    [obj]
      a = 1
      b = 2
  |||,
) &&

std.assertEqual(
  std.manifestToml({
    array1: [1, 2, 3, [4, 5], { a: 6, b: 7, c: [8, 9], d: {} }],
    array2: [{}, { x: "y", y: "x" }],
    object1: {
      field1: "a",
      field2: "b",
    },
    object2: {
      value: 1,
      array: [
        { a: 1, b: 2 },
        { object3: { a: 3, b: 4 } },
      ]
    }
  }) + "\n",
  |||
    array1 = [
      1,
      2,
      3,
      [ 4, 5 ],
      { a = 6, b = 7, c = [ 8, 9 ], d = {  } }
    ]

    [[array2]]

    [[array2]]
      x = "y"
      y = "x"

    [object1]
      field1 = "a"
      field2 = "b"

    [object2]
      value = 1

      [[object2.array]]
        a = 1
        b = 2

      [[object2.array]]


        [object2.array.object3]
          a = 3
          b = 4
  |||,
) &&

std.assertEqual(
  std.manifestTomlEx({
    array1: [1, 2, 3, [4, 5], { a: 6, b: 7, c: [8, 9], d: {} }],
    array2: [{}, { x: "y", y: "x" }],
    object1: {
      field1: "a",
      field2: "b",
    },
    object2: {
      value: 1,
      array: [
        { a: 1, b: 2 },
        { object3: { a: 3, b: 4 } },
      ]
    }
  }, "    ") + "\n",
  |||
    array1 = [
        1,
        2,
        3,
        [ 4, 5 ],
        { a = 6, b = 7, c = [ 8, 9 ], d = {  } }
    ]

    [[array2]]

    [[array2]]
        x = "y"
        y = "x"

    [object1]
        field1 = "a"
        field2 = "b"

    [object2]
        value = 1

        [[object2.array]]
            a = 1
            b = 2

        [[object2.array]]


            [object2.array.object3]
                a = 3
                b = 4
  |||,
) &&

std.assertEqual(
  std.manifestToml({
    "": "empty key",
    "some\"key": "some\"string",
    "some\"object": {
      "some\"array": [
        { a: 1, b: 2 },
        { a: 3, b: 4 },
      ]
    }
  }) + "\n",
  |||
    "" = "empty key"
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

std.assertEqual(
  std.manifestToml({
    value: 0,
    object1: {
      value: 1,
      array1: [
        {
            a: 1,
            object2: {
                value: 2,
                array2: [
                    { a: "x" },
                    { a: "y" },
                ]
            }
        },
        { a: 2 },
      ]
    }
  }) + "\n",
  |||
    value = 0

    [object1]
      value = 1

      [[object1.array1]]
        a = 1

        [object1.array1.object2]
          value = 2

          [[object1.array1.object2.array2]]
            a = "x"

          [[object1.array1.object2.array2]]
            a = "y"

      [[object1.array1]]
        a = 2
  |||,
) &&

true
