# Changelog

## 0.2.0 (unreleased)

### Breaking

- Some semantics of `std.parseYaml` have changed:
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
