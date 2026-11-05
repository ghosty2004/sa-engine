//! IFP animation packages (`anim/ped.ifp`, `anim/anim.img`, `anim/cuts.img`).
//!
//! Loader: `CAnimManager::LoadAnimFile(RwStream*, bool, const char(*)[32])` at 0x4D47F0 (1.0 US).
//! It reads an 8-byte header and dispatches on the FourCC with `strncmp(.., 4)`:
//! `ANP2` or `ANP3` take the "ANP" path (0x4D5172), **anything else** takes the `ANPK` path (the
//! exe never compares with `"ANPK"`). The header size word is not used by either path.
//!
//! # ANP2 / ANP3 (all of `ped.ifp` and `anim.img`)
//! `char block[24]`, `u32 numAnims`, then per animation `char name[24]`, `u32 numSequences`,
//! (ANP3 only) `u32 frameMemorySize`, `u32 flags`, then per sequence `char name[24]`,
//! `u32 frameType`, `u32 numFrames`, `i32 boneTag` and `numFrames` key frames read straight into
//! memory. Frame types (jump table at 0x4D55B8): 1 = rotation (`f32 q[4]`, `f32 time`, 0x14
//! bytes), 2 = rotation + translation (`+ f32 t[3]`, 0x20), 3 = compressed rotation (`i16 q[4]`,
//! `i16 time`, 0xA), 4 = compressed rotation + translation (`+ i16 t[3]`, 0x10). Other types make
//! the exe skip the frame read (the stream then desynchronises); this parser rejects them.
//!
//! Compressed scales, from `CAnimBlendSequence::Uncompress` 0x4D0D40: rotation `* 1/4096`
//! (0x858BFC = 0x39800000), time `* 1/60` (0x859044 = 0x3C888889), translation `* 1/1024`
//! (0x85C6D4 = 0x3A800000). The ANPK loader compresses with the inverse constants 4096.0
//! (0x858BF8) and 1024.0 (0x85C6D8).
//!
//! Quirk: the exe sets the hierarchy's "compressed" flag from `flags & 1` (0x4D52CC) and then
//! overwrites it from the first sequence's frame type (types 1/2 → uncompressed, else compressed,
//! 0x4D53F0). [`AnpAnimation::exe_compressed`] models that.
//!
//! # ANPK (all 148 cutscene `.ifp` in `cuts.img`)
//! Nested sections with an 8-byte header (`FourCC`, `u32 size`). The exe reads only the headers it
//! needs, rounds every data size up to a multiple of 4 and never checks a FourCC except the key
//! frame one: `ANPK` header, `INFO` header + `u32 numAnims` + block name, then per animation a
//! `NAME` section, a `DGAN` header (size unused), an `INFO` section (`u32 numSequences`, name),
//! and per sequence a `CPAN` header (size unused), an `ANIM` section (`char name[28]`,
//! `u32 numFrames`, `u32 next`, `u32 prev`, and `i32 boneTag` only when the rounded size is 44,
//! checked at 0x4D4BA8) and, when `numFrames != 0`, a key frame section `KRTS` (rotation,
//! translation, scale, time: 0x2C bytes per frame), `KRT0` (rotation, translation, time: 0x20) or
//! `KR00` (rotation, time: 0x14) whose size is not used. The exe stores `-x, -y, -z, w`
//! (conjugate) of the rotation and drops the scale.
//!
//! Every header word, name buffer and padding byte is kept raw, so [`Ifp::to_bytes`] gives the
//! input back byte for byte.

use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

/// Size of one compressed rotation unit (`CAnimBlendSequence::Uncompress`, 0x858BFC).
pub const ROTATION_SCALE: f32 = 1.0 / 4096.0;
/// Size of one compressed translation unit (0x85C6D4).
pub const TRANSLATION_SCALE: f32 = 1.0 / 1024.0;
/// Size of one compressed time unit (0x859044, the f32 nearest to 1/60).
pub const TIME_SCALE: f32 = f32::from_bits(0x3C88_8889);

/// A parsed IFP file.
#[derive(Debug, Clone, PartialEq)]
pub enum Ifp {
    /// `ANP2` / `ANP3` package.
    Anp(AnpPackage),
    /// `ANPK` package (and any other FourCC, which the exe treats the same way).
    Anpk(AnpkPackage),
}

