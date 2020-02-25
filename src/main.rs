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

use anyhow::Result;
use chrono::Local;
use std::io::{Read as _, Write as _};
use structopt::StructOpt;

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
    /// Example: webbundle dump ./example.wbn
    #[structopt(name = "dump")]
    Dump { file: String },
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
            let mut builder = webbundle::Bundle::builder()
                .version(webbundle::Version::VersionB1)
                .primary_url(primary_url.parse()?)
                .exchanges_from_dir(resources_dir, base_url.parse()?)?;
            if let Some(manifest) = manifest {
                builder = builder.manifest(manifest.parse()?);
            }
            let bundle = builder.build()?;
            log::debug!("{:#?}", bundle);
            let write = std::io::BufWriter::new(std::fs::File::create(&file)?);
            bundle.write_to(write)?;
        }
        Command::Dump { file } => {
            let mut buf = Vec::new();
            std::fs::File::open(&file)?.read_to_end(&mut buf)?;
            let bundle = webbundle::Bundle::from_bytes(buf)?;
            println!("{:#?}", bundle);
        }
    }
    Ok(())
}
