local test(decoded, encoded) =
  local decodedBytes = std.makeArray(std.length(decoded), function(i) std.codepoint(decoded[i]));
  std.assertEqual(std.base64(decoded), encoded) &&
  std.assertEqual(std.base64(decodedBytes), encoded) &&
  std.assertEqual(std.base64DecodeBytes(encoded), decodedBytes) &&
  std.assertEqual(std.base64Decode(encoded), decoded);

test("", "") &&
test("A", "QQ==") &&
test("AB", "QUI=") &&
test("ABC", "QUJD") &&
test("ABCD", "QUJDRA==") &&
test("ABCDE", "QUJDREU=") &&
test("ABCDEF", "QUJDREVG") &&
test("ABCDEFG", "QUJDREVGRw==") &&

test("\u0000", "AA==") &&
test("\u0001", "AQ==") &&
test("\u007F", "fw==") &&
test("\u00FA", "+g==") &&
test("\u00FF", "/w==") &&

test("one, two; 3", "b25lLCB0d287IDM=") &&

true
