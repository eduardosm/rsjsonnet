local assertApprox(actual, expected) =
  if expected == 0 then
    std.assertEqual(actual, expected)
  else if (std.abs(actual - expected) / std.abs(expected)) < 1e-15 then
    true
  else
    error "assertApprox failed: " + actual + " != " + expected;

assertApprox(std.pi, 3.141592653589793) &&

std.assertEqual(std.abs(1.5), 1.5) &&
std.assertEqual(std.abs(-1.5), 1.5) &&
# Weird, but it is implemented that way
std.assertEqual(std.toString(std.abs(0)), "-0") &&
std.assertEqual(std.toString(std.abs(-0)), "0") &&

std.assertEqual(std.sign(1.5), 1) &&
std.assertEqual(std.sign(-1.5), -1) &&
std.assertEqual(std.sign(0), 0) &&
std.assertEqual(std.sign(-0), 0) &&

std.assertEqual(std.min(1, 2), 1) &&
std.assertEqual(std.min(2, 1), 1) &&
std.assertEqual(std.min(-1, -2), -2) &&
std.assertEqual(std.min(-2, -1), -2) &&
std.assertEqual(std.min(1, 1), 1) &&

std.assertEqual(std.max(1, 2), 2) &&
std.assertEqual(std.max(2, 1), 2) &&
std.assertEqual(std.max(-1, -2), -1) &&
std.assertEqual(std.max(-2, -1), -1) &&
std.assertEqual(std.max(1, 1), 1) &&

std.assertEqual(std.clamp(-3, 1, 5), 1) &&
std.assertEqual(std.clamp(4, 1, 5), 4) &&
std.assertEqual(std.clamp(7, 1, 5), 5) &&

std.assertEqual(std.exponent(0), 0) &&
std.assertEqual(std.exponent(0.09375), -3) &&
std.assertEqual(std.exponent(-0.09375), -3) &&
std.assertEqual(std.exponent(0.25), -1) &&
std.assertEqual(std.exponent(-0.25), -1) &&
std.assertEqual(std.exponent(0.5), 0) &&
std.assertEqual(std.exponent(-0.5), 0) &&
std.assertEqual(std.exponent(0.75), 0) &&
std.assertEqual(std.exponent(-0.75), 0) &&
std.assertEqual(std.exponent(1.0), 1) &&
std.assertEqual(std.exponent(-1.0), 1) &&
std.assertEqual(std.exponent(1.5), 1) &&
std.assertEqual(std.exponent(-1.5), 1) &&
std.assertEqual(std.exponent(20), 5) &&
std.assertEqual(std.exponent(-20), 5) &&

std.assertEqual(std.mantissa(0), 0) &&
std.assertEqual(std.mantissa(0.09375), 0.75) &&
std.assertEqual(std.mantissa(-0.09375), -0.75) &&
std.assertEqual(std.mantissa(0.25), 0.5) &&
std.assertEqual(std.mantissa(-0.25), -0.5) &&
std.assertEqual(std.mantissa(0.5), 0.5) &&
std.assertEqual(std.mantissa(-0.5), -0.5) &&
std.assertEqual(std.mantissa(0.75), 0.75) &&
std.assertEqual(std.mantissa(-0.75), -0.75) &&
std.assertEqual(std.mantissa(1.0), 0.5) &&
std.assertEqual(std.mantissa(-1.0), -0.5) &&
std.assertEqual(std.mantissa(1.5), 0.75) &&
std.assertEqual(std.mantissa(-1.5), -0.75) &&
std.assertEqual(std.mantissa(20), 0.625) &&
std.assertEqual(std.mantissa(-20), -0.625) &&

std.assertEqual(std.floor(-1.75), -2) &&
std.assertEqual(std.floor(-1.50), -2) &&
std.assertEqual(std.floor(-1.25), -2) &&
std.assertEqual(std.floor(-1.00), -1) &&
std.assertEqual(std.floor(-0.75), -1) &&
std.assertEqual(std.floor(-0.50), -1) &&
std.assertEqual(std.floor(-0.25), -1) &&
std.assertEqual(std.floor(0.00), 0) &&
std.assertEqual(std.floor(0.25), 0) &&
std.assertEqual(std.floor(0.50), 0) &&
std.assertEqual(std.floor(0.75), 0) &&
std.assertEqual(std.floor(1.00), 1) &&
std.assertEqual(std.floor(1.25), 1) &&
std.assertEqual(std.floor(1.50), 1) &&
std.assertEqual(std.floor(1.75), 1) &&

std.assertEqual(std.ceil(-1.75), -1) &&
std.assertEqual(std.ceil(-1.50), -1) &&
std.assertEqual(std.ceil(-1.25), -1) &&
std.assertEqual(std.ceil(-1.00), -1) &&
std.assertEqual(std.ceil(-0.75), 0) &&
std.assertEqual(std.ceil(-0.50), 0) &&
std.assertEqual(std.ceil(-0.25), 0) &&
std.assertEqual(std.ceil(0.00), 0) &&
std.assertEqual(std.ceil(0.25), 1) &&
std.assertEqual(std.ceil(0.50), 1) &&
std.assertEqual(std.ceil(0.75), 1) &&
std.assertEqual(std.ceil(1.00), 1) &&
std.assertEqual(std.ceil(1.25), 2) &&
std.assertEqual(std.ceil(1.50), 2) &&
std.assertEqual(std.ceil(1.75), 2) &&

