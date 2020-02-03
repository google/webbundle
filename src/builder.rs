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

use crate::bundle::{Bundle, Exchange, Headers, Request, Response, Url, Version};
use crate::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Builder {
    version: Option<Version>,
    primary_url: Option<Url>,
    manifest: Option<Url>,
    exchanges: Vec<Exchange>,
}

impl Builder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    pub fn primary_url(mut self, primary_url: Url) -> Self {
        self.primary_url = Some(primary_url);
        self
    }

    pub fn build(self) -> Result<Bundle> {
        Ok(Bundle {
            version: self.version.context("no version")?,
            primary_url: self.primary_url.context("no primary_url")?,
            manifest: self.manifest,
            exchanges: self.exchanges,
        })
    }
}

#[allow(dead_code)]
pub struct ExchangeBuilder {
    base_url: Url,
    base_dir: PathBuf,
    exchanges: Vec<Exchange>,
}

#[allow(dead_code)]
impl ExchangeBuilder {
    fn new(base_url: Url, base_dir: PathBuf) -> Self {
        ExchangeBuilder {
            base_url,
            base_dir,
            exchanges: Vec::new(),
        }
    }

    fn url_from_relative_path(&self, relative_path: &Path) -> Result<Url> {
        ensure!(
            relative_path.is_relative(),
            format!("Path is not relative: {}", relative_path.display())
        );
        Ok(self.base_url.join(&relative_path.display().to_string())?)
    }

    fn exchange(mut self, relative_path: impl AsRef<Path>) -> Result<Self> {
        self.exchanges.push(Exchange {
            request: Request {
                url: self.url_from_relative_path(relative_path.as_ref())?,
                variant_key: None,
            },
            response: self.create_response(relative_path)?,
        });
        Ok(self)
    }

    fn create_response(&self, relative_path: impl AsRef<Path>) -> Result<Response> {
        ensure!(
            relative_path.as_ref().is_relative(),
            format!("Path is not relative: {}", relative_path.as_ref().display())
        );
        let path = self.base_dir.join(relative_path);
        let mime = mime_guess::from_path(path.clone()).first_or_octet_stream();
        let mime = format!("{}/{}", mime.type_(), mime.subtype());

        // TODO: We should have a async version
        let body = std::fs::read(path)?;

        let mut headers = Headers::new();
        headers.insert("content-type".to_string(), mime);
        headers.insert("content-length".to_string(), body.len().to_string());
        // TODO: Don't use status pseudo header.
        headers.insert(":status".to_string(), 200.to_string());

        // TODO: Add date to headers?

        Ok(Response { headers, body })
    }

    fn build(self) -> Vec<Exchange> {
        self.exchanges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_fail() {
        assert!(Builder::new().build().is_err());
    }

    #[test]
    fn build() -> Result<()> {
        let bundle = Builder::new()
            .version(Version::Version1)
            .primary_url(Url::parse("https://example.com")?)
            .build()?;
        assert_eq!(bundle.version, Version::Version1);
        assert_eq!(bundle.primary_url, Url::parse("https://example.com")?);
        Ok(())
    }

    #[test]
    fn exchange_builder() -> Result<()> {
        let base_dir = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests/builder");
            path
        };

        let exchanges = ExchangeBuilder::new(Url::parse("https://example.com/")?, base_dir.clone())
            .exchange("index.html")?
            .build();
        assert_eq!(exchanges.len(), 1);
        let exchange = &exchanges[0];
        assert_eq!(
            exchange.request.url,
            Url::parse("https://example.com/index.html")?
        );
        assert!(exchange.request.variant_key.is_none());
        assert_eq!(exchange.response.headers["content-type"], "text/html");
        assert_eq!(
            exchange.response.headers["content-length"],
            std::fs::read(base_dir.join("index.html"))?
                .len()
                .to_string()
        );
        assert_eq!(
            exchange.response.body,
            std::fs::read(base_dir.join("index.html"))?
        );
        Ok(())
    }
}