impl Ifp {
    /// Parses an IFP file. Bytes after the last animation (IMG sector padding) are kept in the
    /// package's `trailing` field.
    pub fn parse(data: &[u8]) -> Result<Ifp> {
        if data.len() >= 4 && (&data[..4] == b"ANP2" || &data[..4] == b"ANP3") {
            Ok(Ifp::Anp(AnpPackage::parse(data)?))
        } else {
            Ok(Ifp::Anpk(AnpkPackage::parse(data)?))
        }
    }

    /// Serialises back to the exact input bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Ifp::Anp(p) => p.to_bytes(),
            Ifp::Anpk(p) => p.to_bytes(),
        }
    }

    /// Block name (up to the first NUL).
    pub fn block_name(&self) -> &[u8] {
        match self {
            Ifp::Anp(p) => crate::bytes::c_str(&p.block_name_raw),
            Ifp::Anpk(p) => p.block_name(),
        }
    }

    /// Number of animations.
    pub fn num_animations(&self) -> usize {
        match self {
            Ifp::Anp(p) => p.animations.len(),
            Ifp::Anpk(p) => p.animations.len(),
        }
    }

    /// Total number of sequences (bones) over all animations.
    pub fn num_sequences(&self) -> usize {
        match self {
            Ifp::Anp(p) => p.animations.iter().map(|a| a.sequences.len()).sum(),
            Ifp::Anpk(p) => p.animations.iter().map(|a| a.sequences.len()).sum(),
        }
    }

    /// Total number of key frames.
    pub fn num_key_frames(&self) -> usize {
        match self {
            Ifp::Anp(p) => p
                .animations
                .iter()
                .flat_map(|a| &a.sequences)
                .map(|s| s.frames.len())
                .sum(),
            Ifp::Anpk(p) => p
                .animations
                .iter()
                .flat_map(|a| &a.sequences)
                .map(|s| s.frames.as_ref().map_or(0, |f| f.frames.len()))
                .sum(),
        }
    }
}

/// A key frame with uncompressed (`f32`) components, as stored in the file.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KeyFrame {
    /// Rotation quaternion `x, y, z, w`.
    pub rot: [f32; 4],
    /// Translation (zero when the frame type has none).
    pub trans: [f32; 3],
    /// Scale (ANPK `KRTS` only; read and dropped by the exe).
    pub scale: [f32; 3],
    /// Time delta to the previous frame (seconds).
    pub time: f32,
}

/// A compressed key frame (`i16` components).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompressedKeyFrame {
    /// Rotation `x, y, z, w` in units of 1/4096.
    pub rot: [i16; 4],
    /// Time delta in units of 1/60 s.
    pub time: i16,
    /// Translation in units of 1/1024 (zero when the frame type has none).
    pub trans: [i16; 3],
}

impl CompressedKeyFrame {
    /// The values `CAnimBlendSequence::Uncompress` computes (`fild` + `fmul` by an `f32`
    /// constant under 24-bit x87 precision = one rounding of the exact product to `f32`).
    pub fn uncompress(&self) -> KeyFrame {
        let mul = |v: i16, c: f32| (f64::from(v) * f64::from(c)) as f32;
        KeyFrame {
            rot: self.rot.map(|v| mul(v, ROTATION_SCALE)),
            trans: self.trans.map(|v| mul(v, TRANSLATION_SCALE)),
            scale: [0.0; 3],
            time: mul(self.time, TIME_SCALE),
        }
    }
}

/// The key frames of an ANP sequence, by frame type.
#[derive(Debug, Clone, PartialEq)]
pub enum AnpFrames {
    /// Type 1: `f32` rotation + time.
    Rot(Vec<KeyFrame>),
    /// Type 2: `f32` rotation + time + translation.
    RotTrans(Vec<KeyFrame>),
    /// Type 3: `i16` rotation + time.
    RotCompressed(Vec<CompressedKeyFrame>),
    /// Type 4: `i16` rotation + time + translation.
    RotTransCompressed(Vec<CompressedKeyFrame>),
}

impl AnpFrames {
    /// The frame type number stored in the file.
    pub fn frame_type(&self) -> u32 {
        match self {
            AnpFrames::Rot(_) => 1,
            AnpFrames::RotTrans(_) => 2,
            AnpFrames::RotCompressed(_) => 3,
            AnpFrames::RotTransCompressed(_) => 4,
        }
    }

