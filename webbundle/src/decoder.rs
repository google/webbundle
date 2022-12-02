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

use crate::bundle::{self, Bundle, Exchange, Request, Response, Uri, Version};
use crate::prelude::*;
use cbor_event::Len;
use http::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};
use std::collections::HashSet;
use std::convert::TryInto;
use std::io::Cursor;

pub(crate) fn parse(bytes: impl AsRef<[u8]>) -> Result<Bundle> {
    Decoder::new(bytes).decode()
}

#[derive(Debug)]
struct SectionOffset {
    name: String,
    offset: u64,
    length: u64,
}

#[derive(Debug)]
struct ResponseLocation {
    offset: u64,
    length: u64,
}

impl ResponseLocation {
    pub fn new(responses_section_offset: u64, offset: u64, length: u64) -> ResponseLocation {
        ResponseLocation {
            offset: responses_section_offset + offset,
            length,
        }
    }
}

#[derive(Debug)]
struct RequestEntry {
    request: Request,
    response_location: ResponseLocation,
}

#[derive(Debug)]
struct Metadata {
    version: Version,
    section_offsets: Vec<SectionOffset>,
}

type Deserializer<R> = cbor_event::de::Deserializer<R>;

struct Decoder<T> {
    de: Deserializer<Cursor<T>>,
}

impl<T> Decoder<T> {
    fn new(buf: T) -> Self {
        Decoder {
            de: Deserializer::from(Cursor::new(buf)),
        }
    }
}

type PrimaryUrl = Uri;

impl<T: AsRef<[u8]>> Decoder<T> {
    fn decode(&mut self) -> Result<Bundle> {
        let metadata = self.read_metadata()?;
        log::debug!("metadata {:?}", metadata);

        let (requests, primary_url) = self.read_sections(&metadata.section_offsets)?;
        let exchanges = self.read_responses(requests)?;

        Ok(Bundle {
            version: metadata.version,
            primary_url,
            exchanges,
        })
    }

    fn read_metadata(&mut self) -> Result<Metadata> {
        ensure!(
            self.read_array_len()? as usize == bundle::TOP_ARRAY_LEN,
            "Invalid header"
        );
        self.read_magic_bytes()?;
        let version = self.read_version()?;
        let section_offsets = self.read_section_offsets()?;
        Ok(Metadata {
            version,
            section_offsets,
        })
    }

    fn read_magic_bytes(&mut self) -> Result<()> {
        log::debug!("read_magic_bytes");
        let magic: Vec<u8> = self.de.bytes().context("Invalid magic bytes")?;
        ensure!(magic == bundle::HEADER_MAGIC_BYTES, "Header magic mismatch");
        Ok(())
    }

    fn read_version(&mut self) -> Result<Version> {
        log::debug!("read_version");
        let bytes: Vec<u8> = self.de.bytes().context("Invalid version format")?;
        ensure!(
            bytes.len() == bundle::VERSION_BYTES_LEN,
            "Invalid version format"
        );
        let version: [u8; bundle::VERSION_BYTES_LEN] =
            AsRef::<[u8]>::as_ref(&bytes).try_into().unwrap();
        Ok(if &version == bundle::Version::Version1.bytes() {
            Version::Version1
        } else if &version == bundle::Version::VersionB2.bytes() {
            Version::VersionB2
        } else {
            Version::Unknown(version)
        })
    }

    fn read_section_offsets(&mut self) -> Result<Vec<SectionOffset>> {
        let bytes = self
            .de
            .bytes()
            .context("Failed to read sectionLength byte string")?;
        ensure!(
            bytes.len() < 8_192,
            format!("sectionLengthsLength is too long ({} bytes)", bytes.len())
        );
        Decoder::new(bytes).read_section_offsets_cbor(self.position())
    }

    fn read_array_len(&mut self) -> Result<u64> {
        match self.de.array()? {
            Len::Len(n) => Ok(n),
            _ => bail!("bundle: bundle: Failed to decode sectionOffset array header"),
        }
    }

    fn position(&self) -> u64 {
        self.de.as_ref().position()
    }

