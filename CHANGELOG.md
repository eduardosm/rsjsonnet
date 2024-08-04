# Changelog

## 0.2.0 (unreleased)

### Breaking

- Parse `Null`, `NULL` and `~` as `null` in `std.parseYaml`.
- Parse `True` and `TRUE` as `true` in `std.parseYaml`.
- Parse `False` and `FALSE` as `false` in `std.parseYaml`.
- Allow leading or trailing dot (e.g., `.5` or `1.`) in floating point numbers
  in `std.parseYaml`.
- Allow explicit `+` in floating point numbers in `std.parseYaml`.
- Allow leading zeros in floating point numbers in `std.parseYaml`.
- Parse octal and hexadecimal intergers as numbers in `std.parseYaml`.

### Fixed

- Detect stack overflow while comparing arrays and objects.
- Detect stack overflow while converting arrays and objects to string.
- Avoid panic when encountering an array or object as object key in
  `std.parseYaml`.
- Avoid panic when encountering a YAML alias in `std.parseYaml`.

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