    /// Bytes per frame for a frame type (0 for an unknown type).
    pub fn frame_size(frame_type: u32) -> usize {
        match frame_type {
            1 => 0x14,
            2 => 0x20,
            3 => 0x0A,
            4 => 0x10,
            _ => 0,
        }
    }

    /// Number of frames.
    pub fn len(&self) -> usize {
        match self {
            AnpFrames::Rot(f) | AnpFrames::RotTrans(f) => f.len(),
            AnpFrames::RotCompressed(f) | AnpFrames::RotTransCompressed(f) => f.len(),
        }
    }

    /// Whether there are no frames.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the frames are compressed (types 3 and 4).
    pub fn is_compressed(&self) -> bool {
        matches!(self, AnpFrames::RotCompressed(_) | AnpFrames::RotTransCompressed(_))
    }

    /// Whether the frames carry a translation (types 2 and 4).
    pub fn has_translation(&self) -> bool {
        matches!(self, AnpFrames::RotTrans(_) | AnpFrames::RotTransCompressed(_))
    }

    /// Frames as the exe holds them after uncompressing.
    pub fn uncompressed(&self) -> Vec<KeyFrame> {
        match self {
            AnpFrames::Rot(f) | AnpFrames::RotTrans(f) => f.clone(),
            AnpFrames::RotCompressed(f) | AnpFrames::RotTransCompressed(f) => {
                f.iter().map(CompressedKeyFrame::uncompress).collect()
            }
        }
    }

    fn read(r: &mut Reader<'_>, frame_type: u32, n: usize) -> Result<AnpFrames> {
        let size = AnpFrames::frame_size(frame_type);
        if size == 0 {
            return Err(Error::unsupported(
                "IFP ANP sequence",
                format!("frame type {frame_type} (the exe skips the frame data and desynchronises)"),
            ));
        }
        r.check_count(n, size)?;
        Ok(match frame_type {
            1 | 2 => {
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    let rot = r.f32s::<4>()?;
                    let time = r.f32()?;
                    let trans = if frame_type == 2 { r.f32s::<3>()? } else { [0.0; 3] };
                    v.push(KeyFrame {
                        rot,
                        trans,
                        scale: [0.0; 3],
                        time,
                    });
                }
                if frame_type == 1 {
                    AnpFrames::Rot(v)
                } else {
                    AnpFrames::RotTrans(v)
                }
            }
            _ => {
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    let rot = [r.i16()?, r.i16()?, r.i16()?, r.i16()?];
                    let time = r.i16()?;
                    let trans = if frame_type == 4 {
                        [r.i16()?, r.i16()?, r.i16()?]
                    } else {
                        [0; 3]
                    };
                    v.push(CompressedKeyFrame { rot, time, trans });
                }
                if frame_type == 3 {
                    AnpFrames::RotCompressed(v)
                } else {
                    AnpFrames::RotTransCompressed(v)
                }
            }
        })
    }

    fn write(&self, out: &mut Vec<u8>) {
        match self {
            AnpFrames::Rot(f) | AnpFrames::RotTrans(f) => {
                let t = self.has_translation();
                for k in f {
                    out.put_f32s(&k.rot);
                    out.put_f32(k.time);
                    if t {
                        out.put_f32s(&k.trans);
                    }
                }
            }
            AnpFrames::RotCompressed(f) | AnpFrames::RotTransCompressed(f) => {
                let t = self.has_translation();
                for k in f {
                    for v in k.rot {
                        out.put_i16(v);
                    }
                    out.put_i16(k.time);
                    if t {
                        for v in k.trans {
                            out.put_i16(v);
                        }
                    }
                }
            }
        }
    }
}

/// One sequence (bone track) of an ANP animation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnpSequence {
    /// Raw name field (24 bytes; bytes after the NUL are kept).
    pub name_raw: [u8; 24],
    /// Bone tag (`-1` = use the name).
    pub bone_tag: i32,
    pub frames: AnpFrames,
}

impl AnpSequence {
    /// Name up to the first NUL.
    pub fn name(&self) -> &[u8] {
        crate::bytes::c_str(&self.name_raw)
    }
}

