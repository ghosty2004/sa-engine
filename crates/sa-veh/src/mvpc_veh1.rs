//! WRITER-MVP-C batch `mvpc-veh-1`: vehicle file loaders on the boot/loading path
//! (`CTrain::ReadAndInterpretTrackFile`). Ported from the 1.0 US bytes (gta-reversed a reading aid
//! only). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

/// the static line buffer the track reader fills (0xB71670)
const LINE: usize = 0xB71670;

orig! {
    /// CRT `operator new` (0x821195)
    fn crt_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `free` (0x8213AE)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// `CFileMgr::LoadFile(const char* path, uint8* buf, int32 size, const char* mode)` (0x538890)
    fn file_load(path: GPtr<u8>, buf: GPtr<u8>, size: i32, mode: u32) -> i32 = cdecl 0x538890 => unported;
    /// CRT `sscanf` (0x8220AD) with one target
    fn sscanf1(s: u32, fmt: u32, a: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sscanf` (0x8220AD) with four targets
    fn sscanf4(s: u32, fmt: u32, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// `eh vector constructor iterator(void* p, uint32 size, int32 count, void (*ctor)(void*))` (0x403D60)
    fn vector_ctor_iterator(p: GPtr<u8>, size: u32, count: i32, ctor: u32) = stdcall 0x403D60 => unported;
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Copies the bytes from `buf + pos` up to the next `'\n'` into the line buffer (no terminator) and
/// returns the position after the `'\n'`.
fn read_line(buf: GPtr<u8>, mut pos: i32) -> (i32, i32) {
    let mut n = 0;
    loop {
        let c = rd::<u8>(buf, pos as isize);
        if c == b'\n' {
            break;
        }
        g::<u8>(LINE + n as usize).write(c);
        n += 1;
        pos += 1;
    }
    (pos + 1, n)
}

/// `_ftol(v * 8.0f)`, low 16 bits.
fn coord16(v: f32) -> i16 {
    X87::from_f32(v).fmul_m(g::<f32>(0x859000).read()).ftol() as u32 as i16
}

/// `CTrain::ReadAndInterpretTrackFile(const char* filename, CTrainNode** nodes, int32* numNodes,
/// float* totalLength, int32 skipStations)` (0x6F55D0; SEH frame). When `*nodes` is null: a 0xB530-byte
/// buffer (`operator new`), `CFileMgr::LoadFile(filename, buf, 0xB530, "rb")`; the first line is copied
/// into the static line buffer 0xB71670 and terminated; when its first 10 bytes equal `"processed\0"`
/// (`repe cmpsb`) the next line replaces it. `sscanf("%d", numNodes)`, `operator new(n * 10)` and the
/// `CTrainNode` constructor (0x6F5370) through the vector-constructor iterator (null when the allocation
/// fails) into `*nodes`. Each of the `*numNodes` (re-read per iteration) lines is copied into the line
/// buffer without a terminator (the tail of a longer previous line stays) and parsed with
/// `"%f %f %f %d"`; x, y, z are stored as `_ftol(v * 8)` (i16 +0, +2, +4); unless `skipStations`, a
/// non-zero station flag appends the float position to the station array starting at 0x8D48F8 (per
/// call). The buffer is freed. Always: the total length from 0.0: for each node `+6 = _ftol(total * 3)`
/// (i16), then `total += sqrt(dx*dx + dy*dy)` with `d = node - next` (`next = (i + 1) % n`, coordinates
/// `i16 * 0.125`, x87); `*totalLength = total` (0.0 without nodes).
#[hook(addr = 0x6F55D0, conv = cdecl, name = "CTrain::ReadAndInterpretTrackFile", expect = "64 A1 00 00 00 00 6A FF",
       route = "split", hazards = "heap-writes,uninit-locals", recs = "grove-walk",
       note = "file read (tracks*.dat); the station flag local is uninitialised when sscanf matches fewer than 4 fields")]
pub fn read_and_interpret_track_file(filename: GPtr<u8>, nodes: GPtr<u8>, count: GPtr<u8>, length: GPtr<u8>, skip: i32) {
    if rd::<u32>(nodes, 0) == 0 {
        let buf = crt_new(0xB530);
        file_load(filename, buf, 0xB530, 0x859C6C);
        let (mut pos, n) = read_line(buf, 0);
        g::<u8>(LINE + n as usize).write(0);
        let processed = (0..10).all(|i| g::<u8>(LINE + i).read() == g::<u8>(0x872484 + i).read());
        if processed {
            let (p, n) = read_line(buf, pos);
            g::<u8>(LINE + n as usize).write(0);
            pos = p;
        }
        sscanf1(LINE as u32, 0x859508, count);
        let n = rd::<i32>(count, 0);
        let p = crt_new((n.wrapping_mul(10)) as u32);
        if !p.is_null() {
            vector_ctor_iterator(p, 10, n, 0x6F5370);
        }
        wr::<GPtr<u8>>(nodes, 0, p);
        let mut station = GPtr::<u8>::from_addr(0x8D48F8);
        let mut vals = [0u32; 4];
        let mut i = 0;
        let mut off = 0isize;
        while i < rd::<i32>(count, 0) {
            let (np, _) = read_line(buf, pos);
            pos = np;
            let v = local(&mut vals);
            sscanf4(LINE as u32, 0x872478, v, v.byte_add(4), v.byte_add(8), v.byte_add(0xC));
            let arr = rd::<GPtr<u8>>(nodes, 0);
            wr::<i16>(arr, off, coord16(f32::from_bits(vals[0])));
            wr::<i16>(arr, off + 2, coord16(f32::from_bits(vals[1])));
            wr::<i16>(arr, off + 4, coord16(f32::from_bits(vals[2])));
            if skip == 0 && vals[3] != 0 {
                wr::<u32>(station, 0, vals[0]);
                wr::<u32>(station, 4, vals[1]);
                wr::<u32>(station, 8, vals[2]);
                station = station.byte_add(0xC);
            }
            i += 1;
            off += 10;
        }
        crt_free(buf);
    }
    let mut total = X87::from_f32(g::<f32>(0x858B50).read());
    let n = rd::<i32>(count, 0);
    if n > 0 {
        let k3 = g::<f32>(0x858B3C).read();
        let k8 = g::<f32>(0x858C48).read();
        let mut i = 0i32;
        loop {
            let arr = rd::<GPtr<u8>>(nodes, 0);
            let cur = arr.byte_add(i as isize * 10);
            wr::<i16>(cur, 6, X87::from_f32(k3).fmul(total).ftol() as u32 as i16);
            let arr = rd::<GPtr<u8>>(nodes, 0);
            let cur = arr.byte_add(i as isize * 10);
            let n2 = rd::<i32>(count, 0);
            let next = arr.byte_add(((i + 1) % n2) as isize * 10);
            let dy = X87::from_i32(rd::<i16>(cur, 2) as i32)
                .fmul_m(k8)
                .fsub(X87::from_i32(rd::<i16>(next, 2) as i32).fmul_m(k8));
            let dx = X87::from_i32(rd::<i16>(cur, 0) as i32)
                .fmul_m(k8)
                .fsub(X87::from_i32(rd::<i16>(next, 0) as i32).fmul_m(k8));
            let len = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
            total = total.fadd(len);
            i += 1;
            if i >= n2 {
                break;
            }
        }
    }
    wr::<f32>(length, 0, total.to_f32());
}

pub const HOOKS: &[HookReg] = &[HOOK_READ_AND_INTERPRET_TRACK_FILE];
