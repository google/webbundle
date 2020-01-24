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
    #[structopt(name = "dump")]
    Dump {
        #[structopt(short = "i", long = "input")]
        input: String,
    },
}

fn main() -> Result<()> {
    let args = Cli::from_args();
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
    match args.cmd {
        Command::Dump { input } => {
            let mut buf = Vec::new();
            std::fs::File::open(&input)?.read_to_end(&mut buf)?;
            let bundle = webbundle::Bundle::parse(buf)?;
            println!("{:#?}", bundle);
        }
    }
    Ok(())
}
