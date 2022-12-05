// Copyright 2021 Google LLC
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

use crate::bundle::{Exchange, Response};
use crate::prelude::*;
use headers::{ContentType, HeaderValue};
use http::StatusCode;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;
use walkdir::WalkDir;

impl crate::builder::Builder {
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
    pub async fn exchanges_from_dir(mut self, dir: impl AsRef<Path>) -> Result<Self> {
        self.exchanges.append(
            &mut ExchangeBuilder::new(PathBuf::from(dir.as_ref()))
                .walk()
                .await?
                .build(),
        );
        Ok(self)
    }

    /// Sync version of `exchanges_from_dir`.
    pub fn exchanges_from_dir_sync(mut self, dir: impl AsRef<Path>) -> Result<Self> {
        self.exchanges.append(
            &mut ExchangeBuilder::new(PathBuf::from(dir.as_ref()))
                .walk_sync()?
                .build(),
        );
        Ok(self)
    }
}

pub(crate) struct ExchangeBuilder {
    base_dir: PathBuf,
    exchanges: Vec<Exchange>,
}

// TODO: Refactor so that async and sync variants share more code.
impl ExchangeBuilder {
    pub fn new(base_dir: PathBuf) -> Self {
        ExchangeBuilder {
            base_dir,
            exchanges: Vec::new(),
        }
    }

    pub async fn walk(mut self) -> Result<Self> {
        // TODO: Walkdir is not async.
        for entry in WalkDir::new(&self.base_dir) {
            let entry = entry?;
            log::debug!("visit: {:?}", entry);
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
                self = self.exchange(&relative_url, &relative_path).await?;

                // for <dir>/index.html -> redirect to "./"
                self = self.exchange_redirect(&relative_path, "./")?;
            } else {
                let relative_path = pathdiff::diff_paths(entry.path(), &self.base_dir).unwrap();
                self = self.exchange(&relative_path, &relative_path).await?;
            }
        }
        Ok(self)
    }

    pub fn walk_sync(mut self) -> Result<Self> {
        for entry in WalkDir::new(&self.base_dir) {
            let entry = entry?;
            log::debug!("visit: {:?}", entry);
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
                self = self.exchange_sync(&relative_url, &relative_path)?;

                // for <dir>/index.html -> redirect to "./"
                self = self.exchange_redirect(&relative_path, "./")?;
            } else {
                let relative_path = pathdiff::diff_paths(entry.path(), &self.base_dir).unwrap();
                self = self.exchange_sync(&relative_path, &relative_path)?;
            }
        }
        Ok(self)
    }

    pub fn build(self) -> Vec<Exchange> {
        self.exchanges
    }

    pub async fn exchange(
        mut self,
        relative_url: impl AsRef<Path>,
        relative_path: impl AsRef<Path>,
    ) -> Result<Self> {
        self.exchanges.push(
            (
                relative_url.as_ref(),
                self.read_file(&relative_path).await?,
                ContentType::from(mime_guess::from_path(&relative_path).first_or_octet_stream()),
            )
                .into(),
        );
        Ok(self)
    }

    pub fn exchange_sync(
        mut self,
        relative_url: impl AsRef<Path>,
        relative_path: impl AsRef<Path>,
    ) -> Result<Self> {
        self.exchanges.push(
            (
                relative_url.as_ref(),
                self.read_file_sync(&relative_path)?,
                ContentType::from(mime_guess::from_path(&relative_path).first_or_octet_stream()),
            )
                .into(),
        );
        Ok(self)
    }

    fn exchange_redirect(mut self, relative_url: &Path, location: &str) -> Result<Self> {
        self.exchanges.push(Exchange {
            request: relative_url.display().to_string().into(),
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

    async fn read_file(&self, relative_path: impl AsRef<Path>) -> Result<Vec<u8>> {
        ensure!(
            relative_path.as_ref().is_relative(),
            format!("Path is not relative: {}", relative_path.as_ref().display())
        );
        let path = self.base_dir.join(relative_path);

        let mut file = tokio::io::BufReader::new(fs::File::open(&path).await?);
        let mut body = Vec::new();
        file.read_to_end(&mut body).await?;
        Ok(body)
    }

    fn read_file_sync(&self, relative_path: impl AsRef<Path>) -> Result<Vec<u8>> {
        use std::io::Read;

        ensure!(
            relative_path.as_ref().is_relative(),
            format!("Path is not relative: {}", relative_path.as_ref().display())
        );
        let path = self.base_dir.join(relative_path);

        let mut file = std::io::BufReader::new(std::fs::File::open(&path)?);
        let mut body = Vec::new();
        file.read_to_end(&mut body)?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{Bundle, Exchange, Version};

    #[tokio::test]
    async fn exchange_builder() -> Result<()> {
        let base_dir = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests/builder");
            path
        };

        let exchanges = ExchangeBuilder::new(base_dir.clone())
            .exchange(".", "index.html")
            .await?
            .build();
        assert_eq!(exchanges.len(), 1);
        let exchange = &exchanges[0];
        assert_eq!(exchange.request.url(), ".");
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

    #[tokio::test]
    async fn walk() -> Result<()> {
        let base_dir = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests/builder");
            path
        };

        let exchanges = ExchangeBuilder::new(base_dir).walk().await?.build();
        assert_eq!(exchanges.len(), 3);

        let top_dir = find_exchange_by_url(&exchanges, "")?;
        assert_eq!(top_dir.response.status(), StatusCode::OK);

        let index_html = find_exchange_by_url(&exchanges, "index.html")?;
        assert_eq!(index_html.response.status(), StatusCode::MOVED_PERMANENTLY);

        let a_js = find_exchange_by_url(&exchanges, "js/hello.js")?;
        assert_eq!(a_js.response.status(), StatusCode::OK);

        Ok(())
    }

    fn find_exchange_by_url<'a>(exchanges: &'a [Exchange], url: &str) -> Result<&'a Exchange> {
        exchanges
            .iter()
            .find(|e| e.request.url() == url)
            .context("not fouond")
    }

    /// This test uses an external tool, `dump-bundle`.
    /// See https://github.com/WICG/webpackage/go/bundle
    #[ignore]
    #[tokio::test]
    async fn encode_and_let_go_dump_bundle_decode_it() -> Result<()> {
        // Create a bundle.
        let base_dir = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests/builder");
            path
        };

        let bundle = Bundle::builder()
            .version(Version::VersionB2)
            .exchanges_from_dir(base_dir)
            .await?
            .build()?;

        let mut file = tempfile::NamedTempFile::new()?;
        bundle.write_to(&mut file)?;

        // Dump the created bundle by `dump-bundle`.
        let res = std::process::Command::new("dump-bundle")
            .arg("-i")
            .arg(file.path())
            .output()?;

        assert!(res.status.success(), "dump-bundle should read the bundle");
        Ok(())
    }
}
