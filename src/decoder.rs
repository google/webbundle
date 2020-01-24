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
use cbor_event::Len;
use std::collections::HashSet;
use std::convert::TryInto;
use std::io::{prelude::*, Cursor};
use std::str::FromStr;

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
    primary_url: Url,
    section_offsets: Vec<SectionOffset>,
    requests: Vec<RequestEntry>,
    manifest: Option<Manifest>,
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

const HEADER_MAGIC_BYTES: [u8; 10] = [0x86, 0x48, 0xf0, 0x9f, 0x8c, 0x90, 0xf0, 0x9f, 0x93, 0xa6];
const VERSION_1: [u8; 4] = [0x31, 0, 0, 0];
const KNOWN_SECTION_NAMES: [&str; 5] = ["index", "manifest", "signatures", "critical", "responses"];

type Manifest = Url;

fn check_url(url: &Url) -> Result<()> {
    ensure!(
        url.fragment().is_none(),
        format!("Url contains fragment: {}", url)
    );
    ensure!(
        url.username().is_empty(),
        format!("Url contains credentials: {}", url)
    );
    Ok(())
}

impl<T: AsRef<[u8]>> Decoder<T> {
    fn decode(&mut self) -> Result<Bundle> {
        let metadata = self.read_metadata()?;
        Ok(Bundle {
            version: metadata.version,
            primary_url: metadata.primary_url,
            exchanges: self.read_responses(metadata.requests)?,
            manifest: metadata.manifest,
        })
    }

    fn read_metadata(&mut self) -> Result<Metadata> {
        self.read_magic_bytes()?;
        let version = self.read_version()?;
        let primary_url = self.read_primary_url()?;
        let section_offsets = self.read_section_offsets()?;
        let (requests, manifest) = self.read_sections(&section_offsets)?;
        Ok(Metadata {
            version,
            primary_url,
            section_offsets,
            requests,
            manifest,
        })
    }

    fn read_magic_bytes(&mut self) -> Result<()> {
        log::debug!("read_magic_bytes");
        let mut buf = [0; HEADER_MAGIC_BYTES.len()];
        self.de.as_mut_ref().read_exact(&mut buf[..])?;
        anyhow::ensure!(buf == HEADER_MAGIC_BYTES, "Header magic mismatch");
        Ok(())
    }

    fn read_version(&mut self) -> Result<Version> {
        log::debug!("read_version");
        let bytes: Vec<u8> = self.de.bytes().context("Invalid version format")?;
        anyhow::ensure!(bytes.len() == VERSION_1.len(), "Invalid version format");
        let version: [u8; VERSION_1.len()] = AsRef::<[u8]>::as_ref(&bytes).try_into().unwrap();
        Ok(if version == VERSION_1 {
            Version::Version1
        } else {
            Version::Unknown(version)
        })
    }

    fn read_primary_url(&mut self) -> Result<Url> {
        log::debug!("read_primary_url");
        let url: String = self
            .de
            .text()
            .context("bundle: Failed to read primary_url string")?;
        Ok(Url::from_str(&url).context("Failed to parse primary_url")?)
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
        debug!("offset: {}", offset);
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
    ) -> Result<(Vec<RequestEntry>, Option<Manifest>)> {
        let n = self
            .read_array_len()
            .context("Failed to read section header")?;
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
        let mut manifest: Option<Manifest> = None;

        for SectionOffset {
            name,
            offset,
            length,
        } in section_offsets
        {
            if !KNOWN_SECTION_NAMES.iter().any(|&n| n == name) {
                warn!("Unknows section name: {}. Skipping", name);
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
                "manifest" => {
                    manifest = Some(section_decoder.read_manifest()?);
                }
                "signatures" => {
                    warn!("signatues section is not supported yet");
                }
                _ => {
                    warn!("Unknown section found: {}", name);
                }
            }
        }
        Ok((requests, manifest))
    }

    fn read_manifest(&mut self) -> Result<Url> {
        debug!("read_manifest");
        let url = self.de.text()?;
        let url = Url::parse(&url)?;
        Ok(url)
    }

    fn read_index(&mut self, responses_section_offset: u64) -> Result<Vec<RequestEntry>> {
        debug!("read_index");
        let index_map_len = match self.de.map()? {
            Len::Len(n) => n,
            Len::Indefinite => {
                bail!("bundle: Failed to decode index section map header");
            }
        };

        let mut requests = vec![];
        for _ in 0..index_map_len {
            let url = self.de.text()?;
            let url = Url::parse(&url)?;
            check_url(&url)?;

            let value_array_len = match self.de.array()? {
                Len::Len(0) => {
                    bail!("bundle: Failed to decode index section. value array is empty");
                }
                Len::Len(n) => n,
                Len::Indefinite => {
                    bail!("bundle: Failed to decode index section value array headder");
                }
            };

            let variant_value = self.de.bytes()?;
            if variant_value.is_empty() {
                ensure!(
                    value_array_len == 3,
                    "bundle: The size of value array must be 3"
                );
                let offset = self.de.unsigned_integer()?;
                let length = self.de.unsigned_integer()?;
                requests.push(RequestEntry {
                    request: Request {
                        url,
                        variant_key: None,
                    },
                    response_location: ResponseLocation::new(
                        responses_section_offset,
                        offset,
                        length,
                    ),
                });
            } else {
                requests.extend(
                    (0..(value_array_len - 1) / 2)
                        .map(|_| {
                            let offset = self.de.unsigned_integer()?;
                            let length = self.de.unsigned_integer()?;
                            Ok(RequestEntry {
                                request: Request {
                                    url: url.clone(),
                                    // TODO: Parse variants value, and set each parsed value.
                                    variant_key: None,
                                },
                                response_location: ResponseLocation::new(
                                    responses_section_offset,
                                    offset,
                                    length,
                                ),
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                );
            }
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
        let headers = self.de.bytes()?;
        let mut nested = Decoder::new(headers);
        let headers = nested.read_headers_cbor()?;
        let body = self.de.bytes()?;
        Ok(Response { headers, body })
    }

    fn read_headers_cbor(&mut self) -> Result<Headers> {
        let headers_map_len = match self.de.map()? {
            Len::Len(n) => n,
            Len::Indefinite => {
                bail!("bundle: Failed to decode responses headers map headder");
            }
        };
        (0..headers_map_len)
            .map(|_| {
                let name = String::from_utf8(self.de.bytes()?)?;
                let value = String::from_utf8(self.de.bytes()?)?;
                ensure!(
                    !name.chars().any(|c| c.is_uppercase()),
                    format!(
                        "Failed to decode response headers: name contains upper-case: {}",
                        name
                    )
                );
                ensure!(
                    name.is_ascii(),
                    format!(
                        "Failed to decode response headers: name contains non-ASCII: {}",
                        name
                    )
                );
                // TODO: Support pseudo values, such as :status
                Ok((name, value))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_env_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn read_magic_test() -> Result<()> {
        init_env_logger();
        assert!(Decoder::new(HEADER_MAGIC_BYTES).read_magic_bytes().is_ok());
        assert!(Decoder::new([]).read_magic_bytes().is_err());
        Ok(())
    }
}
