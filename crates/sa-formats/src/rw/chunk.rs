//! Generic RenderWare binary stream chunk tree.
//!
//! Every chunk starts with a 12-byte header: `u32 type`, `u32 size` (payload bytes), `u32 library
//! version stamp`. Container chunk types (see [`is_container`]) are parsed into child chunks when
//! their payload tiles exactly into whole chunks; otherwise (and for every other type) the payload
//! is kept as raw bytes. Serialising the tree reproduces the input byte for byte.

use super::id;
use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

/// Size of a chunk header in bytes.
pub const HEADER_SIZE: usize = 12;

/// Maximum nesting depth accepted while parsing (SA files nest at most 6 deep).
const MAX_DEPTH: usize = 64;

/// A chunk header as stored in the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    pub ty: u32,
    pub size: u32,
    pub version: u32,
}

impl ChunkHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, "RW chunk header");
        Ok(ChunkHeader {
            ty: r.u32()?,
            size: r.u32()?,
            version: r.u32()?,
        })
    }
}

/// Payload of a chunk.
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkBody {
    /// Raw payload bytes (leaf chunks, plugin data, or a container whose payload did not parse
    /// as a sequence of chunks).
    Data(Vec<u8>),
    /// Child chunks of a container.
    Children(Vec<Chunk>),
}

/// One chunk of a RenderWare binary stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub ty: u32,
    /// Library version stamp from the header (see [`super::LibraryStamp`]).
    pub version: u32,
    pub body: ChunkBody,
}

/// Chunk types whose payload is a sequence of chunks in RenderWare streams.
pub fn is_container(ty: u32) -> bool {
    matches!(
        ty,
        id::EXTENSION
            | id::CAMERA
            | id::TEXTURE
            | id::MATERIAL
            | id::MATLIST
            | id::WORLD
            | id::FRAMELIST
            | id::GEOMETRY
            | id::CLUMP
            | id::LIGHT
            | id::ATOMIC
            | id::TEXTURENATIVE
            | id::TEXDICTIONARY
            | id::GEOMETRYLIST
            | id::UVANIMDICT
    )
}

impl Chunk {
    pub fn new_data(ty: u32, version: u32, data: Vec<u8>) -> Self {
        Chunk {
            ty,
            version,
            body: ChunkBody::Data(data),
        }
    }

    pub fn new_children(ty: u32, version: u32, children: Vec<Chunk>) -> Self {
        Chunk {
            ty,
            version,
            body: ChunkBody::Children(children),
        }
    }

    /// Parses exactly one chunk from the start of `data`; returns it and the bytes consumed.
    pub fn parse(data: &[u8]) -> Result<(Chunk, usize)> {
        Self::parse_at(data, 0)
    }

    fn parse_at(data: &[u8], depth: usize) -> Result<(Chunk, usize)> {
        let h = ChunkHeader::parse(data)?;
        let end = HEADER_SIZE + h.size as usize;
        if data.len() < end {
            return Err(Error::Truncated {
                context: "RW chunk payload",
                offset: HEADER_SIZE,
                needed: h.size as usize,
                available: data.len() - HEADER_SIZE,
            });
        }
        let payload = &data[HEADER_SIZE..end];
        let body = if is_container(h.ty) && depth < MAX_DEPTH {
            match Self::parse_children(payload, depth + 1) {
                Some(children) => ChunkBody::Children(children),
                None => ChunkBody::Data(payload.to_vec()),
            }
        } else {
            ChunkBody::Data(payload.to_vec())
        };
        Ok((
            Chunk {
                ty: h.ty,
                version: h.version,
                body,
            },
            end,
        ))
    }

    /// Splits `payload` into chunks; `None` if it does not tile exactly.
    fn parse_children(payload: &[u8], depth: usize) -> Option<Vec<Chunk>> {
        let mut children = Vec::new();
        let mut pos = 0;
        while pos < payload.len() {
            let (c, n) = Self::parse_at(&payload[pos..], depth).ok()?;
            children.push(c);
            pos += n;
        }
        Some(children)
    }

    /// Payload size in bytes.
    pub fn payload_len(&self) -> usize {
        match &self.body {
            ChunkBody::Data(d) => d.len(),
            ChunkBody::Children(c) => c.iter().map(Chunk::encoded_len).sum(),
        }
    }

    /// Size including the header.
    pub fn encoded_len(&self) -> usize {
        HEADER_SIZE + self.payload_len()
    }

    pub fn write(&self, out: &mut Vec<u8>) {
        out.put_u32(self.ty);
        out.put_u32(self.payload_len() as u32);
        out.put_u32(self.version);
        match &self.body {
            ChunkBody::Data(d) => out.extend_from_slice(d),
            ChunkBody::Children(c) => c.iter().for_each(|c| c.write(out)),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.encoded_len());
        self.write(&mut out);
        out
    }

    pub fn data(&self) -> Option<&[u8]> {
        match &self.body {
            ChunkBody::Data(d) => Some(d),
            ChunkBody::Children(_) => None,
        }
    }

    pub fn children(&self) -> Option<&[Chunk]> {
        match &self.body {
            ChunkBody::Children(c) => Some(c),
            ChunkBody::Data(_) => None,
        }
    }

    /// Visits this chunk and all descendants, depth first.
    pub fn visit(&self, f: &mut impl FnMut(&Chunk, usize)) {
        self.visit_at(f, 0);
    }

    fn visit_at(&self, f: &mut impl FnMut(&Chunk, usize), depth: usize) {
        f(self, depth);
        if let ChunkBody::Children(c) = &self.body {
            for child in c {
                child.visit_at(f, depth + 1);
            }
        }
    }

