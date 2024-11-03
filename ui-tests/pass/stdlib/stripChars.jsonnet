local test(str, chars, l, r) =
  std.assertEqual(std.lstripChars(str, chars), str[l:]) &&
  std.assertEqual(std.rstripChars(str, chars), str[:std.length(str) - r]) &&
  std.assertEqual(
    std.stripChars(str, chars),
    if l + r < std.length(str) then str[l:std.length(str) - r] else ""
  );

test("", "", 0, 0) &&
test("", " \t", 0, 0) &&
test("  \t  \t ", "", 0, 0) &&
test("  \t  \t ", " ", 2, 1) &&
test("  \t  \t ", "\t", 0, 0) &&
test("  \t  \t ", " \t", 7, 7) &&
test("  \tX\t ", " \t", 3, 2) &&
test("\t  X \t", " \t", 3, 2) &&
test("  \tX\t ", "X", 0, 0) &&
test("  \tX\t ", "Y", 0, 0) &&
test("💯👍💯💯", "👍", 0, 0) &&
test("💯👍💯💯", "💯", 1, 2) &&

true
