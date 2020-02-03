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

use crate::builder::Builder;
use crate::decoder;
use crate::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;
pub use url::Url;

/// Represents the version of WebBundle.
#[derive(Debug, PartialEq, Eq)]
pub enum Version {
    /// Version 1; [0x31, 0, 0, 0]
    Version1,
    /// Unknows version
    Unknown([u8; 4]),
}

/// Represents an HTTP request.
#[derive(Debug)]
pub struct Request {
    pub url: Url,
    pub variant_key: Option<String>,
}

pub type Headers = HashMap<String, String>;

/// Represents an HTTP response.
pub struct Response {
    // TODO: Support status
    // pub status: u32;
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl Response {
    pub fn body_as_utf8_lossy_string(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.body[..])
    }
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("headers", &self.headers)
            .field("body_as_utf8_lossy", &self.body_as_utf8_lossy_string())
            .finish()
    }
}

/// Represents an HTTP exchange, a pair of a request and a response.
#[derive(Debug)]
pub struct Exchange {
    pub request: Request,
    pub response: Response,
}

/// Represents a WebBundle.
#[derive(Debug)]
pub struct Bundle {
    pub version: Version,
    pub primary_url: Url,
    pub manifest: Option<Url>,
    pub exchanges: Vec<Exchange>,
}

impl Bundle {
    /// Parses a Bundle from bytes
    // We can't use TryFrom, due to https://github.com/rust-lang/rust/issues/50133
    pub fn parse(bytes: impl AsRef<[u8]>) -> Result<Bundle> {
        decoder::parse(bytes)
    }

    pub fn builder() -> Builder {
        Builder::new()
    }
}