/// One ANP animation (`CAnimBlendHierarchy`).
#[derive(Debug, Clone, PartialEq)]
pub struct AnpAnimation {
    pub name_raw: [u8; 24],
    /// ANP3 only: size of the frame memory block the exe allocates for all sequences.
    pub frame_memory_size: Option<u32>,
    /// ANP3 only: flags word (bit 0 = compressed; overridden by the first sequence's type).
    pub flags: Option<u32>,
    pub sequences: Vec<AnpSequence>,
}

impl AnpAnimation {
    /// Name up to the first NUL.
    pub fn name(&self) -> &[u8] {
        crate::bytes::c_str(&self.name_raw)
    }

    /// Sum of the sequences' frame data sizes (what `frame_memory_size` should hold).
    pub fn frame_data_size(&self) -> usize {
        self.sequences
            .iter()
            .map(|s| s.frames.len() * AnpFrames::frame_size(s.frames.frame_type()))
            .sum()
    }

    /// The hierarchy's compressed flag after loading (0x4D52CC / 0x4D53F0): from the first
    /// sequence's frame type if there is one, else `flags & 1` (ANP3) or unchanged (ANP2, `None`).
    pub fn exe_compressed(&self) -> Option<bool> {
        match self.sequences.first() {
            Some(s) => Some(s.frames.is_compressed()),
            None => self.flags.map(|f| f & 1 != 0),
        }
    }
}

/// An `ANP2` or `ANP3` package.
#[derive(Debug, Clone, PartialEq)]
pub struct AnpPackage {
    /// `true` for `ANP3` (frame memory size + flags per animation).
    pub anp3: bool,
    /// Header size word (not used by the exe).
    pub size: u32,
    pub block_name_raw: [u8; 24],
    pub animations: Vec<AnpAnimation>,
    /// Bytes after the last animation (IMG sector padding).
    pub trailing: Vec<u8>,
}

impl AnpPackage {
    /// Parses an `ANP2`/`ANP3` file.
    pub fn parse(data: &[u8]) -> Result<AnpPackage> {
        let mut r = Reader::new(data, "IFP ANP header");
        let magic = r.array::<4>()?;
        let anp3 = match &magic {
            b"ANP3" => true,
            b"ANP2" => false,
            _ => return Err(Error::invalid("IFP ANP header", format!("magic {magic:02x?}"))),
        };
        let size = r.u32()?;
        let block_name_raw = r.array()?;
        let n = r.u32()? as usize;
        r.set_context("IFP ANP animation");
        // Smallest animation: name + sequence count.
        r.check_count(n, 28)?;
        let mut animations = Vec::with_capacity(n);
        for _ in 0..n {
            let name_raw = r.array()?;
            let nseq = r.u32()? as usize;
            let (frame_memory_size, flags) = if anp3 {
                (Some(r.u32()?), Some(r.u32()?))
            } else {
                (None, None)
            };
            r.set_context("IFP ANP sequence");
            r.check_count(nseq, 36)?;
            let mut sequences = Vec::with_capacity(nseq);
            for _ in 0..nseq {
                let name_raw = r.array()?;
                let frame_type = r.u32()?;
                let nframes = r.u32()? as usize;
                let bone_tag = r.i32()?;
                let frames = AnpFrames::read(&mut r, frame_type, nframes)?;
                sequences.push(AnpSequence {
                    name_raw,
                    bone_tag,
                    frames,
                });
            }
            r.set_context("IFP ANP animation");
            animations.push(AnpAnimation {
                name_raw,
                frame_memory_size,
                flags,
                sequences,
            });
        }
        Ok(AnpPackage {
            anp3,
            size,
            block_name_raw,
            animations,
            trailing: r.rest().to_vec(),
        })
    }

    /// Serialises back to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(if self.anp3 { b"ANP3" } else { b"ANP2" });
        out.put_u32(self.size);
        out.extend_from_slice(&self.block_name_raw);
        out.put_u32(self.animations.len() as u32);
        for a in &self.animations {
            out.extend_from_slice(&a.name_raw);
            out.put_u32(a.sequences.len() as u32);
            if self.anp3 {
                out.put_u32(a.frame_memory_size.unwrap_or(0));
                out.put_u32(a.flags.unwrap_or(0));
            }
            for s in &a.sequences {
                out.extend_from_slice(&s.name_raw);
                out.put_u32(s.frames.frame_type());
                out.put_u32(s.frames.len() as u32);
                out.put_i32(s.bone_tag);
                s.frames.write(&mut out);
            }
        }
        out.extend_from_slice(&self.trailing);
        out
    }

    /// Size the header word would have if it counted every byte after the 8-byte header up to
    /// the end of the last animation.
    pub fn content_size(&self) -> usize {
        self.to_bytes().len() - self.trailing.len() - 8
    }
}

