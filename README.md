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

See [/docs/contributing.md](docs/contributing.md) for instructions.

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
  - [x] `list`
  - [ ] `extract`

## Command line tool

This repository also contains a command line tool, called `webbundle`.

To install `webbundle` command, run the following:

```shell
cargo install --features=cli webbundle
```

See `webbundle --help` for the usage.
