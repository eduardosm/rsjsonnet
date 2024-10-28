std.assertEqual(std.format("", []), "") &&
std.assertEqual(std.format("%%", []), "%") &&

std.assertEqual(std.format("%s", "string"), "string") &&
std.assertEqual(std.format("%s", [[1, 2, 3]]), "[1, 2, 3]") &&

std.assertEqual(std.format("%i", [0]), "0") &&
std.assertEqual(std.format("%i", [-0]), "0") &&
std.assertEqual(std.format("%i", [31]), "31") &&
std.assertEqual(std.format("%d", [31]), "31") &&
std.assertEqual(std.format("%u", [31]), "31") &&
std.assertEqual(std.format("%i", [-31]), "-31") &&
std.assertEqual(std.format("%+i", [31]), "+31") &&
std.assertEqual(std.format("% i", [31]), " 31") &&
std.assertEqual(std.format("%i", [0.7]), "0") &&
std.assertEqual(std.format("%i", [-0.7]), "0") &&
std.assertEqual(std.format("%4i", [31]), "  31") &&
std.assertEqual(std.format("%1i", [-31]), "-31") &&
std.assertEqual(std.format("%1i", [31]), "31") &&
std.assertEqual(std.format("%4i", [-31]), " -31") &&
std.assertEqual(std.format("%01i", [31]), "31") &&
std.assertEqual(std.format("%01i", [-31]), "-31") &&
std.assertEqual(std.format("%04i", [31]), "0031") &&
std.assertEqual(std.format("%04i", [-31]), "-031") &&
std.assertEqual(std.format("%0*i", [1, 31]), "31") &&
std.assertEqual(std.format("%0*i", [1, -31]), "-31") &&
std.assertEqual(std.format("%0*i", [4, 31]), "0031") &&
std.assertEqual(std.format("%0*i", [4, -31]), "-031") &&
std.assertEqual(std.format("%.1i", [31]), "31") &&
std.assertEqual(std.format("%.1i", [-31]), "-31") &&
std.assertEqual(std.format("%.4i", [31]), "0031") &&
std.assertEqual(std.format("%.4i", [-31]), "-0031") &&
std.assertEqual(std.format("%.*i", [1, 31]), "31") &&
std.assertEqual(std.format("%.*i", [1, -31]), "-31") &&
std.assertEqual(std.format("%.*i", [4, 31]), "0031") &&
std.assertEqual(std.format("%.*i", [4, -31]), "-0031") &&
std.assertEqual(std.format("%05.4i", [31]), "00031") &&
std.assertEqual(std.format("%05.4i", [-31]), "-0031") &&
std.assertEqual(std.format("%04.5i", [31]), "00031") &&
std.assertEqual(std.format("%04.5i", [-31]), "-00031") &&
std.assertEqual(std.format("%0*.*i", [5, 4, 31]), "00031") &&
std.assertEqual(std.format("%0*.*i", [5, 4, -31]), "-0031") &&
std.assertEqual(std.format("%0*.*i", [4, 5, 31]), "00031") &&
std.assertEqual(std.format("%0*.*i", [4, 5, -31]), "-00031") &&

std.assertEqual(std.format("%o", [0]), "0") &&
std.assertEqual(std.format("%o", [1]), "1") &&
std.assertEqual(std.format("%o", [511]), "777") &&

std.assertEqual(std.format("%x", [0]), "0") &&
std.assertEqual(std.format("%x", [1]), "1") &&
std.assertEqual(std.format("%x", [10]), "a") &&

std.assertEqual(std.format("%X", [0]), "0") &&
std.assertEqual(std.format("%X", [1]), "1") &&
std.assertEqual(std.format("%X", [10]), "A") &&

std.assertEqual(std.format("%f", [0]), "0.000000") &&
std.assertEqual(std.format("%F", [0]), "0.000000") &&
std.assertEqual(std.format("%f", [1.25]), "1.250000") &&
std.assertEqual(std.format("%F", [1.25]), "1.250000") &&
std.assertEqual(std.format("%.3f", [-1.25]), "-1.250") &&
std.assertEqual(std.format("%+.3f", [1.25]), "+1.250") &&
std.assertEqual(std.format("% .3f", [1.25]), " 1.250") &&
std.assertEqual(std.format("%2.3f", [1.25]), "1.250") &&
std.assertEqual(std.format("%2.3f", [-1.25]), "-1.250") &&
std.assertEqual(std.format("%8.3f", [1.25]), "   1.250") &&
std.assertEqual(std.format("%8.3f", [-1.25]), "  -1.250") &&
std.assertEqual(std.format("%02.3f", [1.25]), "1.250") &&
std.assertEqual(std.format("%02.3f", [-1.25]), "-1.250") &&
std.assertEqual(std.format("%08.3f", [1.25]), "0001.250") &&
std.assertEqual(std.format("%08.3f", [-1.25]), "-001.250") &&
std.assertEqual(std.format("%.3f", [1e20]), "100000000000000000000.000") &&
std.assertEqual(std.format("%.3f", [1e-2]), "0.010") &&
std.assertEqual(std.format("%.3f", [1e-20]), "0.000") &&
std.assertEqual(std.format("%.3f", [-1e-20]), "-0.000") &&
std.assertEqual(std.format("%.3f", [0]), "0.000") &&
std.assertEqual(std.format("%.3f", [-0]), "0.000") &&
std.assertEqual(std.format("%.24f", [1e-20]), "0.000000000000000000010000") &&

