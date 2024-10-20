std.assertEqual(std.escapeStringXML(""), "") &&
std.assertEqual(std.escapeStringXML("string"), "string") &&
std.assertEqual(std.escapeStringXML(" ' \" < > & "), " &apos; &quot; &lt; &gt; &amp; ") &&

std.assertEqual(std.escapeStringXML({ a: "&" }), '{&quot;a&quot;: &quot;&amp;&quot;}') &&

true
