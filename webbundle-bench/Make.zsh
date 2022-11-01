# * Example usages of webbundle-bench

port=8080

# * Build benchmarks
build() {
  cargo run --release -- --out out --depth ${1:-4} --branches ${2:-4}
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
  deno run --allow-all ./run-bench.ts --browser ~/src/chrome1/src/out/Default/chrome \
       --port ${port}
}

bench_with_flag() {
  for arg in "" "--enable-blink-features=SubresourceWebBundlesSameOriginOptimization"; do
    echo
    echo "browser flag: $arg"
    # Please use your own chrome with --browser option.
    deno run --allow-all ./run-bench.ts --browser ~/src/chrome1/src/out/Default/chrome \
         --port ${port} -- $arg
  done
}

bench_with_deno_bench() {
  deno bench --unstable --allow-all ./bench.ts -- --browser ~/src/chrome1/src/out/Default/chrome --port ${port}
  echo
  deno bench --unstable --allow-all ./bench.ts -- --browser ~/src/chrome1/src/out/Default/chrome --port ${port} -- --enable-blink-features=SubresourceWebBundlesSameOriginOptimization
}
