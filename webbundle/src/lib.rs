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
//! # async {
//! use webbundle::{Bundle, Version};
//!
//! // Create an empty bundle. See [`Builder`] for details.
//! let bundle = Bundle::builder()
//!     .version(Version::VersionB2)
//!     .build()?;
//! println!("Created bundle: {:#?}", bundle);
//! let write = std::io::BufWriter::new(std::fs::File::create("example.wbn")?);
//! bundle.write_to(write)?;
//! # Result::Ok::<(), anyhow::Error>(())
//! # };
//! ```
mod builder;
mod bundle;
mod decoder;
mod encoder;
mod prelude;
pub use builder::Builder;
pub use bundle::{Body, Bundle, Exchange, Request, Response, Uri, Version};
pub use prelude::Result;

#[cfg(feature = "fs")]
mod fs;