    pub(crate) fn expect_data(&self, ty: u32, context: &'static str) -> Result<&[u8]> {
        if self.ty != ty {
            return Err(Error::invalid(
                context,
                format!("expected chunk {ty:#x}, found {:#x}", self.ty),
            ));
        }
        self.data()
            .ok_or_else(|| Error::invalid(context, format!("chunk {ty:#x} has children")))
    }

    pub(crate) fn expect_children(&self, ty: u32, context: &'static str) -> Result<&[Chunk]> {
        if self.ty != ty {
            return Err(Error::invalid(
                context,
                format!("expected chunk {ty:#x}, found {:#x}", self.ty),
            ));
        }
        self.children()
            .ok_or_else(|| Error::invalid(context, format!("chunk {ty:#x} payload is not a chunk sequence")))
    }
}

/// A complete RenderWare stream file: top-level chunks plus any trailing bytes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RwStream {
    pub chunks: Vec<Chunk>,
    /// Bytes after the last top-level chunk (IMG sector padding, for example). Parsing stops at
    /// the first position where fewer than 12 bytes remain, the chunk type is 0 (`rwID_NAOBJECT`,
    /// never a stream chunk) or the chunk would not fit.
    pub trailing: Vec<u8>,
}

impl RwStream {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut chunks = Vec::new();
        let mut pos = 0;
        while data.len() - pos >= HEADER_SIZE {
            let h = ChunkHeader::parse(&data[pos..])?;
            if h.ty == id::NAOBJECT || h.size as usize > data.len() - pos - HEADER_SIZE {
                break;
            }
            let (c, n) = Chunk::parse(&data[pos..])?;
            chunks.push(c);
            pos += n;
        }
        if chunks.is_empty() {
            return Err(Error::invalid("RW stream", "no chunk at the start of the data"));
        }
        Ok(RwStream {
            chunks,
            trailing: data[pos..].to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for c in &self.chunks {
            c.write(&mut out);
        }
        out.extend_from_slice(&self.trailing);
        out
    }
}

/// Sequential access to the children of a container while decoding typed structures.
pub(crate) struct Children<'a> {
    list: &'a [Chunk],
    next: usize,
    context: &'static str,
}

impl<'a> Children<'a> {
    pub(crate) fn of(chunk: &'a Chunk, ty: u32, context: &'static str) -> Result<Self> {
        Ok(Children {
            list: chunk.expect_children(ty, context)?,
            next: 0,
            context,
        })
    }

    pub(crate) fn peek_ty(&self) -> Option<u32> {
        self.list.get(self.next).map(|c| c.ty)
    }

    pub(crate) fn next(&mut self, ty: u32) -> Result<&'a Chunk> {
        let c = self
            .list
            .get(self.next)
            .ok_or_else(|| Error::invalid(self.context, format!("missing child chunk {ty:#x}")))?;
        if c.ty != ty {
            return Err(Error::invalid(
                self.context,
                format!("child {}: expected chunk {ty:#x}, found {:#x}", self.next, c.ty),
            ));
        }
        self.next += 1;
        Ok(c)
    }

    pub(crate) fn finish(self) -> Result<()> {
        if self.next != self.list.len() {
            return Err(Error::invalid(
                self.context,
                format!(
                    "{} unexpected trailing child chunk(s), first {:#x}",
                    self.list.len() - self.next,
                    self.list[self.next].ty
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_bytes(ty: u32, version: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.put_u32(ty);
        v.put_u32(payload.len() as u32);
        v.put_u32(version);
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn nested_round_trip() {
        let s = chunk_bytes(id::STRUCT, 0x1803FFFF, &[1, 2, 3, 4]);
        let plugin = chunk_bytes(0x0253F2FE, 0x1803FFFF, b"name");
        let ext = chunk_bytes(id::EXTENSION, 0x1803FFFF, &plugin);
        let mut payload = s.clone();
        payload.extend_from_slice(&ext);
        let mut file = chunk_bytes(id::CLUMP, 0x1803FFFF, &payload);
        file.extend_from_slice(&[0; 20]);
        let st = RwStream::parse(&file).unwrap();
        assert_eq!(st.chunks.len(), 1);
        assert_eq!(st.trailing.len(), 20);
        let c = st.chunks[0].children().unwrap();
        assert_eq!(c[0].data().unwrap(), &[1, 2, 3, 4]);
        assert_eq!(c[1].children().unwrap()[0].data().unwrap(), b"name");
        assert_eq!(st.to_bytes(), file);
    }

    #[test]
    fn container_that_does_not_tile_stays_raw() {
        // A clump whose payload is 5 bytes cannot hold a child chunk.
        let file = chunk_bytes(id::CLUMP, 0x1803FFFF, &[9, 9, 9, 9, 9]);
        let st = RwStream::parse(&file).unwrap();
        assert_eq!(st.chunks[0].data().unwrap(), &[9; 5]);
        assert_eq!(st.to_bytes(), file);
    }

    #[test]
    fn oversize_top_level_chunk_becomes_trailing() {
        let mut file = chunk_bytes(id::TEXDICTIONARY, 1, &chunk_bytes(id::STRUCT, 1, &[0; 4]));
        let mut bad = Vec::new();
        bad.put_u32(id::CLUMP);
        bad.put_u32(1000);
        bad.put_u32(1);
        file.extend_from_slice(&bad);
        let st = RwStream::parse(&file).unwrap();
        assert_eq!(st.chunks.len(), 1);
        assert_eq!(st.trailing, bad);
        assert_eq!(st.to_bytes(), file);
        assert!(Chunk::parse(&bad).is_err());
        assert!(RwStream::parse(&[0; 16]).is_err());
    }
}
