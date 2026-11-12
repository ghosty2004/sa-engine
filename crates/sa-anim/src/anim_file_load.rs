//! `CAnimManager::LoadAnimFile` (STD-6, batch `std6-anim-1`). Ported from the 1.0 US bytes (0x4D47F0):
//! the ANP2 / ANP3 loader (STD-6) and the ANPK branch (0x4D485C..0x4D516D, INTRO-1: every cutscene IFP of
//! `CUTS.IMG` is ANPK).
//!
//! `CAnimBlendHierarchy` (0x18, `ms_aAnimations` 0xB4EA40): +0 hash, +4 sequences, +8 count (i16),
//! +0xA "uncompressed / to be processed" byte, +0xB keep-compressed, +0xC block index, +0x10 total time.
//! `CAnimBlock` (0x20, `ms_aAnimBlocks` 0xB5D4A0): +0 name[16], +0x10 loaded, +0x14 first animation,
//! +0x18 animation count, +0x1C first assoc group. `CAnimBlendSequence` (0xC): +4 flags, +6 frame count,
//! +8 frames.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// CRT `strncmp` (0x8214D0)
    fn crt_strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `strncpy` (0x821F40)
    fn crt_strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CAnimManager::GetAnimationBlock(name)` (0x4D3940)
    fn get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => crate::manager::get_animation_block;
    /// `CAnimManager::GetFirstAssocGroup(name)` (0x4D39B0)
    fn get_first_assoc_group(name: GPtr<u8>) -> i32 = cdecl 0x4D39B0 => crate::manager::get_first_assoc_group;
    /// `CMemoryMgr::Malloc(size, hint)` (0x72F420)
    fn mgr_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CAnimBlendHierarchy::SetName(name)` (0x4CF2D0)
    fn hier_set_name(h: GPtr<u8>, name: GPtr<u8>) = thiscall 0x4CF2D0 => crate::hierarchy::set_name;
    /// `CAnimBlendHierarchy::RemoveQuaternionFlips()` (0x4CF4E0)
    fn hier_remove_quaternion_flips(h: GPtr<u8>) = thiscall 0x4CF4E0 => crate::hierarchy::remove_quaternion_flips;
    /// `CAnimBlendHierarchy::CalcTotalTime()` (0x4CF2F0)
    fn hier_calc_total_time(h: GPtr<u8>) = thiscall 0x4CF2F0 => crate::hierarchy::calc_total_time;
    /// `CAnimBlendSequence::SetName(name)` (0x4D0C50)
    fn seq_set_name(s: GPtr<u8>, name: GPtr<u8>) = thiscall 0x4D0C50 => crate::sequence::set_name;
    /// `CAnimBlendSequence::SetBoneTag(tag)` (0x4D0C70)
    fn seq_set_bone_tag(s: GPtr<u8>, tag: i32) = thiscall 0x4D0C70 => crate::sequence::set_bone_tag;
    /// `CAnimBlendSequence::SetNumFrames(count, hasTranslation, compressed, data)` (0x4D0CD0)
    fn seq_set_num_frames(s: GPtr<u8>, n: i32, trans: u8, compressed: u8, data: GPtr<u8>) = thiscall 0x4D0CD0 => crate::sequence::set_num_frames;
    /// `operator new(size)` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `eh vector constructor iterator(ptr, size, count, ctor, dtor)` (0x821D30)
    fn eh_vector_ctor(p: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
}

/// `CAnimManager::ms_numAnimBlocks`, `ms_numAnimations`, `ms_aAnimBlocks`, `ms_aAnimations`.
const NUM_ANIM_BLOCKS: usize = 0xB4EA30;
const NUM_ANIMATIONS: usize = 0xB4EA2C;
const ANIM_BLOCKS: u32 = 0xB5D4A0;
const ANIMATIONS: u32 = 0xB4EA40;

fn g32(a: usize) -> GPtr<i32> {
    GPtr::from_addr(a)
}

fn rd<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

fn wr<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}

fn read_i32(s: GPtr<u8>) -> i32 {
    let mut v = 0i32;
    stream_read(s, GPtr::from_raw(&mut v as *mut i32).cast(), 4);
    v
}