/// An ANPK section header as read by the exe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionHeader {
    pub fourcc: [u8; 4],
    pub size: u32,
}

impl SectionHeader {
    fn read(r: &mut Reader<'_>) -> Result<SectionHeader> {
        Ok(SectionHeader {
            fourcc: r.array()?,
            size: r.u32()?,
        })
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.fourcc);
        out.put_u32(self.size);
    }

    /// The data size the exe reads: `size` rounded up to a multiple of 4.
    pub fn padded_size(&self) -> usize {
        (self.size as usize + 3) & !3
    }
}

/// A section whose data the exe reads (`size` rounded up to 4 bytes, kept raw).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSection {
    pub header: SectionHeader,
    /// `header.padded_size()` bytes.
    pub data: Vec<u8>,
}

impl DataSection {
    fn read(r: &mut Reader<'_>) -> Result<DataSection> {
        let header = SectionHeader::read(r)?;
        let data = r.bytes(header.padded_size())?.to_vec();
        Ok(DataSection { header, data })
    }

    fn write(&self, out: &mut Vec<u8>) {
        self.header.write(out);
        out.extend_from_slice(&self.data);
    }

    /// The first data word (`Info::Num`), if present.
    pub fn count(&self) -> Option<u32> {
        self.data
            .get(..4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// The string after the count word, up to the first NUL.
    pub fn info_name(&self) -> &[u8] {
        crate::bytes::c_str(self.data.get(4..).unwrap_or(&[]))
    }
}

/// The key frame section of an ANPK sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct AnpkFrames {
    /// `KRTS`, `KRT0` or `KR00` with its (unused) size word.
    pub header: SectionHeader,
    pub frames: Vec<KeyFrame>,
}

impl AnpkFrames {
    /// Bytes per frame for a key frame FourCC (`None` if the exe does not know it).
    pub fn frame_size(fourcc: &[u8; 4]) -> Option<usize> {
        match fourcc {
            b"KRTS" => Some(0x2C),
            b"KRT0" => Some(0x20),
            b"KR00" => Some(0x14),
            _ => None,
        }
    }

    /// Whether the frames carry a translation.
    pub fn has_translation(&self) -> bool {
        self.header.fourcc[2] == b'T'
    }

    /// Whether the frames carry a scale.
    pub fn has_scale(&self) -> bool {
        self.header.fourcc[3] == b'S'
    }
}

/// One `CPAN` sequence of an ANPK animation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnpkSequence {
    /// `CPAN` header (size not used by the exe).
    pub cpan: SectionHeader,
    /// `ANIM` header + rounded data (name[28], frame count, next, prev, optional bone tag).
    pub anim: DataSection,
    /// Key frames (absent when the frame count is 0).
    pub frames: Option<AnpkFrames>,
}

impl AnpkSequence {
    fn word(&self, at: usize) -> Option<u32> {
        self.anim
            .data
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Object (bone) name up to the first NUL.
    pub fn name(&self) -> &[u8] {
        crate::bytes::c_str(&self.anim.data[..self.anim.data.len().min(28)])
    }

    /// Frame count (`ANIM` + 28).
    pub fn num_frames(&self) -> u32 {
        self.word(28).unwrap_or(0)
    }

    /// Bone tag, present only when the rounded `ANIM` size is 44 (0x4D4BA8).
    pub fn bone_tag(&self) -> Option<i32> {
        if self.anim.header.padded_size() == 44 {
            self.word(40).map(|v| v as i32)
        } else {
            None
        }
    }
}

/// One ANPK animation (`NAME` + `DGAN`).
#[derive(Debug, Clone, PartialEq)]
pub struct AnpkAnimation {
    pub name: DataSection,
    /// `DGAN` header (size not used by the exe).
    pub dgan: SectionHeader,
    /// `INFO`: sequence count + name.
    pub info: DataSection,
    pub sequences: Vec<AnpkSequence>,
}

impl AnpkAnimation {
    /// Animation name up to the first NUL.
    pub fn name(&self) -> &[u8] {
        crate::bytes::c_str(&self.name.data)
    }
}

/// An `ANPK` package.
#[derive(Debug, Clone, PartialEq)]
pub struct AnpkPackage {
    /// File header (`ANPK` and the unused size).
    pub header: SectionHeader,
    /// Top-level `INFO`: animation count + block name.
    pub info: DataSection,
    pub animations: Vec<AnpkAnimation>,
    pub trailing: Vec<u8>,
}

impl AnpkPackage {
    /// Block name up to the first NUL.
    pub fn block_name(&self) -> &[u8] {
        self.info.info_name()
    }

