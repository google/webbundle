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

use crate::bundle::{Bundle, Exchange, Request, Response, Uri, Version};
use crate::prelude::*;
use headers::{ContentLength, ContentType, HeaderMapExt as _, HeaderValue};
use http::StatusCode;
use std::path::{Path, PathBuf};
use url::Url;
use walkdir::WalkDir;

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
    /// `base_url` will be used as a prefix for each resource. A relative path
    /// from the given directory to each file is appended to `base_url`.
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
    /// # use webbundle::{Bundle, Version};
    /// let bundle = Bundle::builder()
    ///     .version(Version::VersionB1)
    ///     .primary_url("https://example.com/".parse()?)
    ///     .exchanges_from_dir("build", "https://example.com".parse()?)?
    ///     .build()?;
    /// # Result::Ok::<(), anyhow::Error>(())
    /// ```
    pub fn exchanges_from_dir(mut self, dir: impl AsRef<Path>, base_url: Url) -> Result<Self> {
        self.exchanges.append(
            &mut ExchangeBuilder::new(PathBuf::from(dir.as_ref()), base_url)
                .walk()?
                .build(),
        );
        Ok(self)
    }

    /// Builds the bundle.
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
struct ExchangeBuilder {
    base_url: Url,
    base_dir: PathBuf,
    exchanges: Vec<Exchange>,
}

#[allow(dead_code)]
impl ExchangeBuilder {
    fn new(base_dir: PathBuf, base_url: Url) -> Self {
        ExchangeBuilder {
            base_dir,
            base_url,
            exchanges: Vec::new(),
        }
    }

    fn walk(mut self) -> Result<Self> {
        for entry in WalkDir::new(&self.base_dir) {
            let entry = entry?;
            log::info!("visit: {:?}", entry);
            let file_type = entry.file_type();
            if file_type.is_symlink() {
                log::warn!(
                    "path is symbolink link. Skipping. {}",
                    entry.path().display()
                );
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            if entry.path().file_name().unwrap() == "index.html" {
                let dir = entry.path().parent().unwrap();

                let relative_url = pathdiff::diff_paths(dir, &self.base_dir).unwrap();
                let relative_path = pathdiff::diff_paths(entry.path(), &self.base_dir).unwrap();
                // for <dir> -> Serves the contents of <dir>/index.html
                self = self.exchange(&relative_url, &relative_path)?;

                // for <dir>/index.html -> redirect to "./"
                self = self.exchange_redirect(&relative_path, "./")?;
            } else {
                let relative_path = pathdiff::diff_paths(entry.path(), &self.base_dir).unwrap();
                self = self.exchange(&relative_path, &relative_path)?;
            }
        }
        Ok(self)
    }

    fn build(self) -> Vec<Exchange> {
        self.exchanges
    }

    fn url_from_relative_path(&self, relative_path: &Path) -> Result<Uri> {
        ensure!(
            relative_path.is_relative(),
            format!("Path is not relative: {}", relative_path.display())
        );
        Ok(self
            .base_url
            .join(&relative_path.display().to_string())?
            .to_string()
            .parse()?)
    }

    fn url_join(&self, relative_url: &str) -> Result<Uri> {
        Ok(self.base_url.join(relative_url)?.to_string().parse()?)
    }

    fn exchange(
        mut self,
        relative_url: impl AsRef<Path>,
        relative_path: impl AsRef<Path>,
    ) -> Result<Self> {
        self.exchanges.push(Exchange {
            request: Request::get(self.url_from_relative_path(relative_url.as_ref())?).body(())?,
            response: self.create_response(relative_path)?,
        });
        Ok(self)
    }

    fn exchange_redirect(mut self, relative_url: &Path, location: &str) -> Result<Self> {
        self.exchanges.push(Exchange {
            request: Request::get(self.url_from_relative_path(relative_url)?).body(())?,
            response: Self::create_redirect(location)?,
        });
        Ok(self)
    }

    fn create_redirect(location: &str) -> Result<Response> {
        let mut response = Response::new(Vec::new());
        *response.status_mut() = StatusCode::MOVED_PERMANENTLY;
        response
            .headers_mut()
            .insert("Location", HeaderValue::from_str(location)?);
        Ok(response)
    }

    fn create_response(&self, relative_path: impl AsRef<Path>) -> Result<Response> {
        ensure!(
            relative_path.as_ref().is_relative(),
            format!("Path is not relative: {}", relative_path.as_ref().display())
        );
        let path = self.base_dir.join(relative_path);
        let body = std::fs::read(&path)?;

        let content_length = ContentLength(body.len() as u64);
        let content_type = ContentType::from(mime_guess::from_path(&path).first_or_octet_stream());

        let mut response = Response::new(body);
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().typed_insert(content_length);
        response.headers_mut().typed_insert(content_type);
        Ok(response)
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
        assert_eq!(bundle.primary_url, "https://example.com".parse::<Uri>()?);
        Ok(())
    }

    #[test]
    fn exchange_builder() -> Result<()> {
        let base_dir = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests/builder");
            path
        };

        let exchanges = ExchangeBuilder::new(base_dir.clone(), "https://example.com/".parse()?)
            .exchange(".", "index.html")?
            .build();
        assert_eq!(exchanges.len(), 1);
        let exchange = &exchanges[0];
        assert_eq!(
            exchange.request.uri(),
            &"https://example.com/".parse::<Uri>()?
        );
        assert_eq!(exchange.response.status(), StatusCode::OK);
        assert_eq!(exchange.response.headers()["content-type"], "text/html");
        assert_eq!(
            exchange.response.headers()["content-length"],
            std::fs::read(base_dir.join("index.html"))?
                .len()
                .to_string()
        );
        assert_eq!(
            exchange.response.body(),
            &std::fs::read(base_dir.join("index.html"))?
        );
        Ok(())
    }

    #[test]
    fn walk() -> Result<()> {
        let base_dir = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests/builder");
            path
        };

        let exchanges = ExchangeBuilder::new(base_dir, "https://example.com/".parse()?)
            .walk()?
            .build();
        assert_eq!(exchanges.len(), 3);

        let top_dir = find_exchange_by_uri(&exchanges, "https://example.com/")?;
        assert_eq!(top_dir.response.status(), StatusCode::OK);

        let index_html = find_exchange_by_uri(&exchanges, "https://example.com/index.html")?;
        assert_eq!(index_html.response.status(), StatusCode::MOVED_PERMANENTLY);

        let a_js = find_exchange_by_uri(&exchanges, "https://example.com/js/hello.js")?;
        assert_eq!(a_js.response.status(), StatusCode::OK);

        Ok(())
    }

    fn find_exchange_by_uri<'a>(exchanges: &'a [Exchange], uri: &str) -> Result<&'a Exchange> {
        exchanges
            .iter()
            .find(|e| e.request.uri() == uri)
            .context("not fouond")
    }
}
