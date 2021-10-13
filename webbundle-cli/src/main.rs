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
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Read as _, Write as _};
use std::path::{Component, Path, PathBuf};
use structopt::clap::arg_enum;
use structopt::StructOpt;
use webbundle::{Bundle, Result, Uri, Version};

#[derive(StructOpt)]
struct Cli {
    #[structopt(subcommand)]
    cmd: Command,
}

arg_enum! {
    #[allow(non_camel_case_types)]
    pub enum Format {
        plain,
        json,
        debug,
    }
}

#[derive(StructOpt)]
enum Command {
    /// Example: webbundle create -b "https://example.com/" -p "https://example.com/foo/index.html" example.wbn foo
    #[structopt(name = "create")]
    Create {
        #[structopt(short = "b", long = "base-url")]
        base_url: String,
        #[structopt(short = "p", long = "primary-url")]
        primary_url: Option<String>,
        /// File name
        file: String,
        /// Directory from where resources are read
        resources_dir: String,
        // TODO: Support version
    },
    /// List the contents briefly
    #[structopt(name = "list")]
    List {
        file: String,
        #[structopt(long = "format", possible_values(&Format::variants()))]
        format: Option<Format>,
    },
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

fn list(bundle: &Bundle, format: Option<Format>) {
    match format {
        None | Some(Format::plain) => list_plain(bundle),
        Some(Format::json) => list_json(bundle),
        Some(Format::debug) => list_debug(bundle),
    }
}

fn list_plain(bundle: &Bundle) {
    if let Some(primary_url) = bundle.primary_url() {
        println!("primary_url: {}", primary_url);
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

fn list_json(bundle: &Bundle) {
    #[derive(Serialize)]
    struct Request {
        uri: String,
    }

    #[derive(Serialize)]
    struct Response {
        status: u16,
        size: usize,
        body: String,
    }

    #[derive(Serialize)]
    struct Body {
        body: String,
    }

    #[derive(Serialize)]
    struct Exchange {
        request: Request,
        response: Response,
    }

    #[derive(Serialize)]
    struct Bundle<'a> {
        version: &'a [u8],
        primary_url: &'a Option<String>,
        exchanges: Vec<Exchange>,
    }

    let bundle = Bundle {
        version: bundle.version().bytes(),
        primary_url: &bundle.primary_url().as_ref().map(|uri| uri.to_string()),
        exchanges: bundle
            .exchanges()
            .iter()
            .map(|exchange| Exchange {
                request: Request {
                    uri: exchange.request.uri().to_string(),
                },
                response: Response {
                    status: exchange.response.status().as_u16(),
                    size: exchange.response.body().len(),
                    body: String::from_utf8_lossy(exchange.response.body()).to_string(),
                },
            })
            .collect(),
    };
    println!("{}", serde_json::to_string(&bundle).unwrap());
}

fn list_debug(bundle: &Bundle) {
    println!("{:#?}", bundle);
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

#[tokio::main]
async fn main() -> Result<()> {
    env_logger_init();
    let args = Cli::from_args();
    match args.cmd {
        Command::Create {
            base_url,
            primary_url,
            file,
            resources_dir,
        } => {
            let mut builder = Bundle::builder()
                .version(Version::VersionB2)
                .exchanges_from_dir(resources_dir, base_url.parse()?)
                .await?;
            if let Some(primary_url) = primary_url {
                builder = builder.primary_url(primary_url.parse()?);
            }
            let bundle = builder.build()?;
            log::debug!("{:#?}", bundle);
            let write = BufWriter::new(File::create(&file)?);
            bundle.write_to(write)?;
        }
        Command::List { file, format } => {
            let mut buf = Vec::new();
            File::open(&file)?.read_to_end(&mut buf)?;
            let bundle = Bundle::from_bytes(buf)?;
            list(&bundle, format);
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