    /// Parses an ANPK file following the exe's read sequence.
    pub fn parse(data: &[u8]) -> Result<AnpkPackage> {
        let mut r = Reader::new(data, "IFP ANPK header");
        let header = SectionHeader::read(&mut r)?;
        let info = DataSection::read(&mut r)?;
        let n = info
            .count()
            .ok_or_else(|| Error::invalid("IFP ANPK header", "INFO section has no count"))?
            as usize;
        r.set_context("IFP ANPK animation");
        r.check_count(n, 32)?;
        let mut animations = Vec::with_capacity(n);
        for _ in 0..n {
            let name = DataSection::read(&mut r)?;
            let dgan = SectionHeader::read(&mut r)?;
            let ainfo = DataSection::read(&mut r)?;
            let nseq = ainfo
                .count()
                .ok_or_else(|| Error::invalid("IFP ANPK animation", "INFO section has no count"))?
                as usize;
            r.set_context("IFP ANPK sequence");
            r.check_count(nseq, 16)?;
            let mut sequences = Vec::with_capacity(nseq);
            for _ in 0..nseq {
                let cpan = SectionHeader::read(&mut r)?;
                let anim = DataSection::read(&mut r)?;
                let mut seq = AnpkSequence {
                    cpan,
                    anim,
                    frames: None,
                };
                if seq.anim.data.len() < 32 {
                    return Err(Error::invalid(
                        "IFP ANPK sequence",
                        format!("ANIM section of {} bytes has no frame count", seq.anim.data.len()),
                    ));
                }
                let nframes = seq.num_frames() as usize;
                if nframes != 0 {
                    let kh = SectionHeader::read(&mut r)?;
                    let size = AnpkFrames::frame_size(&kh.fourcc).ok_or_else(|| {
                        Error::unsupported(
                            "IFP ANPK sequence",
                            format!(
                                "key frame section {:02x?} (the exe reads no frames and desynchronises)",
                                kh.fourcc
                            ),
                        )
                    })?;
                    r.check_count(nframes, size)?;
                    let mut frames = Vec::with_capacity(nframes);
                    let t = kh.fourcc[2] == b'T';
                    let s = kh.fourcc[3] == b'S';
                    for _ in 0..nframes {
                        let rot = r.f32s::<4>()?;
                        let trans = if t { r.f32s::<3>()? } else { [0.0; 3] };
                        let scale = if s { r.f32s::<3>()? } else { [0.0; 3] };
                        let time = r.f32()?;
                        frames.push(KeyFrame {
                            rot,
                            trans,
                            scale,
                            time,
                        });
                    }
                    seq.frames = Some(AnpkFrames { header: kh, frames });
                }
                sequences.push(seq);
            }
            r.set_context("IFP ANPK animation");
            animations.push(AnpkAnimation {
                name,
                dgan,
                info: ainfo,
                sequences,
            });
        }
        Ok(AnpkPackage {
            header,
            info,
            animations,
            trailing: r.rest().to_vec(),
        })
    }

    /// Serialises back to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.header.write(&mut out);
        self.info.write(&mut out);
        for a in &self.animations {
            a.name.write(&mut out);
            a.dgan.write(&mut out);
            a.info.write(&mut out);
            for s in &a.sequences {
                s.cpan.write(&mut out);
                s.anim.write(&mut out);
                if let Some(f) = &s.frames {
                    f.header.write(&mut out);
                    for k in &f.frames {
                        out.put_f32s(&k.rot);
                        if f.has_translation() {
                            out.put_f32s(&k.trans);
                        }
                        if f.has_scale() {
                            out.put_f32s(&k.scale);
                        }
                        out.put_f32(k.time);
                    }
                }
            }
        }
        out.extend_from_slice(&self.trailing);
        out
    }

