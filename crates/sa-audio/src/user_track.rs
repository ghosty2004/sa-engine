//! User radio tracks (FRONTIER-2 lane C turn 6): `CAEUserRadioTrackManager::LoadUserTrack` (0x4F35F0) and its callee
//! `GetTrackPath` (0x4F3050). Ported from the 1.0 US bytes. The decoders and `CAEDataStream` they create stay original
//! (catalog/stays-original.csv); their constructors are reached through the original. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{data_stream_ctor, data_stream_destructor_call, data_stream_initialise, vorbis_decoder_ctor};
use crate::common::{g, rd, wr};
use sa_core::bindings::{
    crt_free_8214bd, crt_new, crt_new_821195, file_close, file_mgr_seek, file_read, file_set_dir,
    file_set_dir_my_documents, open_file,
};

orig! {
    /// `CAEMFDecoder::CAEMFDecoder(const char* path, int32 track)` (0x4E7770, `ret 8`)
    fn mf_decoder_ctor(this: GPtr<u8>, path: GPtr<u8>, track: u32) -> u32 = thiscall 0x4E7770 => unported;
    /// `CAEWMADecoder::CAEWMADecoder(CAEDataStream*)` (0x502720)
    fn wma_decoder_ctor(this: GPtr<u8>, stream: GPtr<u8>) -> u32 = thiscall 0x502720 => unported;
    /// `CAEWaveDecoder::CAEWaveDecoder(CAEDataStream*)` (0x503250)
    fn wave_decoder_ctor(this: GPtr<u8>, stream: GPtr<u8>) -> u32 = thiscall 0x503250 => unported;
    /// `CAEUserRadioTrackManager::GetTrackPath(int32)` (0x4F3050)
    fn get_track_path_call(this: GPtr<u8>, track: u32) -> GPtr<u8> = thiscall 0x4F3050 => get_track_path;
}

/// The empty directory string (0x858B54) `CFileMgr::SetDir` gets after the user-files work.
const EMPTY_DIR: usize = 0x858B54;

/// `CAEUserRadioTrackManager::GetTrackPath(int32 track)` (0x4F3050, thiscall, `ret 4`): 0 for `track >= (uint16) +6`.
/// Otherwise the track index file "sa-utrax.dat" (0x859C6C) opened "rb" (0x860A98) in My Documents (`SetDirMyDocuments`,
/// `SetDir("")` after the open); a negative handle gives 0. Else a buffer of `length + 1` bytes (`operator new[]`
/// 0x821195; the 12-byte record at `[this] + 12 * track`: +0 offset, +4 length), `Seek(offset, 0)`, `Read(length)`,
/// `CloseFile`, a terminating 0; returns the buffer.
#[hook(addr = 0x4F3050, conv = thiscall, name = "CAEUserRadioTrackManager::GetTrackPath", expect = "56 8B 74 24 08 57",
       unverified = "frontier2-c12", route = "split", hazards = "heap-writes",
       note = "opens and reads the user track index file in My Documents (OS state)")]
pub fn get_track_path(this: GPtr<u8>, track: u32) -> GPtr<u8> {
    if (track as i32) >= i32::from(rd::<u16>(this, 6)) {
        return GPtr::NULL;
    }
    file_set_dir_my_documents();
    let f = open_file(GPtr::from_addr(0x859C6C), GPtr::from_addr(0x860A98));
    file_set_dir(GPtr::from_addr(EMPTY_DIR));
    if (f.addr() as i32) < 0 {
        return GPtr::NULL;
    }
    let rec = (track as i32 as isize) * 12;
    let buf = crt_new_821195(rd::<u32>(g::<u8>(rd::<u32>(this, 0) as usize), rec + 4).wrapping_add(1));
    file_mgr_seek(f, rd::<i32>(g::<u8>(rd::<u32>(this, 0) as usize), rec), 0);
    file_read(f, buf, rd::<u32>(g::<u8>(rd::<u32>(this, 0) as usize), rec + 4));
    file_close(f);
    let len = rd::<u32>(g::<u8>(rd::<u32>(this, 0) as usize), rec + 4);
    wr::<u8>(buf, len as isize, 0);
    buf
}

