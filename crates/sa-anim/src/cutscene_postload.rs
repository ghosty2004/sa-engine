//! `CCutsceneMgr::LoadCutsceneData_postload` (batch `mvpl-anim-2`): loads the cutscene's animation file and camera
//! path splines from `anim\cuts.img`. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CMessages::ClearThisPrintBigNow(eMessageStyle)` (0x69ED80)
    fn messages_clear_this_print_big_now(style: u32) = cdecl 0x69ED80 => unported;
    /// `RwStreamOpen(type, access, data)` (0x7ECEF0)
    fn rw_stream_open(ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => unported;
    /// CRT `sprintf(buf, fmt, const char*)` (0x821BB5)
    fn sprintf_s(buf: GPtr<u8>, fmt: GPtr<u8>, s: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// `CDirectory::FindItem(const char* name, uint32& offset, uint32& size)` (0x5324A0, `ret 0xC`), AL
    fn directory_find_item(dir: GPtr<u8>, name: GPtr<u8>, offset: GPtr<u32>, size: GPtr<u32>) -> u8 = thiscall 0x5324A0 => unported;
    /// `CStreaming::MakeSpaceFor(int32)` (0x40E120)
    fn streaming_make_space_for(n: u32) = cdecl 0x40E120 => unported;
    /// `CStreaming::ImGonnaUseStreamingMemory()` (0x407BE0)
    fn streaming_im_gonna_use_streaming_memory() = cdecl 0x407BE0 => unported;
    /// `CStreaming::IHaveUsedStreamingMemory()` (0x407BF0)
    fn streaming_i_have_used_streaming_memory() = cdecl 0x407BF0 => unported;
    /// `RwStreamSkip(RwStream*, uint32)` (0x7ECD00)
    fn rw_stream_skip(s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ECD00 => unported;
    /// `CAnimManager::LoadAnimFile(RwStream*, bool, const char (*)[32])` (0x4D47F0)
    fn load_anim_file(s: GPtr<u8>, compressed: u32, names: GPtr<u8>) = cdecl 0x4D47F0 => crate::anim_file_load::load_anim_file;
    /// `CAnimBlendAssocGroup::CreateAssociations(const char* block, const char (*)[32], const char (*)[32], int32)`
    /// (0x4CE3B0, `ret 0x10`)
    fn group_create_associations_3(g: GPtr<u8>, block: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, n: i32) = thiscall 0x4CE3B0 => unported;
    /// `RwStreamClose(RwStream*, void*)` (0x7ECE20)
    fn rw_stream_close(s: GPtr<u8>, data: u32) -> i32 = cdecl 0x7ECE20 => unported;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    fn file_mgr_open_file(name: GPtr<u8>, mode: GPtr<u8>) -> u32 = cdecl 0x538900 => unported;
    /// `CFileMgr::Seek(FILESTREAM, int32, int32)` (0x538990)
    fn file_mgr_seek(f: u32, off: u32, origin: u32) -> i32 = cdecl 0x538990 => unported;
    /// `CCamera::LoadPathSplines(FILESTREAM)` (0x5B24D0, `ret 4`)
    fn camera_load_path_splines(cam: GPtr<u8>, f: u32) = thiscall 0x5B24D0 => unported;
    /// `CFileMgr::CloseFile(FILESTREAM)` (0x5389D0)
    fn file_mgr_close_file(f: u32) -> i32 = cdecl 0x5389D0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWanted::ClearQdCrimes()` (0x561FE0)
    fn wanted_clear_qd_crimes(w: GPtr<u8>) = thiscall 0x561FE0 => unported;
}

fn ga(a: u32) -> GPtr<u8> {
    GPtr::from_addr(a as usize)
}

/// `CCutsceneMgr::LoadCutsceneData_postload()` (0x5AFBC0, cdecl). `ClearThisPrintBigNow(0)`; dwords 0x8D2530 and
/// 0x8A5B20 and the byte 0x9654B0 cleared; `RwStreamOpen(rwSTREAMFILENAME, rwSTREAMREAD, "ANIM\\CUTS.IMG" 0x85EEB0)`;
/// the name buffer 0xB71670 = `sprintf("%s.IFP", cutscene name 0xBC3F0C)`. Found in the cutscene
/// directory (0xB5F848, `FindItem(name, &offset, &size)`): `MakeSpaceFor(size << 10)`, `ImGonnaUseStreamingMemory`,
/// `RwStreamSkip(offset << 11)`, `LoadAnimFile(stream, true, 0xBC2CC0)`, `ms_cutsceneAssociations.CreateAssociations(
/// 0xBC3F0C, 0xBC3288, 0xBC38C8, 32)`, `IHaveUsedStreamingMemory`, anim loaded (0xB5F850) = 1; else 0.
/// `RwStreamClose(stream, 0)`. Then `CFileMgr::OpenFile(0x85EEB0, "rb" 0x859C6C)`, the name = `sprintf(0x8689A4,
/// cutscene name)` ("%s.DAT" 0x8689A4); found: `ImGonnaUseStreamingMemory`, `Seek(offset << 11, 0)`,
/// `TheCamera.LoadPathSplines(file)`, `IHaveUsedStreamingMemory`, camera loaded (0xBC4006) = 1; else 0;
/// `CloseFile`. Cutscene status 0xB5F84C = 2, 0xBC3F08 = 0, and `FindPlayerPed(-1)`'s player data (+0x480; NULL
/// gives a NULL `CWanted`, as the original) `->m_pWanted->ClearQdCrimes()`.
#[hook(addr = 0x5afbc0, conv = cdecl, name = "CCutsceneMgr::LoadCutsceneData_postload", expect = "83 EC 08 56 6A 00",
       unverified = "mvpl-anim-2", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads anim\\cuts.img through RwStream and CFileMgr (file I/O): one run per OS state")]
pub fn cutscene_load_data_postload() {
    messages_clear_this_print_big_now(0);
    ga(0x8D2530).cast::<u32>().write(0);
    ga(0x8A5B20).cast::<u32>().write(0);
    ga(0x9654B0).write(0);
    let stream = rw_stream_open(2, 1, ga(0x85EEB0));
    let name = ga(0xB71670);
    sprintf_s(name, ga(0x8689AC), ga(0xBC3F0C));
    let dir = || ga(0xB5F848).cast::<GPtr<u8>>().read();
    let mut offset = 0u32;
    let mut size = 0u32;
    let po = GPtr::from_raw(&mut offset as *mut u32);
    let ps = GPtr::from_raw(&mut size as *mut u32);
    if directory_find_item(dir(), name, po, ps) != 0 {
        streaming_make_space_for(size << 10);
        streaming_im_gonna_use_streaming_memory();
        rw_stream_skip(stream, offset << 11);
        load_anim_file(stream, 1, ga(0xBC2CC0));
        group_create_associations_3(ga(0xBC4020), ga(0xBC3F0C), ga(0xBC3288), ga(0xBC38C8), 0x20);
        streaming_i_have_used_streaming_memory();
        ga(0xB5F850).write(1);
    } else {
        ga(0xB5F850).write(0);
    }
    rw_stream_close(stream, 0);
    let f = file_mgr_open_file(ga(0x85EEB0), ga(0x859C6C));
    sprintf_s(name, ga(0x8689A4), ga(0xBC3F0C));
    if directory_find_item(dir(), name, po, ps) != 0 {
        streaming_im_gonna_use_streaming_memory();
        file_mgr_seek(f, offset << 11, 0);
        camera_load_path_splines(ga(0xB6F028), f);
        streaming_i_have_used_streaming_memory();
        ga(0xBC4006).write(1);
    } else {
        ga(0xBC4006).write(0);
    }
    file_mgr_close_file(f);
    ga(0xB5F84C).cast::<u32>().write(2);
    ga(0xBC3F08).cast::<u32>().write(0);
    let pd = find_player_ped(-1).byte_add(0x480).cast::<GPtr<u8>>().read();
    let wanted = if pd.is_null() { GPtr::NULL } else { pd.cast::<GPtr<u8>>().read() };
    wanted_clear_qd_crimes(wanted);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CUTSCENE_LOAD_DATA_POSTLOAD];
