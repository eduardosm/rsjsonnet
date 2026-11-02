// Overlong UTF-8 sequences are invalid and replaced with U+FFFD

std.assertEqual("¡Å", "\uFFFD\uFFFD") &&
std.assertEqual(@"¿Ä", "\uFFFD\uFFFD") &&
std.assertEqual(|||
  ¿Ø
|||, "\uFFFD\uFFFD\n") &&

true