    /// Checks the section sizes the exe ignores against the nesting they describe. Returns one
    /// message per mismatch (informational: the exe does not use these words).
    pub fn size_mismatches(&self) -> Vec<String> {
        let mut v = Vec::new();
        let mut total = 8 + self.info.data.len();
        for (ai, a) in self.animations.iter().enumerate() {
            let mut dgan = 8 + a.info.data.len();
            for (si, s) in a.sequences.iter().enumerate() {
                let mut cpan = 8 + s.anim.data.len();
                if let Some(f) = &s.frames {
                    let fs = AnpkFrames::frame_size(&f.header.fourcc).unwrap_or(0) * f.frames.len();
                    if f.header.size as usize != fs {
                        v.push(format!(
                            "anim {ai} seq {si}: {} size {} != {fs}",
                            String::from_utf8_lossy(&f.header.fourcc),
                            f.header.size
                        ));
                    }
                    cpan += 8 + fs;
                }
                if s.cpan.size as usize != cpan {
                    v.push(format!("anim {ai} seq {si}: CPAN size {} != {cpan}", s.cpan.size));
                }
                dgan += 8 + cpan;
            }
            if a.dgan.size as usize != dgan {
                v.push(format!("anim {ai}: DGAN size {} != {dgan}", a.dgan.size));
            }
            total += 8 + a.name.data.len() + 8 + dgan;
        }
        if self.header.size as usize != total {
            v.push(format!("ANPK size {} != {total}", self.header.size));
        }
        v
    }
}

