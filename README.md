# WebBundle

[![build](https://github.com/google/webbundle/workflows/build/badge.svg)](https://github.com/google/webbundle/actions)

`google/webbundle` is a project which aims to provide a high-performace library
and various tools for handling WebBundle format.

This is not an officially supported Google product.

## Specification

- [Web Bundles](https://wicg.github.io/webpackage/draft-yasskin-wpack-bundled-exchanges.html)

## [webbundle](https://github.com/google/webbundle/tree/master/webbundle)

[![crates.io](https://img.shields.io/crates/v/webbundle.svg)](https://crates.io/crates/webbundle?label=webbundle)

A core library. See [the documentation](https://docs.rs/webbundle).

## [webbundle-cli](https://github.com/google/webbundle/tree/master/webbundle-cli)

[![crates.io](https://img.shields.io/crates/v/webbundle-cli.svg)](https://crates.io/crates/webbundle-cli)

A command line tool for WebBundle.

### Installation

[Archives of precompiled binaries for `webbundle-cli` are available for Windows, macOS and Linux](https://github.com/google/webbundle/releases).

If you're a Rust programmer, `webbundle-cli` can be installed with `cargo`.

```shell
cargo install webbundle-cli
```

### Examples

The binary name for `webbundle-cli` is `webbundle`.

#### create

Create `example.wbn` from the files under `build/dist` directory. This is
similar to `tar cvf example.tar build/dist`.

```
$ webbundle create --base-url "https://example.com/" --primary-url "https://example.com/foo/" example.wbn build/dist
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

## [webbundle-server](https://github.com/google/webbundle/tree/master/webbundle-server)

[![crates.io](https://img.shields.io/crates/v/webbundle-server.svg)](https://crates.io/crates/webbundle-server)

An experimental web server which dynamically assembles and serves WebBundle.

## TODO

The development is at very early stage. There are many TODO items:

- [x] Parser
  - [x] Support b2 format
- [x] Encoder
  - [x] Support b2 format
- [x] WebBundle Builder
  - [x] Create a WebBundle from a directory structure
  - [x] Low-level APIs to create and manipulate WebBundle file
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
    chromium
  - [x] `webbundle-server`: Experimental http server which can assemble and
    serve a webbundle dynamically, based on request parameters
  - [ ] `webbundle-wasm`: WebAssembly binding

## Contributing

See [contributing.md](contributing.md) for instructions.