/// `CAnimManager::LoadAnimFile(stream, loadCompressed, uncompressedAnims)` (0x4D47F0), ANP2 / ANP3:
/// the 8-byte header ("ANP3" at 0x85C704 sets the packed-data flag, "ANP2" at 0x85C70C), the block name
/// (24 bytes) and animation count. The block is found (`GetAnimationBlock`) or appended
/// (`ms_numAnimBlocks`, name `strncpy` 16, count, first animation = `ms_numAnimations`, first assoc
/// group); an existing block with count 0 gets the count and first animation. It is marked loaded. Per
/// animation (consecutive `ms_aAnimations` entries from the block's first): name, sequence count, with
/// ANP3 the frame-data size and flags (bit 0 -> +0xA) and one `CMemoryMgr::Malloc(size, 0)` block the
/// sequences share; name hash, count, block index, keep-compressed 0; the sequences (`operator new`
/// with the count in front, `eh vector constructor iterator`). Per sequence: name, frame type, frame
/// count, bone tag; the first sequence decides +0xA (0 for types 1 / 2, else 1); types 1..4 = rotation
/// (20-byte frames), rotation + translation (32), compressed rotation (10), compressed rotation +
/// translation (16): `SetNumFrames(count, translation, compressed, shared data)`, the frames read, and
/// with ANP3 the shared pointer advances and the sequence gets flag 8 (also for other types). An
/// animation with +0xA == 0 gets `RemoveQuaternionFlips` and `CalcTotalTime`. Finally
/// `ms_numAnimations = max(ms_numAnimations, last index + 1)`.
/// Other headers: the ANPK branch ([`load_anim_file_anpk`], INTRO-1: the intro's cutscene animations).
#[hook(addr = 0x4D47F0, conv = cdecl, name = "CAnimManager::LoadAnimFile", expect = "55 8B EC 83 E4 F8 6A FF 68 9C BD 83 00",
       unverified = "intro2-1", route = "L2", hazards = "heap-writes", recs = "new-game,intro",
       note = "ANP2 / ANP3 (std6-anim-1) and ANPK (intro2-1, the cutscene IFPs)")]
pub fn load_anim_file(s: GPtr<u8>, load_compressed: u32, uncompressed: GPtr<u8>) {
    let mut hdr = [0u8; 8];
    stream_read(s, GPtr::from_raw(hdr.as_mut_ptr()), 8);
    let id = GPtr::from_raw(hdr.as_mut_ptr());
    if crt_strncmp(id, GPtr::from_addr(0x85C70C), 4) != 0 && crt_strncmp(id, GPtr::from_addr(0x85C704), 4) != 0 {
        load_anim_file_anpk(s, load_compressed, uncompressed);
        return;
    }
    let packed = crt_strncmp(id, GPtr::from_addr(0x85C704), 4) == 0;
    let mut name = [0u8; 0x40];
    let np = GPtr::from_raw(name.as_mut_ptr());
    stream_read(s, np, 0x18);
    let n_anims = read_i32(s);
    let mut blk = get_animation_block(np);
    if blk.is_null() {
        let i = g32(NUM_ANIM_BLOCKS).read();
        blk = GPtr::from_addr((ANIM_BLOCKS as i32).wrapping_add(i << 5) as u32 as usize);
        g32(NUM_ANIM_BLOCKS).write(i + 1);
        crt_strncpy(blk, np, 0x10);
        wr::<i32>(blk, 0x18, n_anims);
        wr::<i32>(blk, 0x14, g32(NUM_ANIMATIONS).read());
        wr::<i32>(blk, 0x1C, get_first_assoc_group(blk));
    } else if rd::<i32>(blk, 0x18) == 0 {
        wr::<i32>(blk, 0x18, n_anims);
        wr::<i32>(blk, 0x14, g32(NUM_ANIMATIONS).read());
    }
    let mut index = rd::<i32>(blk, 0x14);
    wr::<u8>(blk, 0x10, 1);
    let blk_idx = ((blk.addr() as u32).wrapping_sub(ANIM_BLOCKS) as i32) >> 5;
    let mut hier_addr = ANIMATIONS.wrapping_add((index as u32).wrapping_mul(0x18));
    let mut a = 0;
    while a < n_anims {
        let hier = GPtr::<u8>::from_addr(hier_addr as usize);
        hier_addr = hier_addr.wrapping_add(0x18);
        index += 1;
        let mut data = GPtr::<u8>::NULL;
        stream_read(s, np, 0x18);
        let n_seq = read_i32(s);
        if packed {
            let size = read_i32(s);
            let flags = read_i32(s);
            wr::<u8>(hier, 0xA, flags as u8 & 1);
            data = mgr_malloc(size as u32, 0);
        }
        hier_set_name(hier, np);
        wr::<i16>(hier, 8, n_seq as i16);
        wr::<i32>(hier, 0xC, blk_idx);
        wr::<u8>(hier, 0xB, 0);
        let mem = operator_new((n_seq as u32).wrapping_mul(12).wrapping_add(4));
        let seqs = if mem.is_null() {
            GPtr::NULL
        } else {
            mem.cast::<i32>().write(n_seq);
            let p = mem.byte_add(4);
            eh_vector_ctor(p, 0xC, n_seq as u32, 0x4D0C10, 0x4D0C30);
            p
        };
        wr::<u32>(hier, 4, seqs.addr() as u32);
        let mut first = true;
        for k in 0..n_seq.max(0) {
            let mut sname = [0u8; 0x18];
            stream_read(s, GPtr::from_raw(sname.as_mut_ptr()), 0x18);
            let frame_type = read_i32(s);
            let n_frames = read_i32(s);
            let bone_tag = read_i32(s);
            let seq = GPtr::<u8>::from_addr((rd::<u32>(hier, 4)).wrapping_add(k as u32 * 12) as usize);
            seq_set_name(seq, GPtr::from_raw(sname.as_mut_ptr()));
            seq_set_bone_tag(seq, bone_tag);
            if first {
                first = false;
                wr::<u8>(hier, 0xA, if frame_type == 1 || frame_type == 2 { 0 } else { 1 });
            }
            let (trans, compressed, frame_size) = match frame_type {
                1 => (0, 0, 20),
                2 => (1, 0, 32),
                3 => (0, 1, 10),
                4 => (1, 1, 16),
                _ => (0, 0, 0),
            };
            if frame_size != 0 {
                seq_set_num_frames(seq, n_frames, trans, compressed, data);
                let bytes = (n_frames as u32).wrapping_mul(frame_size);
                stream_read(s, GPtr::from_addr(rd::<u32>(seq, 8) as usize), bytes);
                if packed {
                    data = data.byte_add(bytes as i32 as isize);
                }
            }
            if packed {
                wr::<u8>(seq, 4, rd::<u8>(seq, 4) | 8);
            }
        }
        if rd::<u8>(hier, 0xA) == 0 {
            hier_remove_quaternion_flips(hier);
            hier_calc_total_time(hier);
        }
        a += 1;
    }
    if index > g32(NUM_ANIMATIONS).read() {
        g32(NUM_ANIMATIONS).write(index);
    }
}

