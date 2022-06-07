# Synthetic Module Benchmark with Web Bundles

## Basic Usage

- Install `Rust` and `Deno` (optional) beforehand.
- See [`Make.zsh`](./Make.zsh) for example usages. The following is summary:

1. Checkout:

   ```shell
   git clone https://github.com/google/webbundle.git`
   ```

2. Generate the modules and the benchmarks.

   Example:

   ```shell
   cd webbundle/webbundle-bench
   cargo run --release -- --out out --depth 4 --branches 4
   ```

   See `build()` in `Make.zsh`.

3. Start webserver.

   Example:

   ```shell
   cd ../webbundle-server
   cargo build --release
   cd ../webbundle-bench
   RUST_LOG=error ../target/release/webbundle-server --port 8080
   ```

   See `run_webserver()` in `Make.zsh`.

4. Open `http://localhost:8080/out/index.html` in your browser, and click each benchmark.

5. (Optional) Run the benchmark using puppeteer for automation:

   ```shell
   deno run --allow-all ./run-bench.ts --port 8080
   ```

   See `bench()` in `Make.zsh`.

## What's not implemented

`webbundle-bench` is inspired by
[`js-modules-benchmark`](https://github.com/GoogleChromeLabs/js-module-benchmark).

`webbundle-bench` is rewritten with Rust and Deno so we don't depend on Python and Node.js, and supports very minimum features which are necessary to benchmark Web Bundle loading performance.

`webbundle-bench` is missing many features at this point:

- [ ] Support `modulepreload`.
- [ ] Rules to generate modules.
- [ ] Specify the size of generated modules.
- [ ] Support other bundlers (e.g. `rollup`)
