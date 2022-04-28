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

use crate::bundle::{Bundle, Exchange, Uri, Version};
use crate::prelude::*;

#[cfg(feature = "fs")]
use crate::fs::builder::ExchangeBuilder;
#[cfg(feature = "fs")]
use std::path::{Path, PathBuf};

/// A Bundle builder.
#[derive(Default)]
pub struct Builder {
    version: Option<Version>,
    primary_url: Option<Uri>,
    manifest: Option<Uri>,
    exchanges: Vec<Exchange>,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Sets the version.
    pub fn version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the primary url.
    pub fn primary_url(mut self, primary_url: Uri) -> Self {
        self.primary_url = Some(primary_url);
        self
    }

    /// Sets the manifest url.
    pub fn manifest(mut self, manifest: Uri) -> Self {
        self.manifest = Some(manifest);
        self
    }

    /// Adds the exchange.
    pub fn exchange(mut self, exchange: Exchange) -> Self {
        self.exchanges.push(exchange);
        self
    }

    /// Append exchanges from files rooted at the given directory.
    ///
    /// One exchange is created for each file, however, two exchanges
    /// are created for `index.html` file, as follows:
    ///
    /// 1. The pareent directory **serves** the contents of `index.html` file.
    /// 2. The URL for `index.html` file is a redirect to the parent directory
    ///    (`301` MOVED PERMANENTLY).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async {
    /// use webbundle::{Bundle, Version};
    /// let bundle = Bundle::builder()
    ///     .version(Version::VersionB2)
    ///     .exchanges_from_dir("build").await?
    ///     .build()?;
    /// # std::result::Result::Ok::<_, anyhow::Error>(bundle)
    /// # };
    /// ```
    #[cfg(feature = "fs")]
    pub async fn exchanges_from_dir(mut self, dir: impl AsRef<Path>) -> Result<Self> {
        self.exchanges.append(
            &mut ExchangeBuilder::new(PathBuf::from(dir.as_ref()))
                .walk()
                .await?
                .build(),
        );
        Ok(self)
    }

    /// Builds the bundle.
    pub fn build(self) -> Result<Bundle> {
        Ok(Bundle {
            version: self.version.context("no version")?,
            primary_url: self.primary_url,
            exchanges: self.exchanges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_invalid_bundle() -> Result<()> {
        assert!(Builder::new().build().is_err());
        assert!(Builder::new()
            .primary_url("https://example.com/".parse()?)
            .build()
            .is_err());
        Ok(())
    }

    #[test]
    fn build() -> Result<()> {
        let bundle = Builder::new()
            .version(Version::Version1)
            .primary_url("https://example.com".parse()?)
            .build()?;
        assert_eq!(bundle.version, Version::Version1);
        assert_eq!(
            bundle.primary_url,
            Some("https://example.com".parse::<Uri>()?)
        );
        Ok(())
    }
}