/// `CAEUserRadioTrackManager::LoadUserTrack(int32 track)` (0x4F35F0, thiscall, `ret 4`; SEH frame for the `new`
/// expressions, not modelled): a `CAEStreamingDecoder*` or 0. Nothing unless the manager is initialised (+4), `track`
/// is in `0..(uint16) +6`, the decoder of the track's type (record `[this] + 12 * track` +8) is enabled (byte `+8 +
/// type`) and `GetTrackPath` gives a path. Type 4: a `CAEMFDecoder(path, track)` (0xC4 bytes), returned as is. Others:
/// a `CAEDataStream(track, path, 0, 0, 0)` (0x28 bytes) initialised between `SetDirMyDocuments` and `SetDir("")`; a
/// failed one is destroyed and freed. Then type 1 a `CAEVorbisDecoder(stream, true)` (0x28), 2 a `CAEWaveDecoder`
/// (0x3C), 3 a `CAEWMADecoder` (0x20038); any other type, or a failed allocation, destroys and frees the stream.
#[hook(addr = 0x4F35F0, conv = thiscall, name = "CAEUserRadioTrackManager::LoadUserTrack", expect = "6A FF 68 C7 C1 83 00",
       unverified = "frontier2-c12", route = "split", hazards = "heap-writes",
       note = "opens the user's track files (OS state, through GetTrackPath and CAEDataStream::Initialise)")]
pub fn load_user_track(this: u32, track: u32) -> u32 {
    let me = g::<u8>(this as usize);
    if rd::<u8>(me, 4) == 0 {
        return 0;
    }
    let t = track as i32;
    if t < 0 || t >= i32::from(rd::<u16>(me, 6)) {
        return 0;
    }
    let rec = t as isize * 12;
    let ty = rd::<i32>(g::<u8>(rd::<u32>(me, 0) as usize), rec + 8);
    if rd::<u8>(me, ty as isize + 8) == 0 {
        return 0;
    }
    let path = get_track_path_call(me, track);
    if path.is_null() {
        return 0;
    }
    if rd::<i32>(g::<u8>(rd::<u32>(me, 0) as usize), rec + 8) == 4 {
        let m = crt_new(0xC4);
        if m.is_null() {
            return 0;
        }
        return mf_decoder_ctor(m, path, track);
    }
    let m = crt_new(0x28);
    let stream = if m.is_null() { GPtr::NULL } else { data_stream_ctor(m, track as i32, path, 0, 0, 0) };
    if stream.is_null() {
        return 0;
    }
    file_set_dir_my_documents();
    let ok = data_stream_initialise(stream);
    file_set_dir(GPtr::from_addr(EMPTY_DIR));
    if ok == 0 {
        data_stream_destructor_call(stream);
        crt_free_8214bd(stream);
        return 0;
    }
    let dec = match rd::<i32>(g::<u8>(rd::<u32>(me, 0) as usize), rec + 8) {
        1 => {
            let d = crt_new(0x28);
            if d.is_null() { 0 } else { vorbis_decoder_ctor(d.addr() as u32, stream.addr() as u32, 1) }
        }
        2 => {
            let d = crt_new(0x3C);
            if d.is_null() { 0 } else { wave_decoder_ctor(d, stream) }
        }
        3 => {
            let d = crt_new(0x20038);
            if d.is_null() { 0 } else { wma_decoder_ctor(d, stream) }
        }
        _ => 0,
    };
    if dec == 0 {
        data_stream_destructor_call(stream);
        crt_free_8214bd(stream);
    }
    dec
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GET_TRACK_PATH, HOOK_LOAD_USER_TRACK];