/// Reads an IFP section header (id, size) and rounds the size up to a multiple of 4 (`4 - size % 4` added when
/// `size & 3`), as the original does after every header read but the key-frame one.
fn read_section_header(s: GPtr<u8>, h: &mut [u32; 2]) {
    stream_read(s, GPtr::from_raw(h.as_mut_ptr()).cast(), 8);
    let r = h[1] & 3;
    if r != 0 {
        h[1] = h[1].wrapping_add(4 - r);
    }
}

/// `_ftol(v * k)` of a float through the x87 model, low 16 bits (the word store).
fn ftol_mul16(v: f32, k: f32) -> u16 {
    sa_math::x87::X87::from_f32(v).fmul_m(k).ftol() as u16
}

/// The ANPK branch of `CAnimManager::LoadAnimFile` (0x4D485C..0x4D516D; the 8-byte file header has been read, its
/// size is rounded but unused). Section headers are rounded to 4 ([`read_section_header`]) and their data read into
/// one 0x100-byte buffer. `INFO`: the animation count (+0) and the block name (+4): the block is found or appended
/// and marked loaded as in the ANP2 / ANP3 path. Per animation (the block's count +0x18 of them, from its first):
/// `NAME` (`SetName`), block index (+0xC); compressed = the `loadCompressed` byte, cleared when the name hash
/// matches the `GetUppercaseKey` of any entry of the 32-byte `uncompressed` list (no early exit; a NULL list is
/// skipped); +0xA = compressed, +0xB = 0. `DGAN` (header only), `INFO` (sequence count +0 -> +8 as i16;
/// `operator new(count * 12 + 4)` with the count in front and the `eh vector constructor iterator`, NULL when the
/// allocation fails -> +4). Per sequence: `CPAN` (header only), `ANIM`: `SetName(data)`, with a 0x2C-byte section
/// `SetBoneTag(data + 0x28)`; frame count at data + 0x1C, none -> the next sequence. The key-frame header (not
/// rounded): "KRTS" (0x85C6FC) -> `SetNumFrames(n, 1, compressed, NULL)`, 0x2C-byte frames (rotation, translation,
/// scale ignored, time); "KRT0" (0x85C6F4) -> the same with 0x20-byte frames; "KR00" (0x85C6EC) ->
/// `SetNumFrames(n, 0, compressed, NULL)`, 0x14-byte frames; another id sets no frames (the frame loop runs on the
/// sequence's count +6). The rotation is conjugated (x, y, z negated). Compressed frames (stride 0x10 when the
/// sequence has translation, flag bit 1, else 0xA): words `_ftol(q * 4096.0)` at +0..+6, `_ftol(time * 60.0 + 0.5)`
/// at +8, `_ftol(t * 1024.0)` at +0xA..+0xE. Uncompressed (stride 0x20 / 0x14): the quaternion at +0, time at
/// +0x10, translation at +0x14. After its sequences, an animation loaded uncompressed gets `RemoveQuaternionFlips`
/// and `CalcTotalTime`. Finally `ms_numAnimations = max(ms_numAnimations, first + count)`.
fn load_anim_file_anpk(s: GPtr<u8>, load_compressed: u32, uncompressed: GPtr<u8>) {
    let mut h = [0u32; 2];
    let mut buf = [0u8; 0x100];
    let bp = GPtr::from_raw(buf.as_mut_ptr());
    read_section_header(s, &mut h);
    stream_read(s, bp, h[1]);
    let n_anims = rd::<i32>(bp, 0);
    let np = bp.byte_add(4);
    let mut blk = get_animation_block(np);
    if blk.is_null() {
        let i = g32(NUM_ANIM_BLOCKS).read();
        blk = GPtr::from_addr((ANIM_BLOCKS as i32).wrapping_add(i << 5) as u32 as usize);
        g32(NUM_ANIM_BLOCKS).write(i + 1);
        crt_strncpy(blk, np, 0x10);
        wr::<i32>(blk, 0x18, n_anims);
        wr::<i32>(blk, 0x14, g32(NUM_ANIMATIONS).read());
        wr::<i32>(blk, 0x1C, get_first_assoc_group(blk));
    } else if rd::<i32>(blk, 0x18) == 0 {
        wr::<i32>(blk, 0x18, n_anims);
        wr::<i32>(blk, 0x14, g32(NUM_ANIMATIONS).read());
    }
    let mut index = rd::<i32>(blk, 0x14);
    wr::<u8>(blk, 0x10, 1);
    if rd::<i32>(blk, 0x18) > 0 {
        let blk_idx = ((blk.addr() as u32).wrapping_sub(ANIM_BLOCKS) as i32) >> 5;
        let mut hier_addr = ANIMATIONS.wrapping_add((index as u32).wrapping_mul(0x18));
        let mut a = 0;
        loop {
            // NAME
            read_section_header(s, &mut h);
            stream_read(s, bp, h[1]);
            index += 1;
            let hier = GPtr::<u8>::from_addr(hier_addr as usize);
            hier_addr = hier_addr.wrapping_add(0x18);
            hier_set_name(hier, bp);
            let mut compressed = load_compressed as u8;
            wr::<i32>(hier, 0xC, blk_idx);
            if !uncompressed.is_null() {
                let mut e = uncompressed;
                while rd::<u8>(e, 0) != 0 {
                    let hash = rd::<u32>(hier, 0);
                    if get_uppercase_key(e) == hash {
                        compressed = 0;
                    }
                    e = e.byte_add(0x20);
                }
            }
            wr::<u8>(hier, 0xA, compressed);
            wr::<u8>(hier, 0xB, 0);
            // DGAN (header only), INFO
            let mut dgan = [0u32; 2];
            read_section_header(s, &mut dgan);
            read_section_header(s, &mut h);
            stream_read(s, bp, h[1]);
            let n_seq = rd::<i32>(bp, 0);
            wr::<i16>(hier, 8, n_seq as i16);
            let mem = operator_new((n_seq as u32).wrapping_mul(12).wrapping_add(4));
            let (seqs, n) = if mem.is_null() {
                (GPtr::NULL, 0)
            } else {
                mem.cast::<i32>().write(n_seq);
                let p = mem.byte_add(4);
                eh_vector_ctor(p, 0xC, n_seq as u32, 0x4D0C10, 0x4D0C30);
                (p, n_seq)
            };
            wr::<u32>(hier, 4, seqs.addr() as u32);
            for k in 0..n.max(0) {
                // CPAN (header only), ANIM
                let mut cpan = [0u32; 2];
                read_section_header(s, &mut cpan);
                read_section_header(s, &mut h);
                stream_read(s, bp, h[1]);
                let seq = GPtr::<u8>::from_addr(rd::<u32>(hier, 4).wrapping_add(k as u32 * 12) as usize);
                seq_set_name(seq, bp);
                if h[1] == 0x2C {
                    seq_set_bone_tag(seq, rd::<i32>(bp, 0x28));
                }
                let n_frames = rd::<i32>(bp, 0x1C);
                if n_frames == 0 {
                    continue;
                }
                let mut kf = [0u32; 2];
                stream_read(s, GPtr::from_raw(kf.as_mut_ptr()).cast(), 8);
                let id = GPtr::from_raw(kf.as_mut_ptr()).cast::<u8>();
                let mut krts = false;
                let mut krt0 = false;
                if crt_strncmp(id, GPtr::from_addr(0x85C6FC), 4) == 0 {
                    krts = true;
                    seq_set_num_frames(seq, n_frames, 1, compressed, GPtr::NULL);
                } else if crt_strncmp(id, GPtr::from_addr(0x85C6F4), 4) == 0 {
                    krt0 = true;
                    seq_set_num_frames(seq, n_frames, 1, compressed, GPtr::NULL);
                } else if crt_strncmp(id, GPtr::from_addr(0x85C6EC), 4) == 0 {
                    seq_set_num_frames(seq, n_frames, 0, compressed, GPtr::NULL);
                }
                let (mut off_c_t, mut off_c, mut off_u_t, mut off_u) = (0u32, 0u32, 0u32, 0u32);
                let mut f = 0;
                while f < i32::from(rd::<i16>(seq, 6)) {
                    let mut fr = [0f32; 11];
                    let fp = GPtr::from_raw(fr.as_mut_ptr()).cast::<u8>();
                    let (has_t, time) = if krts {
                        stream_read(s, fp, 0x2C);
                        (true, fr[10])
                    } else if krt0 {
                        stream_read(s, fp, 0x20);
                        (true, fr[7])
                    } else {
                        stream_read(s, fp, 0x14);
                        (false, fr[4])
                    };
                    let q = [-fr[0], -fr[1], -fr[2], fr[3]];
                    let t = [fr[4], fr[5], fr[6]];
                    let seq_t = (rd::<u8>(seq, 4) >> 1) & 1 != 0;
                    let base = rd::<u32>(seq, 8);
                    if compressed != 0 {
                        let p = GPtr::<u8>::from_addr(base.wrapping_add(if seq_t { off_c_t } else { off_c }) as usize);
                        for (i, v) in q.iter().enumerate() {
                            wr::<u16>(p, 2 * i as isize, ftol_mul16(*v, 4096.0));
                        }
                        if has_t {
                            for (i, v) in t.iter().enumerate() {
                                wr::<u16>(p, 0xA + 2 * i as isize, ftol_mul16(*v, 1024.0));
                            }
                        }
                        let tv = sa_math::x87::X87::from_f32(time).fmul_m(60.0).fadd_m(0.5).ftol() as u16;
                        wr::<u16>(p, 8, tv);
                    } else {
                        let p = GPtr::<u8>::from_addr(base.wrapping_add(if seq_t { off_u_t } else { off_u }) as usize);
                        for (i, v) in q.iter().enumerate() {
                            wr::<f32>(p, 4 * i as isize, *v);
                        }
                        if has_t {
                            for (i, v) in t.iter().enumerate() {
                                wr::<f32>(p, 0x14 + 4 * i as isize, *v);
                            }
                        }
                        wr::<f32>(p, 0x10, time);
                    }
                    off_c_t = off_c_t.wrapping_add(0x10);
                    off_c = off_c.wrapping_add(0xA);
                    off_u_t = off_u_t.wrapping_add(0x20);
                    off_u = off_u.wrapping_add(0x14);
                    f += 1;
                }
            }
            if compressed == 0 {
                hier_remove_quaternion_flips(hier);
                hier_calc_total_time(hier);
            }
            a += 1;
            if a >= rd::<i32>(blk, 0x18) {
                break;
            }
        }
    }
    if index > g32(NUM_ANIMATIONS).read() {
        g32(NUM_ANIMATIONS).write(index);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_LOAD_ANIM_FILE];
