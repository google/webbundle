use anyhow::{ensure, Context as _, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let package_name = env::var("CARGO_PKG_NAME").unwrap();
    let output_file = target_dir()
        .unwrap()
        .join(format!("{}-bindgen", package_name))
        .join(format!("{}.h", package_name))
        .display()
        .to_string();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&output_file);
}

// ref.
// https://github.com/dtolnay/cxx/blob/850ca90849e7fb2c045fecdd428f865686e3bb4c/src/paths.rs

fn out_dir() -> Result<PathBuf> {
    env::var("OUT_DIR")
        .map(PathBuf::from)
        .context("OUT_DIR is not set")
}

fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf> {
    Ok(fs::canonicalize(path)?)
}

fn target_dir() -> Result<PathBuf> {
    let mut dir = out_dir().and_then(canonicalize)?;
    // println!("{:?}", dir);
    // eprintln!("{:?}", dir);
    loop {
        if dir.ends_with("target") {
            return Ok(dir);
        }
        ensure!(dir.pop(), "target dir is not found")
    }
}
