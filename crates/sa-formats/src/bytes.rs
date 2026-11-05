//! Little-endian byte cursor and writer helpers.

use crate::error::{Error, Result};

/// Bounds-checked little-endian reader over a byte slice.
#[derive(Clone)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    context: &'static str,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8], context: &'static str) -> Self {
        Reader {
            data,
            pos: 0,
            context,
        }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(crate) fn set_context(&mut self, context: &'static str) {
        self.context = context;
    }

    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.remaining() < n {
            return Err(Error::Truncated {
                context: self.context,
                offset: self.pos,
                needed: n,
                available: self.remaining(),
            });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub(crate) fn rest(&mut self) -> &'a [u8] {
        let s = &self.data[self.pos..];
        self.pos = self.data.len();
        s
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut a = [0u8; N];
        a.copy_from_slice(self.bytes(N)?);
        Ok(a)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub(crate) fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub(crate) fn f32s<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut a = [0f32; N];
        for v in &mut a {
            *v = self.f32()?;
        }
        Ok(a)
    }

    /// Checks that `count` items of `item_size` bytes fit before allocating for them.
    pub(crate) fn check_count(&self, count: usize, item_size: usize) -> Result<()> {
        let needed = count.saturating_mul(item_size);
        if needed > self.remaining() {
            return Err(Error::Truncated {
                context: self.context,
                offset: self.pos,
                needed,
                available: self.remaining(),
            });
        }
        Ok(())
    }

    pub(crate) fn expect_end(&self) -> Result<()> {
        if self.remaining() != 0 {
            return Err(Error::invalid(
                self.context,
                format!("{} unexpected bytes at offset {:#x}", self.remaining(), self.pos),
            ));
        }
        Ok(())
    }
}

pub(crate) trait Put {
    fn put_u8(&mut self, v: u8);
    fn put_u16(&mut self, v: u16);
    fn put_i16(&mut self, v: i16);
    fn put_u32(&mut self, v: u32);
    fn put_i32(&mut self, v: i32);
    fn put_f32(&mut self, v: f32);
    fn put_f32s(&mut self, v: &[f32]);
}

impl Put for Vec<u8> {
    fn put_u8(&mut self, v: u8) {
        self.push(v);
    }
    fn put_u16(&mut self, v: u16) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_i16(&mut self, v: i16) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_u32(&mut self, v: u32) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_i32(&mut self, v: i32) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_f32(&mut self, v: f32) {
        self.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    fn put_f32s(&mut self, v: &[f32]) {
        for x in v {
            self.put_f32(*x);
        }
    }
}

/// Returns the bytes before the first NUL (or all bytes if there is none).
pub(crate) fn c_str(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(n) => &bytes[..n],
        None => bytes,
    }
}
