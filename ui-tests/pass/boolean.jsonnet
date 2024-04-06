std.assertEqual(true && true, true) &&
std.assertEqual(true && false, false) &&
std.assertEqual(false && true, false) &&
std.assertEqual(false && false, false) &&

std.assertEqual(true || true, true) &&
std.assertEqual(true || false, true) &&
std.assertEqual(false || true, true) &&
std.assertEqual(false || false, false) &&

std.assertEqual(false && error "err", false) &&
std.assertEqual(true || error "err", true) &&

std.assertEqual(!true, false) &&
std.assertEqual(!false, true) &&

true
