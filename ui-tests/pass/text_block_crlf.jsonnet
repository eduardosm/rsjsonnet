local value1 = |||
  first line
  second line
|||;

local value2 = |||-
  first line
  second line
|||;

std.assertEqual(value1, "first line\r\nsecond line\r\n") &&
std.assertEqual(value2, "first line\r\nsecond line\r")
