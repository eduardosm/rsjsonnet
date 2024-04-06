# rsjsonnet

A Rust implementation of the [Jsonnet language](https://jsonnet.org/).
Currently, it targets Jsonnet version 0.20.

This project provides three crates:

* [rsjsonnet](rsjsonnet/README.md): a command line program with an
  interface similar to the official implementation.
* [rsjsonnet-lang](rsjsonnet-lang/README.md): a library that provides
  parsing and evaluation of Jsonnet programs.
* [rsjsonnet-front](rsjsonnet-front/README.md): a library built on top of
  rsjsonnet-lang and provides easy source loading and error printing.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

The following are taken from the [C++ implementation](https://github.com/google/jsonnet),
which is licensed under Apache 2.0.

* The part of the standard library that is implemented in Jsonnet
  ([rsjsonnet-lang/src/program/std.jsonnet](rsjsonnet-lang/src/program/std.jsonnet)).
* Part of the test suite ([ui-tests/jsonnet-0.20.0](ui-tests/jsonnet-0.20.0)).