/// Statistics about fields the exe ignores or derives (for reports): `(label, count)`.
pub fn ifp_notes(ifp: &Ifp) -> Vec<(String, u64)> {
    let mut m: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut add = |k: String| *m.entry(k).or_default() += 1;
    match ifp {
        Ifp::Anp(p) => {
            if p.size as usize != p.content_size() {
                add("header size word != bytes after the header".into());
            }
            for a in &p.animations {
                if a.frame_memory_size.is_some_and(|sz| sz as usize != a.frame_data_size()) {
                    add("animations with frame memory size != frame data size".into());
                }
                if let (Some(f), Some(c)) = (a.flags, a.exe_compressed()) {
                    if (f & 1 != 0) != c {
                        add("animations whose flags bit 0 disagrees with the first frame type".into());
                    }
                    if f & !1 != 0 {
                        add(format!("animations with flags {f:#x}"));
                    }
                }
                let types: std::collections::BTreeSet<u32> =
                    a.sequences.iter().map(|s| s.frames.frame_type()).collect();
                if types.iter().any(|t| *t == 1 || *t == 2) && types.iter().any(|t| *t == 3 || *t == 4) {
                    add("animations mixing compressed and uncompressed sequences".into());
                }
                for s in &a.sequences {
                    add(format!("sequences with frame type {}", s.frames.frame_type()));
                }
            }
        }
        Ifp::Anpk(p) => {
            if !p.size_mismatches().is_empty() {
                add("files whose unused section sizes do not match the nesting".into());
            }
            for a in &p.animations {
                for s in &a.sequences {
                    add(format!("ANIM sections of {} bytes", s.anim.header.size));
                    match &s.frames {
                        Some(f) => add(format!(
                            "key frame sections {}",
                            String::from_utf8_lossy(&f.header.fourcc)
                        )),
                        None => add("sequences without frames".into()),
                    }
                }
            }
        }
    }
    m.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name24(s: &[u8]) -> [u8; 24] {
        let mut a = [0u8; 24];
        a[..s.len()].copy_from_slice(s);
        a
    }

    #[test]
    fn anp3_round_trip_and_uncompress() {
        let pkg = AnpPackage {
            anp3: true,
            size: 123,
            block_name_raw: name24(b"ped"),
            animations: vec![AnpAnimation {
                name_raw: name24(b"walk"),
                frame_memory_size: Some(16 + 10),
                flags: Some(1),
                sequences: vec![
                    AnpSequence {
                        name_raw: name24(b"root"),
                        bone_tag: 0,
                        frames: AnpFrames::RotTransCompressed(vec![CompressedKeyFrame {
                            rot: [0, 0, 0, 4096],
                            time: 60,
                            trans: [1024, -512, 3],
                        }]),
                    },
                    AnpSequence {
                        name_raw: name24(b"head"),
                        bone_tag: 5,
                        frames: AnpFrames::RotCompressed(vec![CompressedKeyFrame {
                            rot: [-4096, 0, 0, 0],
                            time: 1,
                            trans: [0; 3],
                        }]),
                    },
                ],
            }],
            trailing: vec![0; 5],
        };
        let bytes = pkg.to_bytes();
        let back = Ifp::parse(&bytes).unwrap();
        assert_eq!(back, Ifp::Anp(pkg.clone()));
        assert_eq!(back.to_bytes(), bytes);
        assert_eq!(back.num_key_frames(), 2);
        let a = &pkg.animations[0];
        assert_eq!(a.frame_data_size(), 26);
        assert_eq!(a.exe_compressed(), Some(true));
        let k = a.sequences[0].frames.uncompressed()[0];
        assert_eq!(k.rot, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(k.trans, [1.0, -0.5, 3.0 / 1024.0]);
        assert_eq!(k.time, 60.0 * TIME_SCALE);
        assert_eq!(TIME_SCALE.to_bits(), 0x3C88_8889);
    }

    #[test]
    fn anp_rejects_unknown_frame_type() {
        let mut b = b"ANP2".to_vec();
        b.put_u32(0);
        b.extend_from_slice(&name24(b"x"));
        b.put_u32(1);
        b.extend_from_slice(&name24(b"a"));
        b.put_u32(1);
        b.extend_from_slice(&name24(b"s"));
        b.put_u32(7);
        b.put_u32(1);
        b.put_i32(-1);
        assert!(matches!(Ifp::parse(&b), Err(Error::Unsupported { .. })));
    }

    fn section(out: &mut Vec<u8>, cc: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(cc);
        out.put_u32(data.len() as u32);
        out.extend_from_slice(data);
        while !out.len().is_multiple_of(4) {
            out.push(0xAB);
        }
    }

    #[test]
    fn anpk_round_trip() {
        let mut b = Vec::new();
        b.extend_from_slice(b"ANPK");
        b.put_u32(0x1234);
        let mut info = vec![];
        info.put_u32(1);
        info.extend_from_slice(b"cut\0");
        section(&mut b, b"INFO", &info);
        section(&mut b, b"NAME", b"anim1\0"); // padded to 8 with 0xAB
        b.extend_from_slice(b"DGAN");
        b.put_u32(0);
        let mut ainfo = vec![];
        ainfo.put_u32(2);
        ainfo.extend_from_slice(&[0; 4]);
        section(&mut b, b"INFO", &ainfo);
        // sequence 1: 44-byte ANIM with bone tag, 1 KRT0 frame
        b.extend_from_slice(b"CPAN");
        b.put_u32(0);
        let mut anim = name24(b"Root").to_vec();
        anim.extend_from_slice(&[0; 4]);
        anim.put_u32(1);
        anim.put_u32(0);
        anim.put_u32(0);
        anim.put_i32(7);
        section(&mut b, b"ANIM", &anim);
        b.extend_from_slice(b"KRT0");
        b.put_u32(32);
        b.put_f32s(&[0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 3.0, 0.5]);
        // sequence 2: 40-byte ANIM, no frames
        b.extend_from_slice(b"CPAN");
        b.put_u32(0);
        let mut anim2 = name24(b"Bone").to_vec();
        anim2.extend_from_slice(&[0; 16]);
        section(&mut b, b"ANIM", &anim2);
        b.extend_from_slice(&[0; 7]);

        let p = AnpkPackage::parse(&b).unwrap();
        assert_eq!(p.to_bytes(), b);
        assert_eq!(p.block_name(), b"cut");
        assert_eq!(p.animations[0].name(), b"anim1");
        let s = &p.animations[0].sequences;
        assert_eq!(s[0].bone_tag(), Some(7));
        assert_eq!(s[1].bone_tag(), None);
        let f = s[0].frames.as_ref().unwrap();
        assert_eq!(f.frames[0].trans, [1.0, 2.0, 3.0]);
        assert_eq!(f.frames[0].time, 0.5);
        assert_eq!(p.trailing.len(), 7);
        assert!(!p.size_mismatches().is_empty());
        assert_eq!(Ifp::parse(&b).unwrap().num_key_frames(), 1);
    }
}
