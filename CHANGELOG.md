# Changelog

## Unreleased

### Changed

- When a stack trace is larger than the specified maximum, hide items in the
  middle instead of the end.
- Documentation improvements

### Fixed

- Do not use `f64::log2` to implement `std.exponent` and `std.mantissa`, which
  can be too inaccurate in some platforms.
- Errors will not show `<000D>` at the end of each source line when source has
  CRLF line endings.

## 0.1.0 (2024-04-06)

- Initial release
