# Web bundles

[![build](https://github.com/google/webbundle/workflows/build/badge.svg)](https://github.com/google/webbundle/actions)

`google/webbundle` is a project which aims to provide a high-performace library
and various tools for handling Web bundles format.

This is not an officially supported Google product.

# Specification

- [Web Bundles (IETF draft)](https://wpack-wg.github.io/bundled-responses/draft-ietf-wpack-bundled-responses.html)
- [Subresource Loading](https://wicg.github.io/webpackage/subresource-loading.html)
   ([Explainer](https://github.com/WICG/webpackage/blob/main/explainers/subresource-loading.md)):

# Crates

There are several crates in the repository.

## [webbundle](https://github.com/google/webbundle/tree/main/webbundle)

[![crates.io](https://img.shields.io/crates/v/webbundle.svg)](https://crates.io/crates/webbundle?label=webbundle)

The core library. See [the documentation](https://docs.rs/webbundle).

## [webbundle-cli](https://github.com/google/webbundle/tree/main/webbundle-cli)

[![crates.io](https://img.shields.io/crates/v/webbundle-cli.svg)](https://crates.io/crates/webbundle-cli)

The command line tool for packaging resources as Web Bundles.

### Installation

[Archives of precompiled binaries for `webbundle-cli` are available for Windows, macOS and Linux](https://github.com/google/webbundle/releases).

If you're using Rust, `webbundle-cli` can be installed with `cargo`.

```shell
cargo install webbundle-cli
```

### Examples

The binary name for `webbundle-cli` is `webbundle`.

#### create

Create `example.wbn` from the files under `build/dist` directory. This is
similar to `tar cvf example.tar build/dist`.

```
$ webbundle create example.wbn build/dist
```

#### list

List the contents of `example.wbn`. This is similar to `tar tvf example.tar`.

```
$ webbundle list ./example.wbn
```

#### extract

Extract the contents of `example.wbn`. This is similar to `tar xvf example.tar`.

```
$ webbundle extract ./example.wbn
```

See `webbundle --help` for detail usage.

## [webbundle-server](https://github.com/google/webbundle/tree/main/webbundle-server)

[![crates.io](https://img.shields.io/crates/v/webbundle-server.svg)](https://crates.io/crates/webbundle-server)

The experimental web server which dynamically serves Web bundles from underlying resources.

## [webbundle-bench](https://github.com/google/webbundle/tree/main/webbundle-bench)

[![crates.io](https://img.shields.io/crates/v/webbundle-bench.svg)](https://crates.io/crates/webbundle-bench)

The benchmark tool for measuring the browser's loading performance with Web bundles.

# TODO

The development is at very early stage. There are many TODO items:

- [x] Parser
  - [x] Support b2 format
- [x] Encoder
  - [x] Support b2 format
- [x] Web Bundles Builder
  - [x] Create a Web Bundle from a directory structure
  - [x] Low-level APIs to create and manipulate Web Bundle file
- [x] Use `http::Request`, `http::Response` and `http::Uri` for better
  ergonomics
- [ ] Use async/await to avoid blocking operations
- [ ] More CLI subcommands
  - [x] `create`
  - [x] `list`
  - [x] `extract`
  - [ ] Make these subcommands more ergonomics
- [ ] Focus the performance. Avoid copy as much as possible.
- [ ] Split this crate into several crates:
  - [x] `webbundle`: Core library
  - [x] `webbundle-cli`: CLI, like a `tar` command
  - [x] `webbundle-ffi`: Foreign function interface for C or C++ program, like a
    chromium.
  - [x] `webbundle-server`: Experimental http server which can assemble and
    serve a webbundle dynamically, based on request parameters
  - [ ] `webbundle-wasm`: WebAssembly binding
  - [X] `webbundle-bench`: The benchmark tool

## Contributing

See [contributing.md](contributing.md) for instructions.
