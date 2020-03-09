// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::{ensure, Context as _};
use chrono::Local;
use std::fs::File;
use std::io::{BufWriter, Read as _, Write as _};
use std::path::{Component, Path, PathBuf};
use structopt::StructOpt;
use webbundle::{Bundle, Result, Uri, Version};

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Example: webbundle create -b "https://example.com/" -p "https://example.com/foo/index.html" example.wbn foo
    #[structopt(name = "create")]
    Create {
        #[structopt(short = "b", long = "base-url")]
        base_url: String,
        #[structopt(short = "p", long = "primary-url")]
        primary_url: String,
        #[structopt(short = "m", long = "manifest")]
        manifest: Option<String>,
        /// File name
        file: String,
        /// Directory from where resources are read
        resources_dir: String,
        // TODO: Support version
    },
    /// (deprecated) Example: webbundle dump ./example.wbn
    #[structopt(name = "dump")]
    Dump { file: String },
    /// List the contents briefly
    #[structopt(name = "list")]
    List { file: String },
    /// Extract the contents
    #[structopt(name = "extract")]
    Extract { file: String },
}

fn env_logger_init() {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {:5} {}] ({}:{}) {}",
                Local::now().format("%+"),
                buf.default_styled_level(record.level()),
                record.target(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args(),
            )
        })
        .init();
}

fn list(bundle: &Bundle) {
    println!("primary-url: {}", bundle.primary_url());
    if let Some(manifest) = bundle.manifest() {
        println!("manifest: {}", manifest);
    }
    for exchange in bundle.exchanges() {
        let request = &exchange.request;
        let response = &exchange.response;
        println!(
            "{} {} {} bytes",
            request.uri(),
            response.status(),
            response.body().len()
        );
        log::debug!("headers: {:?}", response.headers());
    }
}

fn make_url_path_relative(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref()
        .components()
        .fold(PathBuf::new(), |mut result, p| match p {
            Component::Normal(x) => {
                result.push(x);
                result
            }
            Component::ParentDir => {
                log::warn!("path contains: {}", path.as_ref().display());
                result.pop();
                result
            }
            _ => result,
        })
}

fn url_to_path(uri: &Uri) -> PathBuf {
    let mut path = PathBuf::new();
    if let Some(scheme) = uri.scheme_str() {
        path.push(scheme);
    }
    if let Some(host) = uri.host() {
        path.push(host);
    }
    if let Some(port) = uri.port() {
        path.push(port.to_string());
    }
    let relative = make_url_path_relative(uri.path());
    // We push `relative` here even if it is empty.
    // That makes sure path ends with "/".
    path.push(relative);
    // TODO: Push query
    path
}

#[test]
fn url_to_path_test() -> Result<()> {
    assert_eq!(
        url_to_path(&"https://example.com/".parse()?),
        Path::new("https/example.com/")
    );
    assert_eq!(
        url_to_path(&"https://example.com".parse()?),
        Path::new("https/example.com/")
    );
    assert_eq!(
        url_to_path(&"https://example.com/index.html".parse()?),
        Path::new("https/example.com/index.html")
    );
    assert_eq!(
        url_to_path(&"https://example.com/a/".parse()?),
        Path::new("https/example.com/a/")
    );
    assert_eq!(
        url_to_path(&"https://example.com/a/b".parse()?),
        Path::new("https/example.com/a/b")
    );
    assert_eq!(
        url_to_path(&"https://example.com/a/b/".parse()?),
        Path::new("https/example.com/a/b/")
    );
    Ok(())
}

fn extract(bundle: &Bundle) -> Result<()> {
    // TODO: Avoid the conflict of file names.
    // The current approach is too naive.
    for exchange in bundle.exchanges() {
        let path = url_to_path(exchange.request.uri());
        ensure!(
            path.is_relative(),
            format!("path shoould be relative: {}", path.display())
        );
        if !exchange.response.status().is_success() {
            log::info!("Skipping: {:?}", exchange.request.uri());
            continue;
        }
        // TODO: "/" should be path::sep in windows?
        if path.display().to_string().ends_with('/') {
            if !path.exists() {
                std::fs::create_dir_all(&path)?;
            }
            // Use index.html
            let index_html = path.join("index.html");
            log::info!(
                "extract: {} => {}",
                exchange.request.uri(),
                index_html.display()
            );
            let mut write = BufWriter::new(File::create(&index_html)?);
            write.write_all(exchange.response.body())?;
        } else {
            log::info!("extract: {} => {}", exchange.request.uri(), path.display());
            let parent = path.parent().context("weired url")?;
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
            let mut write = BufWriter::new(File::create(&path)?);
            write.write_all(exchange.response.body())?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger_init();
    let args = Cli::from_args();
    match args.cmd {
        Command::Create {
            base_url,
            primary_url,
            file,
            manifest,
            resources_dir,
        } => {
            let mut builder = Bundle::builder()
                .version(Version::VersionB1)
                .primary_url(primary_url.parse()?)
                .exchanges_from_dir(resources_dir, base_url.parse()?)?;
            if let Some(manifest) = manifest {
                builder = builder.manifest(manifest.parse()?);
            }
            let bundle = builder.build()?;
            log::debug!("{:#?}", bundle);
            let write = BufWriter::new(File::create(&file)?);
            bundle.write_to(write)?;
        }
        Command::List { file } => {
            let mut buf = Vec::new();
            File::open(&file)?.read_to_end(&mut buf)?;
            let bundle = Bundle::from_bytes(buf)?;
            list(&bundle);
        }
        Command::Dump { file } => {
            let mut buf = Vec::new();
            File::open(&file)?.read_to_end(&mut buf)?;
            let bundle = Bundle::from_bytes(buf)?;
            println!("{:#?}", bundle);
        }
        Command::Extract { file } => {
            let mut buf = Vec::new();
            File::open(&file)?.read_to_end(&mut buf)?;
            let bundle = Bundle::from_bytes(buf)?;
            extract(&bundle)?;
        }
    }
    Ok(())
}
