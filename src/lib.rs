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

//! # WebBundle library
//!
//! `webbundle` is an experimental library for WebBundle format.
//!
//! # Example
//!
//! ## WebBundle Parsing
//!
//! ```no_run
//! use webbundle::Bundle;
//! use std::io::{Read as _};
//!
//! let mut bytes = Vec::new();
//! std::fs::File::open("example.wbn")?.read_to_end(&mut bytes)?;
//! let bundle = Bundle::from_bytes(bytes)?;
//! println!("Parsed bundle: {:#?}", bundle);
//! # Result::Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Creating a bundle from files
//!
//! ```no_run
//! use webbundle::{Bundle, Version};
//!
//! let bundle = Bundle::builder()
//!     .version(Version::VersionB1)
//!     .primary_url("https://example.com/index.html".parse()?)
//!     .exchanges_from_dir("assets", "https://example.com".parse()?)?
//!     .build()?;
//! println!("Created bundle: {:#?}", bundle);
//! let write = std::io::BufWriter::new(std::fs::File::create("example.wbn")?);
//! bundle.write_to(write)?;
//! # Result::Ok::<(), anyhow::Error>(())
//! ```
//! 
//! ## 'webbundle' command line tool
//! 
//! ### create
//! ```no_run
//! $ webbundle create -b "https://example.com/" -p "https://example.com/foo/index.html" example.wbn foo
//! ```
//! 
//! ### dump
//! ```no_run
//! $ webbundle dump ./example.wbn
//! ```
//! 

mod builder;
mod bundle;
mod decoder;
mod encoder;
mod prelude;
pub use builder::Builder;
pub use bundle::{Body, Bundle, Exchange, Request, Response, Uri, Version};
pub use prelude::Result;
