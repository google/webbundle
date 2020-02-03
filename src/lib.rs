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
//! ## Parsing
//!
//! ```should_panic
//! use webbundle::{Bundle, Result};
//! use std::io::{Read as _};
//!
//! fn main() -> Result<()> {
//!     let mut bytes = Vec::new();
//!     std::fs::File::open("your_bundle.wbn")?.read_to_end(&mut bytes)?;
//!     let bundle = Bundle::parse(bytes)?;
//!     println!("parsed bundle: {:#?}", bundle);
//!     Ok(())
//! }
//! ```
//!
//! # Future plans:
//!
//! - Support Variants
//! - Support Signatures
//! - Generate WebBundle from various sources, statically or dynamically
//!

mod builder;
pub mod bundle;
mod decoder;
mod prelude;
pub use bundle::Bundle;
pub use prelude::Result;