std.assertEqual(std.round(-1.75), -2) &&
std.assertEqual(std.round(-1.50), -1) &&
std.assertEqual(std.round(-1.25), -1) &&
std.assertEqual(std.round(-1.00), -1) &&
std.assertEqual(std.round(-0.75), -1) &&
std.assertEqual(std.round(-0.50), 0) &&
std.assertEqual(std.round(-0.25), 0) &&
std.assertEqual(std.round(0.00), 0) &&
std.assertEqual(std.round(0.25), 0) &&
std.assertEqual(std.round(0.50), 1) &&
std.assertEqual(std.round(0.75), 1) &&
std.assertEqual(std.round(1.00), 1) &&
std.assertEqual(std.round(1.25), 1) &&
std.assertEqual(std.round(1.50), 2) &&
std.assertEqual(std.round(1.75), 2) &&

std.assertEqual(std.modulo(5.5, 2), 1.5) &&
std.assertEqual(std.modulo(5.5, -2), 1.5) &&
std.assertEqual(std.modulo(-5.5, 2), -1.5) &&
std.assertEqual(std.modulo(-5.5, -2), -1.5) &&

std.assertEqual(std.pow(10, 3), 1000) &&
assertApprox(std.pow(7.7, 3.3), 842.20805947902409) &&

assertApprox(std.exp(5.5), 244.69193226422038) &&
assertApprox(std.exp(-2.4), 0.090717953289412512) &&

assertApprox(std.log(33.33), 3.5064578923196481) &&
assertApprox(std.log(0.0044), -5.4261507380579213) &&
assertApprox(std.log2(41.14), 5.362469888750209) &&
assertApprox(std.log2(0.059), -4.083141235300245) &&
assertApprox(std.log10(88.88), 1.948804045932811) &&
assertApprox(std.log10(0.00077), -3.113509274827518) &&

std.assertEqual(std.sqrt(0), 0) &&
std.assertEqual(std.sqrt(1), 1) &&
std.assertEqual(std.sqrt(4), 2) &&
std.assertEqual(std.sqrt(2.25), 1.5) &&
std.assertEqual(std.sqrt(1089), 33) &&
assertApprox(std.sqrt(2), 1.4142135623730951) &&
assertApprox(std.sqrt(0.05), 0.22360679774997896) &&
assertApprox(std.sqrt(1e-308), 1e-154) &&
assertApprox(std.sqrt(1e308), 1e154) &&

assertApprox(std.hypot(3, 4), 5) &&
assertApprox(std.hypot(1.5e301, 3.1e300), 1.5316984037335809e301) &&

assertApprox(std.deg2rad(40.6), 0.7086036763096978) &&
assertApprox(std.rad2deg(1.32), 75.63042895726866) &&

assertApprox(std.sin(1.5), 0.99749498660405445) &&
assertApprox(std.cos(1.5), 0.070737201667702906) &&
assertApprox(std.tan(1.5), 14.101419947171719) &&

assertApprox(std.asin(0.75), 0.848062078981481) &&
assertApprox(std.acos(0.75), 0.72273424781341566) &&
assertApprox(std.atan(0.75), 0.64350110879328437) &&
assertApprox(std.atan2(0.75, 0.25), 1.2490457723982544) &&

std.assertEqual(std.isEven(4.0), true) &&
std.assertEqual(std.isEven(-4.0), true) &&
std.assertEqual(std.isEven(5.0), false) &&
std.assertEqual(std.isEven(-5.0), false) &&

std.assertEqual(std.isOdd(4.0), false) &&
std.assertEqual(std.isOdd(-4.0), false) &&
std.assertEqual(std.isOdd(5.0), true) &&
std.assertEqual(std.isOdd(-5.0), true) &&

std.assertEqual(std.isInteger(56.0), true) &&
std.assertEqual(std.isInteger(-56.0), true) &&
std.assertEqual(std.isInteger(1.5e200), true) &&
std.assertEqual(std.isInteger(-1.5e200), true) &&
std.assertEqual(std.isInteger(40.2), false) &&
std.assertEqual(std.isInteger(-40.2), false) &&
std.assertEqual(std.isInteger(1e-200), false) &&
std.assertEqual(std.isInteger(1e-200), false) &&

std.assertEqual(std.isDecimal(56.0), false) &&
std.assertEqual(std.isDecimal(-56.0), false) &&
std.assertEqual(std.isDecimal(1.5e200), false) &&
std.assertEqual(std.isDecimal(-1.5e200), false) &&
std.assertEqual(std.isDecimal(40.2), true) &&
std.assertEqual(std.isDecimal(-40.2), true) &&
std.assertEqual(std.isDecimal(1e-200), true) &&
std.assertEqual(std.isDecimal(1e-200), true) &&

true
