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

use crate::bundle::{Bundle, Exchange, Url, Version};
use crate::prelude::*;

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
            .primary_url(Url::parse("http://example.com")?)
            .build()?;
        assert_eq!(bundle.version, Version::Version1);
        assert_eq!(bundle.primary_url, Url::parse("http://example.com")?);
        Ok(())
    }
}