std.assertEqual(std.format("%e", [0]), "0.000000e+00") &&
std.assertEqual(std.format("%E", [0]), "0.000000E+00") &&
std.assertEqual(std.format("%e", [10.25]), "1.025000e+01") &&
std.assertEqual(std.format("%E", [10.25]), "1.025000E+01") &&
std.assertEqual(std.format("%e", [0.25]), "2.500000e-01") &&
std.assertEqual(std.format("%.3e", [-1.5]), "-1.500e+00") &&
std.assertEqual(std.format("%+.3e", [1.5]), "+1.500e+00") &&
std.assertEqual(std.format("% .3e", [1.5]), " 1.500e+00") &&
std.assertEqual(std.format("%.3e", [1.5e5]), "1.500e+05") &&
std.assertEqual(std.format("%.3e", [1.5e50]), "1.500e+50") &&
std.assertEqual(std.format("%.3e", [1.5e100]), "1.500e+100") &&
std.assertEqual(std.format("%4.3e", [1.5e1]), "1.500e+01") &&
std.assertEqual(std.format("%4.3e", [-1.5e1]), "-1.500e+01") &&
std.assertEqual(std.format("%12.3e", [1.5e1]), "   1.500e+01") &&
std.assertEqual(std.format("%12.3e", [-1.5e1]), "  -1.500e+01") &&
std.assertEqual(std.format("%04.3e", [1.5e1]), "1.500e+01") &&
std.assertEqual(std.format("%04.3e", [-1.5e1]), "-1.500e+01") &&
std.assertEqual(std.format("%012.3e", [1.5e1]), "0001.500e+01") &&
std.assertEqual(std.format("%012.3e", [-1.5e1]), "-001.500e+01") &&

std.assertEqual(std.format("%g", [0]), "0") &&
std.assertEqual(std.format("%G", [0]), "0") &&
std.assertEqual(std.format("%g", [1.25]), "1.25") &&
std.assertEqual(std.format("%g", [-1.25]), "-1.25") &&
std.assertEqual(std.format("%+g", [1.25]), "+1.25") &&
std.assertEqual(std.format("% g", [1.25]), " 1.25") &&
std.assertEqual(std.format("%.5g", [1.25]), "1.25") &&
std.assertEqual(std.format("%g", [1.25e10]), "1.25e+10") &&
std.assertEqual(std.format("%G", [1.25e10]), "1.25E+10") &&
std.assertEqual(std.format("%.2g", [1.25e10]), "1.2e+10") &&
std.assertEqual(std.format("%.5g", [1.25e10]), "1.25e+10") &&
std.assertEqual(std.format("%#g", [1.25]), "1.25000") &&
std.assertEqual(std.format("%#.2g", [1.25]), "1.2") &&
std.assertEqual(std.format("%#.5g", [1.25]), "1.2500") &&
std.assertEqual(std.format("%#g", [1.25e10]), "1.25000e+10") &&
std.assertEqual(std.format("%#.5g", [1.25e10]), "1.2500e+10") &&
std.assertEqual(std.format("%8g", [1.25]), "    1.25") &&
std.assertEqual(std.format("%8g", [-1.25]), "   -1.25") &&
std.assertEqual(std.format("%08g", [1.25]), "00001.25") &&
std.assertEqual(std.format("%08g", [-1.25]), "-0001.25") &&
std.assertEqual(std.format("%10g", [1.5e10]), "   1.5e+10") &&
std.assertEqual(std.format("%10g", [-1.5e10]), "  -1.5e+10") &&
std.assertEqual(std.format("%010g", [1.5e10]), "0001.5e+10") &&
std.assertEqual(std.format("%010g", [-1.5e10]), "-001.5e+10") &&

// Single value.
std.assertEqual(std.format("%d", 10), "10") &&

// Format length modifiers are ignored.
std.assertEqual(std.format("%hi", [10]), "10") &&
std.assertEqual(std.format("%li", [10]), "10") &&
std.assertEqual(std.format("%Li", [10]), "10") &&

std.assertEqual(std.format("", {}), "") &&
std.assertEqual(std.format("%(a)d", {a: 10}), "10") &&

std.assertEqual("%d" % [10], "10") &&
std.assertEqual("%d" % 10, "10") &&
std.assertEqual("%(a)d" % { a: 10 }, "10") &&

true
