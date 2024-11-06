# rsjsonnet

[![GitHub Actions Status](https://github.com/eduardosm/rsjsonnet/workflows/CI/badge.svg)](https://github.com/eduardosm/rsjsonnet/actions)
![MSRV](https://img.shields.io/badge/rustc-1.74+-lightgray.svg)

A Rust implementation of the [Jsonnet language](https://jsonnet.org/).
Currently, it targets Jsonnet version 0.20.

This project provides three crates:

* [rsjsonnet](rsjsonnet/README.md): a command line program with an
  interface similar to the official implementation.
* [rsjsonnet-lang](rsjsonnet-lang/README.md): a library that provides
  parsing and evaluation of Jsonnet programs.
* [rsjsonnet-front](rsjsonnet-front/README.md): a library built on top of
  rsjsonnet-lang and provides easy source loading and error printing.

## Command line program

### Pre-built binaries

You can download binaries of the command line for Linux and Windows from the
[GitHub releases page](https://github.com/eduardosm/rsjsonnet/releases).

### Build from source

If you have a Rust toolchain installed on your system, you can build the
latest version with:

```sh
cargo install --locked rsjsonnet
```

### Usage

rsjsonnet provides a command line interface similar to the one of the official
implementation.

```text
Usage: rsjsonnet [OPTIONS] <filename>

Arguments:
  <filename>

Options:
  -e, --exec                      Treat filename as code
  -J, --jpath <dir>               Specify an additional library search dir (right-most wins)
  -o, --output-file <file>        Write to the output file rather than stdout
  -m, --multi <dir>               Write multiple files to the directory, list files on stdout
  -y, --yaml-stream               Write output as a YAML stream of JSON documents
  -S, --string                    Expect a string, manifest as plain text
  -s, --max-stack <n>             Number of allowed stack frames
  -t, --max-trace <n>             Max length of stack trace before cropping
  -V, --ext-str <var[=val]>       Provide an external variable as a string
      --ext-str-file <var=file>   Provide an external variable as a string read from a file
      --ext-code <var[=code]>     Provide an external variable as code
      --ext-code-file <var=file>  Provide an external variable as code read from a file
  -A, --tla-str <var[=val]>       Provide a top-level argument as a string
      --tla-str-file <var=file>   Provide a top-level argument as a string read from a file
      --tla-code <var[=code]>     Provide a top-level argument as code
      --tla-code-file <var=file>  Provide a top-level argument as code read from a file
  -h, --help                      Print help
```

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

Part of the test suite ([ui-tests/jsonnet-0.20.0](ui-tests/jsonnet-0.20.0)) is
taken from the [C++ implementation](https://github.com/google/jsonnet), which
is licensed under Apache 2.0.
