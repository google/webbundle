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
use crate::encoder;
use crate::prelude::*;
pub use http::Uri;

use std::convert::TryFrom;
use std::io::Write;

pub type Body = Vec<u8>;

pub type Request = http::Request<()>;
pub type Response = http::Response<Body>;

pub const HEADER_MAGIC_BYTES: [u8; 8] = [0xf0, 0x9f, 0x8c, 0x90, 0xf0, 0x9f, 0x93, 0xa6];
pub(crate) const VERSION_BYTES_LEN: usize = 4;
pub(crate) const TOP_ARRAY_LEN: usize = 6;
pub(crate) const KNOWN_SECTION_NAMES: [&str; 5] =
    ["index", "manifest", "signatures", "critical", "responses"];

/// Represents the version of WebBundle.
#[derive(Debug, PartialEq, Eq)]
pub enum Version {
    /// Version b1, which is used in Google Chrome
    VersionB1,
    /// Version 1
    Version1,
    /// Unknown version
    Unknown([u8; 4]),
}

impl Version {
    /// Gets the bytes which represents this version.
    pub fn bytes(&self) -> &[u8; 4] {
        match self {
            Version::VersionB1 => &[0x62, 0x31, 0, 0],
            Version::Version1 => &[0x31, 0, 0, 0],
            Version::Unknown(a) => &a,
        }
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
    pub(crate) version: Version,
    pub(crate) primary_url: Uri,
    pub(crate) manifest: Option<Uri>,
    pub(crate) exchanges: Vec<Exchange>,
}

impl Bundle {
    /// Gets the version.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Gets the primary url.
    pub fn primary_url(&self) -> &Uri {
        &self.primary_url
    }

    /// Gets the manifest.
    pub fn manifest(&self) -> &Option<Uri> {
        &self.manifest
    }

    /// Gets the exchanges.
    pub fn exchanges(&self) -> &[Exchange] {
        &self.exchanges
    }

    /// Parses the given bytes and returns the parsed Bundle.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Bundle> {
        decoder::parse(bytes)
    }

    /// Encodes this bundle and write the result to the given `write`.
    pub fn write_to<W: Write + Sized>(&self, write: W) -> Result<()> {
        encoder::encode(&self, write)
    }

    /// Encodes this bundle.
    pub fn encode(&self) -> Result<Vec<u8>> {
        encoder::encode_to_vec(&self)
    }

    /// Returns a new builder.
    pub fn builder() -> Builder {
        Builder::new()
    }
}

impl<'a> TryFrom<&'a [u8]> for Bundle {
    type Error = anyhow::Error;

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        Bundle::from_bytes(bytes)
    }
}
