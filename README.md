# WebBundle library

[![build](https://github.com/google/webbundle/workflows/build/badge.svg)](https://github.com/google/webbundle/actions)
[![crates.io](https://img.shields.io/crates/v/webbundle.svg)](https://crates.io/crates/webbundle)

An experimental WebBundle library for packaging web sites.

This is not an officially supported Google product.

## Documentation

[https://docs.rs/webbundle](https://docs.rs/webbundle)

## Specification

- [Web Bundles](https://wicg.github.io/webpackage/draft-yasskin-wpack-bundled-exchanges.html)

## Contributing

See [contributing.md](contributing.md) for instructions.

## TODO

The development is at very early stage. There are many TODO items:

- [x] Parser
- [x] WebBundle Builder
  - [x] Create a WebBundle from a directory structure
  - [x] Low-level APIs to create and manipulate WebBundle file
- [x] Use `http::Request`, `http::Response` and `http::Uri` for better engonomics
- [ ] Support Signatures
- [ ] Support Variants
- [ ] Use async/await to avoid blocking operations
- [ ] More CLI subcommands
  - [x] `create`
  - [x] `dump` (deprecated)
  - [x] `list`
  - [x] `extract`
  - [ ] Make these subcommands more ergonomics
- [ ] Focus the performance. Avoid copy as much as possible.
- [ ] Split this crate into several crates:
  - [ ] `webbundle`: Core library
  - [ ] `webbundle-cli`: CLI, like a `tar` command
  - [ ] `webbundle-ffi`: Foreign function interface for C or C++ program, like a chromium
  - [ ] `webbundle-server`: Experimental http server which can assemble and serve a webbundle dynamically, based on request parameters

## Command line tool

This repository also contains a command line tool, called `webbundle`.

### Instalation

[Archives of precompiled binaries for `webbundle` are available for
Windows, macOS and Linux](https://github.com/google/webbundle/releases).

If you're a Rust programmer, `webbundle` can be installed with `cargo`.

```shell
cargo install --features=cli webbundle
```

## Examples

### create

Create `example.wbn` from the files under `build/dist` directory.
This is similar to `tar cvf example.tar build/dist`.

```
$ webbundle create --base-url "https://example.com/" --primary-url "https://example.com/foo/" example.wbn build/dist
```

### list

List the contents of `example.wbn`.
This is similar to `tar tvf example.tar`.

```
$ webbundle list ./example.wbn
```

### extract

Extract the contents of `example.wbn`.
This is similar to `tar xvf example.tar`.

```
$ webbundle extract ./example.wbn
```

See `webbundle --help` for detail usage.
