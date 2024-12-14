std.assertEqual(std.manifestJson(null), "null") &&
std.assertEqual(std.manifestJson(true), "true") &&
std.assertEqual(std.manifestJson(false), "false") &&
std.assertEqual(std.manifestJson(0), "0") &&
std.assertEqual(std.manifestJson(-0), "-0") &&
std.assertEqual(std.manifestJson(1.5), "1.5") &&
std.assertEqual(std.manifestJson(-1.5), "-1.5") &&
std.assertEqual(std.manifestJson("string"), '"string"') &&
std.assertEqual(std.manifestJson("\n"), @'"\n"') &&

std.assertEqual(
  std.manifestJson([]) + "\n",
  |||
    [

    ]
  |||,
) &&

std.assertEqual(
  std.manifestJson([1]) + "\n",
  |||
    [
        1
    ]
  |||,
) &&

std.assertEqual(
  std.manifestJson([1, 2]) + "\n",
  |||
    [
        1,
        2
    ]
  |||,
) &&

std.assertEqual(
  std.manifestJson([[]]) + "\n",
  |||
    [
        [

        ]
    ]
  |||,
) &&

std.assertEqual(
  std.manifestJson([[1, 2]]) + "\n",
  |||
    [
        [
            1,
            2
        ]
    ]
  |||,
) &&

std.assertEqual(
  std.manifestJson({}) + "\n",
  |||
    {

    }
  |||,
) &&

std.assertEqual(
  std.manifestJson({ a: 1 }) + "\n",
  |||
    {
        "a": 1
    }
  |||,
) &&

std.assertEqual(
  std.manifestJson({ a: 1, b: 2 }) + "\n",
  |||
    {
        "a": 1,
        "b": 2
    }
  |||,
) &&

std.assertEqual(
  std.manifestJson({ a: {} }) + "\n",
  |||
    {
        "a": {

        }
    }
  |||,
) &&

std.assertEqual(
  std.manifestJson({ a: { b: 2, c: 3 } }) + "\n",
  |||
    {
        "a": {
            "b": 2,
            "c": 3
        }
    }
  |||,
) &&

std.assertEqual(std.manifestJsonMinified(null), "null") &&
std.assertEqual(std.manifestJsonMinified(true), "true") &&
std.assertEqual(std.manifestJsonMinified(false), "false") &&
std.assertEqual(std.manifestJsonMinified(0), "0") &&
std.assertEqual(std.manifestJsonMinified(-0), "-0") &&
std.assertEqual(std.manifestJsonMinified(1.5), "1.5") &&
std.assertEqual(std.manifestJsonMinified(-1.5), "-1.5") &&
std.assertEqual(std.manifestJsonMinified("string"), '"string"') &&
std.assertEqual(std.manifestJsonMinified("\n"), @'"\n"') &&
std.assertEqual(std.manifestJsonMinified([]), "[]") &&
std.assertEqual(std.manifestJsonMinified([1]), "[1]") &&
std.assertEqual(std.manifestJsonMinified([1, 2]), "[1,2]") &&
std.assertEqual(std.manifestJsonMinified([[]]), "[[]]") &&
std.assertEqual(std.manifestJsonMinified([[1, 2]]), "[[1,2]]") &&
std.assertEqual(std.manifestJsonMinified({}), "{}") &&
std.assertEqual(std.manifestJsonMinified({ a: 1 }), '{"a":1}') &&
std.assertEqual(std.manifestJsonMinified({ a: 1, b: 2 }), '{"a":1,"b":2}') &&
std.assertEqual(std.manifestJsonMinified({ a: {} }), '{"a":{}}') &&
std.assertEqual(std.manifestJsonMinified({ a: { b: 2, c: 3 } }), '{"a":{"b":2,"c":3}}') &&

std.assertEqual(
  std.manifestJsonEx({ a: 1, b: [1, 2] }, "\t", "\r", " :"),
  '{\r\t"a" :1,\r\t"b" :[\r\t\t1,\r\t\t2\r\t]\r}',
) &&

true
