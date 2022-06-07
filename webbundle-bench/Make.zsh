# * Example usages of webbundle-bench

port=8080

# * Build benchmarks
build() {
  cargo run --release -- --out out --depth 4 --branches 4
}

# * Run webserver
run_webserver() {
  cd ../webbundle-server && cargo build --release && \
    cd ../webbundle-bench && RUST_LOG=error ../target/release/webbundle-server --port ${port}
}

# Run web server written in Deno, as an alternative of
# `webbundle-server`. Either should work, although `webbundle-server`
# is faster.
run_webserver_deno() {
  deno run --allow-all ./run-webserver.ts --port ${port}
}

# * Run Benchmark
bench() {
  # Please use your own chrome with --browser option.
  deno run --allow-all ./run-bench.ts --browser ~/src/chrome1/src/out/Default/chrome --port ${port}
}