    fn read_section_offsets_cbor(&mut self, mut offset: u64) -> Result<Vec<SectionOffset>> {
        let n = self
            .read_array_len()
            .context("bundle: bundle: Failed to decode sectionOffset array header")?;
        let section_num = n / 2;
        offset += self.position();
        let mut seen_names = HashSet::new();
        let mut section_offsets = Vec::with_capacity(section_num as usize);
        for _ in 0..section_num {
            let name = self.de.text()?;
            ensure!(!seen_names.contains(&name), "Duplicate section name");
            seen_names.insert(name.clone());
            let length = self.de.unsigned_integer()?;
            section_offsets.push(SectionOffset {
                name,
                offset,
                length,
            });
            offset += length;
        }
        ensure!(!section_offsets.is_empty(), "bundle: section is empty");
        ensure!(
            section_offsets.last().unwrap().name == "responses",
            "bundle: Last section is not \"responses\""
        );
        Ok(section_offsets)
    }

    fn inner_buf(&self) -> &[u8] {
        self.de.as_ref().get_ref().as_ref()
    }

    fn new_decoder_from_range(&self, start: u64, end: u64) -> Decoder<&[u8]> {
        // TODO: Check range, instead of panic
        Decoder::new(&self.inner_buf()[start as usize..end as usize])
    }

    fn read_sections(
        &mut self,
        section_offsets: &[SectionOffset],
    ) -> Result<(Vec<RequestEntry>, Option<PrimaryUrl>)> {
        log::debug!("read_sections");
        let n = self
            .read_array_len()
            .context("Failed to read section header")?;
        log::debug!("n: {:?}", n);
        ensure!(
            n as usize == section_offsets.len(),
            format!(
                "bundle: Expected {} sections, got {} sections",
                section_offsets.len(),
                n
            )
        );

        let responses_section_offset = section_offsets.last().unwrap().offset;

        let mut requests = vec![];
        let mut primary_url: Option<PrimaryUrl> = None;

        for SectionOffset {
            name,
            offset,
            length,
        } in section_offsets
        {
            if !bundle::KNOWN_SECTION_NAMES.iter().any(|&n| n == name) {
                log::warn!("Unknows section name: {}. Skipping", name);
                continue;
            }
            let mut section_decoder = self.new_decoder_from_range(*offset, offset + length);

            // TODO: Support ignoredSections
            match name.as_ref() {
                "index" => {
                    requests = section_decoder.read_index(responses_section_offset)?;
                }
                "responses" => {
                    // Skip responses section becuase we read responses later.
                }
                "primary" => {
                    primary_url = Some(section_decoder.read_primary_url()?);
                }
                _ => {
                    log::warn!("Unknown section found: {}", name);
                }
            }
        }
        Ok((requests, primary_url))
    }

    fn read_primary_url(&mut self) -> Result<PrimaryUrl> {
        log::debug!("read_primary_url");
        self.de
            .text()
            .context("bundle: Failed to read primary_url string")?
            .parse()
            .context("Failed to parse primary_url")
    }

    fn read_index(&mut self, responses_section_offset: u64) -> Result<Vec<RequestEntry>> {
        let index_map_len = match self.de.map()? {
            Len::Len(n) => n,
            Len::Indefinite => {
                bail!("bundle: Failed to decode index section map header");
            }
        };
        // dbg!(index_map_len);

        let mut requests = vec![];
        for _ in 0..index_map_len {
            // TODO: support relative URL, which can not be Uri.
            let url = self.de.text()?;
            ensure!(
                self.read_array_len()? == 2,
                "bundle: Failed to decode index item"
            );
            let offset = self.de.unsigned_integer()?;
            let length = self.de.unsigned_integer()?;
            requests.push(RequestEntry {
                request: url.into(),
                response_location: ResponseLocation::new(responses_section_offset, offset, length),
            });
        }
        Ok(requests)
    }

