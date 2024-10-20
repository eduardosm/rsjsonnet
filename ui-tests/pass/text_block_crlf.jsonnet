local value = |||
  first line
  second line
|||;

std.assertEqual(value, "first line\r\nsecond line\r\n")
