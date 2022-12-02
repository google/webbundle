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
use http::StatusCode;
pub use http::Uri;

use headers::{ContentLength, ContentType, HeaderMapExt as _};

use std::convert::TryFrom;
use std::io::Write;
use std::path::Path;

pub type Body = Vec<u8>;
pub type Response = http::Response<Body>;
pub type HeaderMap = http::header::HeaderMap;

/// Represents a HTTP exchange's request.
///
/// This is different from `http::request::Request` because
/// a resource's URL in Web Bundle can be a relative URL, eg. "./foo.html".
/// `http::request::Request` requires Uri, which can not be a relative URL.
#[derive(Debug, Clone)]
pub struct Request {
    url: String,
    headers: HeaderMap,
}

impl Request {
    /// Creates a new `Request` with the given url and headers.
    pub fn new(url: String, headers: HeaderMap) -> Request {
        Request { url, headers }
    }

    /// Returns a reference to the associated url.
    pub fn url(&self) -> &String {
        &self.url
    }

    /// Returns a reference to the associated header field map.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
}

impl From<(String, HeaderMap)> for Request {
    fn from((url, headers): (String, HeaderMap)) -> Self {
        Self::new(url, headers)
    }
}

impl From<String> for Request {
    fn from(url: String) -> Self {
        Self::new(url, HeaderMap::new())
    }
}

// TODO: Use TryFrom?
impl From<&Path> for Request {
    fn from(path: &Path) -> Self {
        // path.display().to_string() can't be used because
        // that may contain a backslash, `\\`, in Windows.
        let url = path
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>()
            .join("/");
        Self::new(url, HeaderMap::new())
    }
}

pub const HEADER_MAGIC_BYTES: [u8; 8] = [0xf0, 0x9f, 0x8c, 0x90, 0xf0, 0x9f, 0x93, 0xa6];
pub(crate) const VERSION_BYTES_LEN: usize = 4;
pub(crate) const TOP_ARRAY_LEN: usize = 5;
pub(crate) const KNOWN_SECTION_NAMES: [&str; 4] = ["index", "critical", "responses", "primary"];

/// Represents the version of WebBundle.
#[derive(Debug, PartialEq, Eq)]
pub enum Version {
    /// Version b2, which is used in Google Chrome
    VersionB2,
    /// Version 1
    Version1,
    /// Unknown version
    Unknown([u8; 4]),
}

impl Version {
    /// Gets the bytes which represents this version.
    pub fn bytes(&self) -> &[u8; 4] {
        match self {
            Version::VersionB2 => &[0x62, 0x32, 0, 0],
            Version::Version1 => &[0x31, 0, 0, 0],
            Version::Unknown(a) => a,
        }
    }
}

/// Represents an HTTP exchange, a pair of a request and a response.
#[derive(Debug)]
pub struct Exchange {
    pub request: Request,
    pub response: Response,
}

impl Clone for Exchange {
    fn clone(&self) -> Self {
        Exchange {
            request: self.request.clone(),
            response: {
                let mut response = Response::new(self.response.body().clone());
                *response.status_mut() = self.response.status();
                *response.headers_mut() = self.response.headers().clone();
                response
            },
        }
    }
}

impl<T> From<(T, Vec<u8>, ContentType)> for Exchange
where
    T: Into<Request>,
{
    fn from((request, body, content_type): (T, Vec<u8>, ContentType)) -> Self {
        let request: Request = request.into();
        let response = {
            let content_length = ContentLength(body.len() as u64);
            let mut response = Response::new(body);
            *response.status_mut() = StatusCode::OK;
            response.headers_mut().typed_insert(content_length);
            response.headers_mut().typed_insert(content_type);
            response
        };
        Exchange { request, response }
    }
}

impl<T> From<(T, Vec<u8>)> for Exchange
where
    T: Into<Request>,
{
    fn from((request, body): (T, Vec<u8>)) -> Self {
        let request: Request = request.into();
        let content_type =
            ContentType::from(mime_guess::from_path(&request.url).first_or_octet_stream());
        (request, body, content_type).into()
    }
}

/// Represents a WebBundle.
#[derive(Debug)]
pub struct Bundle {
    pub(crate) version: Version,
    pub(crate) primary_url: Option<Uri>,
    pub(crate) exchanges: Vec<Exchange>,
}

impl Bundle {
    /// Gets the version.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Gets the primary url.
    pub fn primary_url(&self) -> &Option<Uri> {
        &self.primary_url
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
        encoder::encode(self, write)
    }

    /// Encodes this bundle.
    pub fn encode(&self) -> Result<Vec<u8>> {
        encoder::encode_to_vec(self)
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

#[cfg(test)]
mod tests {
    use super::*;
    use headers::ContentType;

    #[test]
    fn request_from_path() {
        let path = Path::new("foo/bar");
        let request: Request = path.into();
        assert_eq!(request.url(), "foo/bar");

        let path_str = format!("foo{}bar", std::path::MAIN_SEPARATOR);
        let path = Path::new(&path_str);
        let request: Request = path.into();
        assert_eq!(request.url(), "foo/bar");
    }

    #[test]
    fn exchange_from() {
        let exchange = Exchange::from(("index.html".to_string(), "hello".to_string().into_bytes()));
        assert_eq!(exchange.request.url(), "index.html");
        assert_eq!(exchange.response.body(), b"hello");
        assert_eq!(
            exchange.response.headers().typed_get::<ContentType>(),
            Some(ContentType::html())
        );
    }

    #[test]
    fn exchange_from_with_content_type() {
        let exchange = Exchange::from(("./foo/".to_string(), vec![], ContentType::html()));
        assert_eq!(exchange.request.url(), "./foo/");
        assert_eq!(exchange.response.body(), &[]);
        assert_eq!(
            exchange.response.headers().typed_get::<ContentType>(),
            Some(ContentType::html())
        );
    }
}