    fn read_responses(&mut self, requests: Vec<RequestEntry>) -> Result<Vec<Exchange>> {
        requests
            .into_iter()
            .map(
                |RequestEntry {
                     request,
                     response_location: ResponseLocation { offset, length },
                 }| {
                    let response = self
                        .new_decoder_from_range(offset, offset + length)
                        .read_response()?;
                    Ok(Exchange { request, response })
                },
            )
            .collect()
    }

    fn read_response(&mut self) -> Result<Response> {
        let responses_array_len = self
            .read_array_len()
            .context("bundle: Failed to decode responses section array headder")?;
        ensure!(
            responses_array_len == 2,
            "bundle: Failed to decode response entry"
        );
        log::debug!("read_response: headers byte 1");
        let headers = self.de.bytes()?;
        log::debug!("read_response: headers byte 2");
        let mut nested = Decoder::new(headers);
        let (status, headers) = nested.read_headers_cbor()?;
        let body = self.de.bytes()?;
        let mut response = Response::new(body);
        *response.status_mut() = status;
        *response.headers_mut() = headers;
        Ok(response)
    }

    fn read_headers_cbor(&mut self) -> Result<(StatusCode, HeaderMap)> {
        let headers_map_len = match self.de.map()? {
            Len::Len(n) => n,
            Len::Indefinite => {
                bail!("bundle: Failed to decode responses headers map headder");
            }
        };
        let mut headers = HeaderMap::new();
        let mut status = None;
        for _ in 0..headers_map_len {
            let name = String::from_utf8(self.de.bytes()?)?;
            let value = String::from_utf8(self.de.bytes()?)?;
            if name.starts_with(':') {
                ensure!(name == ":status", "Unknown pseudo headers");
                ensure!(status.is_none(), ":status is duplicated");
                status = Some(value.parse()?);
                continue;
            }
            headers.insert(
                HeaderName::from_lowercase(name.as_bytes())?,
                HeaderValue::from_str(value.as_str())?,
            );
        }
        ensure!(status.is_some(), "no :status header");
        Ok((status.unwrap(), headers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{Bundle, Version};

    #[test]
    fn encode_and_decode() -> Result<()> {
        let bundle = Bundle::builder()
            .version(Version::VersionB2)
            .primary_url("https://example.com/index.html".parse()?)
            .exchange(Exchange::from((
                "https://example.com/index.html".to_string(),
                vec![],
            )))
            .build()?;

        let encoded = bundle.encode()?;

        // Decode encoded bundle.
        let bundle = Bundle::from_bytes(encoded)?;
        assert_eq!(bundle.version(), &Version::VersionB2);
        assert_eq!(
            bundle.primary_url(),
            &Some("https://example.com/index.html".parse()?)
        );
        assert_eq!(bundle.exchanges().len(), 1);
        assert_eq!(
            bundle.exchanges()[0].request.url(),
            "https://example.com/index.html"
        );
        assert_eq!(bundle.exchanges()[0].response.body(), &[]);
        Ok(())
    }

    /// This test uses an external tool, `gen-bundle`.
    /// See https://github.com/WICG/webpackage/go/bundle
    #[ignore]
    #[test]
    fn decode_bundle_encoded_by_go_gen_bundle() -> Result<()> {
        use std::io::Read;

        let base_dir = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("tests/builder");
            path
        };

        let mut file = tempfile::NamedTempFile::new()?;

        // Create a bundle by `gen-bundle`.
        let res = std::process::Command::new("gen-bundle")
            .arg("--version")
            .arg("b2")
            .arg("-dir")
            .arg(base_dir)
            .arg("-baseURL")
            .arg("https://example.com/")
            .arg("-o")
            .arg(file.path())
            .output()?;
        assert!(res.status.success());

        // Parse the created bundle.
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let bundle = Bundle::from_bytes(bytes)?;

        assert_eq!(bundle.version, Version::VersionB2);
        assert_eq!(bundle.exchanges.len(), 3);
        assert_eq!(bundle.exchanges[0].request.url(), "https://example.com/");
        assert_eq!(
            bundle.exchanges[1].request.url(),
            "https://example.com/index.html"
        );
        assert_eq!(
            bundle.exchanges[2].request.url(),
            "https://example.com/js/hello.js"
        );
        Ok(())
    }
}
