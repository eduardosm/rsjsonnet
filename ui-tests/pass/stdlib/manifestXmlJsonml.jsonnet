std.assertEqual(
  std.manifestXmlJsonml(["tag"]),
  "<tag></tag>",
) &&
std.assertEqual(
  std.manifestXmlJsonml(["tag", "a", "b"]),
  "<tag>ab</tag>",
) &&
std.assertEqual(
  std.manifestXmlJsonml(["a", ["b", ["c"]]]),
  '<a><b><c></c></b></a>',
) &&
std.assertEqual(
  std.manifestXmlJsonml(["tag", { a: null, b: true, c: false, d: 1.5, e: "string" }]),
  '<tag a="null" b="true" c="false" d="1.5" e="string"></tag>',
) &&
std.assertEqual(
  std.manifestXmlJsonml(["tag", { x: 1 }, "a", "b"]),
  '<tag x="1">ab</tag>',
) &&
std.assertEqual(
  std.manifestXmlJsonml(["tag", { x: 1 }, "a ", ["inner", "something"], " "]),
  '<tag x="1">a <inner>something</inner> </tag>',
) &&
std.assertEqual(
  std.manifestXmlJsonml(["tag", { x: 1 }, "a ", ["inner", { y: 2 }, "something"], " "]),
  '<tag x="1">a <inner y="2">something</inner> </tag>',
) &&

true
