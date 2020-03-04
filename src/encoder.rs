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

use crate::bundle::{self, Bundle, Exchange, Response, Uri};
use crate::prelude::*;
use cbor_event::Len;
use std::io::Write;

use cbor_event::se::Serializer;

struct CountWrite<W> {
    count: usize,
    inner: W,
}

impl<W> CountWrite<W> {
    fn new(inner: W) -> Self {
        CountWrite { count: 0, inner }
    }
}

impl<W: Write> Write for CountWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.count += buf.len();
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub(crate) fn encode<W: Write + Sized>(bundle: &Bundle, write: W) -> Result<()> {
    Encoder::new(CountWrite::new(write)).encode(bundle)?;
    Ok(())
}

pub(crate) fn encode_to_vec(bundle: &Bundle) -> Result<Vec<u8>> {
    let mut write = Vec::new();
    encode(bundle, &mut write)?;
    Ok(write)
}

struct Encoder<W: Write> {
    se: Serializer<W>,
}

trait Count {
    fn count(&self) -> usize;
}

impl<W: Write> Count for Serializer<CountWrite<W>> {
    fn count(&self) -> usize {
        // Use unsafe because Serializer.0 is private.
        // TODO: Avoid to use unsafe.
        let se_ptr: *const Serializer<CountWrite<W>> = self;
        let count_write_ptr = se_ptr.cast::<CountWrite<W>>();
        unsafe { (*count_write_ptr).count }
    }
}

impl<W: Write> Encoder<W> {
    fn new(write: W) -> Self {
        Encoder {
            se: Serializer::new(write),
        }
    }

    fn write_magic(&mut self) -> Result<()> {
        self.se.write_bytes(bundle::HEADER_MAGIC_BYTES)?;
        Ok(())
    }

    fn write_version(&mut self, version: &bundle::Version) -> Result<()> {
        self.se.write_bytes(version.bytes())?;
        Ok(())
    }

    fn write_primary_url(&mut self, primary_url: &Uri) -> Result<()> {
        self.se.write_text(&primary_url.to_string())?;
        Ok(())
    }
}

impl<W: Write + Sized> Encoder<CountWrite<W>> {
    fn encode(&mut self, bundle: &Bundle) -> Result<()> {
        self.se
            .write_array(Len::Len(bundle::TOP_ARRAY_LEN as u64))?;
        self.write_magic()?;
        self.write_version(&bundle.version)?;
        self.write_primary_url(&bundle.primary_url)?;

        let sections = encode_sections(bundle)?;

        let section_length_cbor = encode_section_lengths(&sections)?;
        self.se.write_bytes(section_length_cbor)?;

        self.se.write_array(Len::Len(sections.len() as u64))?;
        for section in sections {
            self.se.write_raw_bytes(&section.bytes)?;
        }

        // Write the length of bytes
        self.se.write_unsigned_integer(self.se.count() as u64 + 8)?; // 8 is the length of u64.
        Ok(())
    }
}

struct Section {
    name: &'static str,
    bytes: Vec<u8>,
}

fn encode_sections(bundle: &Bundle) -> Result<Vec<Section>> {
    let mut sections = Vec::new();

    // manifest
    if let Some(uri) = &bundle.manifest {
        let bytes = encode_manifest_section(uri)?;
        sections.push(Section {
            name: "manifest",
            bytes,
        });
    };

    // responses
    let (response_section_bytes, response_locations) = encode_response_section(&bundle.exchanges)?;

    let response_section = Section {
        name: "responses",
        bytes: response_section_bytes,
    };

    // index from responses
    let index_section = Section {
        name: "index",
        bytes: encode_index_section(&response_locations)?,
    };

    sections.push(index_section);
    sections.push(response_section);
    Ok(sections)
}

fn encode_manifest_section(url: &Uri) -> Result<Vec<u8>> {
    let mut se = Serializer::new_vec();
    se.write_text(url.to_string())?;
    Ok(se.finalize())
}

struct ResponseLocation {
    uri: Uri,
    offset: usize,
    length: usize,
}

fn encode_response_section(exchanges: &[Exchange]) -> Result<(Vec<u8>, Vec<ResponseLocation>)> {
    let mut se = Serializer::new(CountWrite::new(Vec::new()));

    se.write_array(Len::Len(exchanges.len() as u64))?;

    let mut response_locations = Vec::new();

    for exchange in exchanges {
        let offset = se.count();

        se.write_array(Len::Len(2))?;
        se.write_bytes(&encode_headers(&exchange.response)?)?;
        se.write_bytes(&exchange.response.body())?;

        response_locations.push(ResponseLocation {
            uri: exchange.request.uri().clone(),
            offset,
            length: se.count() - offset,
        });
    }

    Ok((se.finalize().inner, response_locations))
}

fn encode_index_section(response_locations: &[ResponseLocation]) -> Result<Vec<u8>> {
    // Map keys must be sorted.
    // See [3.9. Canonical CBOR](https://tools.ietf.org/html/rfc7049#section-3.9)
    let mut map = std::collections::BTreeMap::<Vec<u8>, Vec<u8>>::new();

    for response_location in response_locations {
        let mut key = Serializer::new_vec();
        key.write_text(response_location.uri.to_string())?;

        let mut value = Serializer::new_vec();
        value.write_array(Len::Len(3))?;
        // TODO: Support variants.
        value.write_bytes(b"")?;
        value.write_unsigned_integer(response_location.offset as u64)?;
        value.write_unsigned_integer(response_location.length as u64)?;

        map.insert(key.finalize(), value.finalize());
    }

    let mut se = Serializer::new_vec();
    se.write_map(Len::Len(response_locations.len() as u64))?;
    for (key, value) in map {
        se.write_raw_bytes(&key)?;
        se.write_raw_bytes(&value)?;
    }
    Ok(se.finalize())
}

fn encode_section_lengths(sections: &[Section]) -> Result<Vec<u8>> {
    let mut se = Serializer::new_vec();

    se.write_array(Len::Len((sections.len() * 2) as u64))?;
    for section in sections {
        se.write_text(section.name)?;
        se.write_unsigned_integer(section.bytes.len() as u64)?;
    }
    Ok(se.finalize())
}

fn encode_headers(response: &Response) -> Result<Vec<u8>> {
    // Map keys must be sorted.
    // See [3.9. Canonical CBOR](https://tools.ietf.org/html/rfc7049#section-3.9)
    let mut map = std::collections::BTreeMap::<Vec<u8>, Vec<u8>>::new();

    // Write status
    let mut key = Serializer::new_vec();
    key.write_bytes(b":status")?;
    let mut value = Serializer::new_vec();
    value.write_bytes(response.status().as_u16().to_string().as_bytes())?;
    map.insert(key.finalize(), value.finalize());

    // Write headers
    for (header_name, header_value) in response.headers() {
        let mut key = Serializer::new_vec();
        key.write_bytes(header_name.as_str().as_bytes())?;
        let mut value = Serializer::new_vec();
        value.write_bytes(header_value.to_str()?.as_bytes())?;
        map.insert(key.finalize(), value.finalize());
    }

    let mut se = Serializer::new_vec();
    se.write_map(Len::Len(map.len() as u64))?;
    for (key, value) in map {
        se.write_raw_bytes(&key)?;
        se.write_raw_bytes(&value)?;
    }
    Ok(se.finalize())
}
