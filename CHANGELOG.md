# Changelog

## 0.4.0 (2025-05-10)

### Breaking

- Performing bitwise operations with integers that cannot be safely represented
  by a `f64` will fail.

### Added

- New language features from Jsonnet 0.21:
  - Slices now accept negative indexes.
  - Chomped multi-line text blocks (which start with `|||-`).
- New standard library elements from Jsonnet 0.21:
  - `std.pi` (constant)
  - `std.log2(x)`
  - `std.log10(x)`
  - `std.atan2(y, x)`
  - `std.deg2rad(x)`
  - `std.rad2deg(x)`
  - `std.hypot(a, b)`
  - `std.isEven(x)`
  - `std.isOdd(x)`
  - `std.isInteger(x)`
  - `std.isDecimal(x)`
  - `std.trim(str)`
  - `std.equalsIgnoreCase(str1, str2)`
  - `std.flattenDeepArray(value)`
  - `std.minArray(arr, keyF, onEmpty)`
  - `std.maxArray(arr, keyF, onEmpty)`
  - `std.contains(arr, elem)`
  - `std.avg(arr)`
  - `std.remove(arr, elem)`
  - `std.removeAt(arr, idx)`
  - `std.objectRemoveKey(obj, key)`
  - `std.sha1(s)`
  - `std.sha256(s)`
  - `std.sha512(s)`
  - `std.sha3(s)`

### Compatibility

- Minimum Supported Rust Version (MSRV) has been bumped to 1.75.

## 0.3.0 (2024-11-07)

### Breaking

- `EvalErrorKind::InvalidBuiltInFuncArgType` has been renamed to
  `EvalErrorKind::InvalidStdFuncArgType`.
- New variants have been added to `EvalErrorKind`.
- New variants have been added to `EvalStackTraceItem`.
- `ExpectedThing` has been renamed to `ExpectedToken`.
- `ParseError::Expected` now uses `ActualToken` instead of `TokenKind`.
- Arena allocation is now used for interned strings, AST and some internal data
  of `Program`.
- String interner cannot be garbage-collected anymore.

### Fixed

- Reject tags in object keys in `std.parseYaml`.
- Avoid some panics in `std.parseYaml` with some inputs found with fuzzing.
- Avoid stack overflow when dropping expressions with too much nesting.

### Changed

- Improved error message when passing a value of invalid type to a built-in function.
- Stack trace improvements.
- Performance improvements.

## 0.2.0 (2024-08-12)

### Breaking

- Some semantics of `std.parseYaml` have changed:
  - Empty YAML documents are now parsed as `null` instead of an empty string.
  - Anchors and aliases are now supported.
  - `Null`, `NULL` and `~` are now parsed as `null`.
  - `True` and `TRUE` are now parsed as `true` .
  - `False` and `FALSE` are now parsed as `false`.
  - Leading or trailing dot (e.g., `.5` or `1.`) is now allowed in floating
    point numbers.
  - Explicit `+` is now allowed in floating point numbers.
  - Leading zeros are now allowed in floating point numbers.
  - Octal and hexadecimal intergers are now parsed as numbers.

### Fixed

- Detect stack overflow while comparing arrays and objects.
- Detect stack overflow while converting arrays and objects to string.
- Avoid panic when encountering an array or object as object key in
  `std.parseYaml`.
- Avoid panic when encountering a YAML alias in `std.parseYaml`.
- Return character indices instead of UTF-8 byte indices in `std.findSubstr`.
- Avoid panic when the first character of the pattern passed to `std.findSubstr`
  is not ASCII.

## 0.1.2 (2024-07-20)

### Fixed

- Avoid panic if the array passed to `std.format` does not have enough items.

## 0.1.1 (2024-04-24)

### Changed

- When a stack trace is larger than the specified maximum, hide items in the
  middle instead of the end.
- Documentation improvements.

### Fixed

- Do not use `f64::log2` to implement `std.exponent` and `std.mantissa`, which
  can be too inaccurate in some platforms.
- Errors will not show `<000D>` at the end of each source line when source has
  CRLF line endings.

## 0.1.0 (2024-04-06)

- Initial release
