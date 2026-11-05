//! `CFileLoader` (IDE/IPL/DAT/COL loaders) and `LoadingScreenLoadingFile` (PORT-STREAM, owner C).
//!
//! Source: the 1.0 US bytes (capstone over the exe, protection chunks followed), the Ghidra
//! decompilation and gta-reversed `FileLoader.cpp` (pin 20d8d2a) as reading aids, and
//! `docs/re/specs/STREAM/CFileLoader*.md`. The text parsers reproduce the exe's own parsing: the CRT
//! `sscanf` (0x8220AD) is called through `orig!` with the original format strings (their addresses
//! in `.rdata`), conversions land in locals in the original order, and the callees get the values in
//! the original argument order. sa-formats is not used here (its parsers are not the exe's CRT).
//!
//! Differences from gta-reversed that the port keeps (see `docs/arch/stream-port.md`):
//! - LoadLine(buffer) copies '\r' as ' ' (gr-deviations `FileLoader.cpp:250/251`);
//! - LoadOcclusionVolume tests only `name[len-7..len-5] == "int"` and parses flags with `%d`
//!   (`FileLoader.cpp:1424`);
//! - LoadCollisionFile(filename) picks the version only for COLL/COL2/COL3 and keeps the previous
//!   entry's version otherwise (`FileLoader.cpp:553` note);
//! - LoadTimeObject continues with the first parse for counts other than 1..3 (`FileLoader.cpp:1777`);
//! - LoadVehicleObject: wheel sizes not defaulted, f_heli = HELI, unknown type/class keep the field
//!   (`FileLoader.cpp:1811/1845/1846/1854/1899/1900`);
//! - LoadAtomicFile(stream) returns 0 without StopUsingCommonVehicleTexDicationary when the model has
//!   no RW object (`FileLoader.cpp:172`);
//! - LoadAudioZone passes `flags != 0` (gr: `flags == 1`) *new*.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::modelinfo::vehicle::sscanf_n;
use crate::shared::{field, model_info};

sa_core::globals! {
    list GLOBALS;
    /// `CFileLoader::ms_line` (LoadLine(FILE*) buffer)
    pub static MS_LINE @ 0xB71848: [u8; 512] = [0; 512];
    /// `gAtomicModelId` (LoadAtomicFile → SetRelatedModelInfoCB)
    pub static ATOMIC_MODEL_ID @ 0xB71840: i32 = 0;
    /// LoadCollisionFile(filename) data buffer (32 KB)
    pub static COL_FILE_BUFFER @ 0xBC40D8: [u8; 0x8000] = [0; 0x8000];
    // foreign (owner: sa-core memory/common, not declared there as globals)
    /// `PC_Scratch` / `s_MemoryHeapBuffer` (LoadLine(buffer) output)
    pub static PC_SCRATCH @ 0xC8E0C8: [u8; 0x4000] = [0; 0x4000];
    /// `gString` (common.h)
    pub static G_STRING @ 0xB71670: [u8; 352] = [0; 352];
}

fn s(addr: u32) -> GPtr<u8> {
    GPtr::from_addr(addr as usize)
}
fn rd8(p: GPtr<u8>, off: isize) -> u8 {
    field::<u8, u8>(p, off).read()
}
fn wr8(p: GPtr<u8>, off: isize, v: u8) {
    field::<u8, u8>(p, off).write(v)
}
fn rd16(p: GPtr<u8>, off: isize) -> i16 {
    field::<i16, u8>(p, off).read()
}
fn wr16(p: GPtr<u8>, off: isize, v: i16) {
    field::<i16, u8>(p, off).write(v)
}
fn rd32(p: GPtr<u8>, off: isize) -> u32 {
    field::<u32, u8>(p, off).read()
}
fn wr32(p: GPtr<u8>, off: isize, v: u32) {
    field::<u32, u8>(p, off).write(v)
}
fn rdp(p: GPtr<u8>, off: isize) -> GPtr<u8> {
    field::<GPtr<u8>, u8>(p, off).read()
}
fn wrp(p: GPtr<u8>, off: isize, v: GPtr<u8>) {
    field::<GPtr<u8>, u8>(p, off).write(v)
}
/// A pointer to a local (sscanf target / out parameter).
fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}
fn f(v: u32) -> f32 {
    f32::from_bits(v)
}
/// `repe cmpsb` over `lit.len()` bytes (the literal includes its NUL).
fn cmpsb_eq(p: GPtr<u8>, lit: &[u8]) -> bool {
    lit.iter().enumerate().all(|(k, b)| rd8(p, k as isize) == *b)
}

/// Calls into other (possibly original) functions.
pub mod call {
    use super::*;
    orig! {
        /// CFileMgr::ReadLine (0x5389B0)
        pub fn read_line(f: GPtr<u8>, buf: GPtr<u8>, len: i32) -> bool = cdecl 0x5389B0 => sa_core::filemgr::read_line;
        /// CFileMgr::Read (0x538950)
        pub fn read(f: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x538950 => sa_core::filemgr::read;
        /// CFileMgr::OpenFile (0x538900)
        pub fn open_file(name: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
        /// CFileMgr::CloseFile (0x5389D0)
        pub fn close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
        /// CRT `fopen` thunk (0x8232D8)
        pub fn fopen(name: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x8232D8 => unported;
        /// CRT `fclose` (0x82318B)
        pub fn fclose(f: GPtr<u8>) -> i32 = cdecl 0x82318B => unported;
        /// CRT `strrchr` (0x821490)
        pub fn strrchr(s: GPtr<u8>, c: i32) -> GPtr<u8> = cdecl 0x821490 => unported;
        /// CRT `sprintf` (0x821BB5), one argument
        pub fn sprintf1(d: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
        /// CRT `strncmp` (0x8214D0)
        pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
        /// CRT `strncpy` (0x821F40)
        pub fn strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
        /// CLoadingScreen::LoadingScreen(msg1, msg2, msg3) (0x53DED0)
        pub fn loading_screen(a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>) = cdecl 0x53DED0 => crate::loading_screen::loading_screen;
        /// CKeyGen::GetUppercaseKey (0x53CF30)
        pub fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => sa_core::keygen::get_uppercase_key;
        /// CBaseModelInfo::SetTexDictionary (0x4C4B40)
        pub fn set_tex_dictionary(mi: GPtr<u8>, name: GPtr<u8>) = thiscall 0x4C4B40 => unported;
        /// CBaseModelInfo::SetColModel (0x4C4BC0)
        pub fn set_col_model(mi: GPtr<u8>, col: GPtr<u8>, owns: u8) = thiscall 0x4C4BC0 => unported;
        /// CBaseModelInfo::SetBaseModelInfoFlags (0x5B3AD0)
        pub fn set_base_model_info_flags(mi: GPtr<u8>, flags: u32) = cdecl 0x5B3AD0 => unported;
        /// CAtomicModelInfo::SetAtomicModelInfoFlags (0x5B3B20)
        pub fn set_atomic_model_info_flags(mi: GPtr<u8>, flags: u32) = cdecl 0x5B3B20 => unported;
        /// CModelInfo::AddAtomicModel (0x4C6620)
        pub fn add_atomic_model(id: i32) -> GPtr<u8> = cdecl 0x4C6620 => unported;
        /// CModelInfo::AddDamageAtomicModel (0x4C6650)
        pub fn add_damage_atomic_model(id: i32) -> GPtr<u8> = cdecl 0x4C6650 => unported;
        /// CModelInfo::AddTimeModel (0x4C66B0)
        pub fn add_time_model(id: i32) -> GPtr<u8> = cdecl 0x4C66B0 => unported;
        /// CModelInfo::AddWeaponModel (0x4C6710)
        pub fn add_weapon_model(id: i32) -> GPtr<u8> = cdecl 0x4C6710 => unported;
        /// CModelInfo::AddClumpModel (0x4C6740)
        pub fn add_clump_model(id: i32) -> GPtr<u8> = cdecl 0x4C6740 => unported;
        /// CModelInfo::AddVehicleModel (0x4C6770)
        pub fn add_vehicle_model(id: i32) -> GPtr<u8> = cdecl 0x4C6770 => unported;
        /// CModelInfo::AddPedModel (0x4C67A0)
        pub fn add_ped_model(id: i32) -> GPtr<u8> = cdecl 0x4C67A0 => unported;
        /// CModelInfo::GetModelInfo(name, &id) (0x4C5940)
        pub fn get_model_info_full(name: GPtr<u8>, id: GPtr<u8>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
        /// CModelInfo::GetModelInfo(name, min, max) (0x4C5A20)
        pub fn get_model_info_min_max(name: GPtr<u8>, min: i32, max: i32) -> GPtr<u8> = cdecl 0x4C5A20 => unported;
        /// CTimeInfo::FindOtherTimeModel (0x4C47E0)
        pub fn find_other_time_model(ti: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C47E0 => unported;
        /// virtual CBaseModelInfo slot 0x30: SetAnimFile(name)
        pub fn v_set_anim_file(mi: GPtr<u8>, name: GPtr<u8>) = thiscall vslot 0x30 => unported;
        /// virtual slot 0x14: GetTimeInfo()
        pub fn v_get_time_info(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x14 => unported;
        /// virtual slot 0x10: GetModelType()
        pub fn v_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
        /// virtual slot 0x04: AsAtomicModelInfoPtr()
        pub fn v_as_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x04 => unported;
        /// virtual slot 0x08: AsDamageAtomicModelInfoPtr()
        pub fn v_as_damage_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x08 => unported;
        /// virtual slot 0x3C: SetAtomic(atomic)
        pub fn v_set_atomic(mi: GPtr<u8>, atomic: GPtr<u8>) = thiscall vslot 0x3C => unported;
        /// virtual slot 0x40: SetClump(clump)
        pub fn v_set_clump(mi: GPtr<u8>, clump: GPtr<u8>) = thiscall vslot 0x40 => unported;
        /// CDamageAtomicModelInfo::SetDamagedAtomic (0x4C48D0)
        pub fn set_damaged_atomic(mi: GPtr<u8>, atomic: GPtr<u8>) = thiscall 0x4C48D0 => unported;
        /// RwHelper GetNameAndDamage (0x5370A0)
        pub fn get_name_and_damage(node: GPtr<u8>, out: GPtr<u8>, damage: GPtr<u8>) = cdecl 0x5370A0 => unported;
        /// GetFrameNodeName (0x72FB30)
        pub fn get_frame_node_name(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x72FB30 => unported;
        /// CVisibilityPlugins::SetAtomicRenderCallback (0x7328A0)
        pub fn set_atomic_render_callback(atomic: GPtr<u8>, cb: u32) = cdecl 0x7328A0 => unported;
        /// CVisibilityPlugins::SetModelInfoIndex (0x732230)
        pub fn set_model_info_index(atomic: GPtr<u8>, id: i32) = cdecl 0x732230 => unported;
        /// RpClumpRemoveAtomic (0x74A4C0)
        pub fn rp_clump_remove_atomic(clump: GPtr<u8>, atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A4C0 => unported;
        /// RwFrameCreate (0x7F0410)
        pub fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
        /// RpAtomicSetFrame (0x74BF20)
        pub fn rp_atomic_set_frame(atomic: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
        /// RwStreamOpen (0x7ECEF0)
        pub fn rw_stream_open(ty: u32, access: u32, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => unported;
        /// RwStreamClose (0x7ECE20)
        pub fn rw_stream_close(stream: GPtr<u8>, data: GPtr<u8>) -> i32 = cdecl 0x7ECE20 => unported;
        /// RwStreamFindChunk (0x7ED2D0)
        pub fn rw_stream_find_chunk(stream: GPtr<u8>, ty: u32, len: GPtr<u8>, ver: GPtr<u8>) -> i32 = cdecl 0x7ED2D0 => unported;
        /// RwTexDictionaryGtaStreamRead (0x730FC0)
        pub fn rw_tex_dictionary_gta_stream_read(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x730FC0 => crate::rw_gta_stream::rw_tex_dictionary_gta_stream_read;
        /// RwTexDictionaryCreate (0x7F3600)
        pub fn rw_tex_dictionary_create() -> GPtr<u8> = cdecl 0x7F3600 => unported;
        /// RwTexDictionaryForAllTextures (0x7F3730)
        pub fn rw_tex_dictionary_for_all_textures(dict: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3730 => unported;
        /// RpClumpStreamRead (0x74B420)
        pub fn rp_clump_stream_read(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x74B420 => unported;
        /// RpClumpDestroy (0x74A310)
        pub fn rp_clump_destroy(clump: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
        /// RpClumpCreate (0x74A290)
        pub fn rp_clump_create() -> GPtr<u8> = cdecl 0x74A290 => unported;
        /// RpClumpForAllAtomics (0x749B70)
        pub fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
        /// _rwFrameCloneAndLinkClones (0x7EFFB0)
        pub fn rw_frame_clone_and_link_clones(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EFFB0 => unported;
        /// RwFrameAddChild (0x7F0B00)
        pub fn rw_frame_add_child(parent: GPtr<u8>, child: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0B00 => unported;
        /// RpClumpGtaStreamRead1 (0x72E570)
        pub fn rp_clump_gta_stream_read1(stream: GPtr<u8>) -> u8 = cdecl 0x72E570 => crate::rw_gta_stream::rp_clump_gta_stream_read1;
        /// RpClumpGtaStreamRead2 (0x72E620)
        pub fn rp_clump_gta_stream_read2(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x72E620 => crate::rw_gta_stream::rp_clump_gta_stream_read2;
        /// CCollisionPlugin::SetModelInfo (0x41B350)
        pub fn collision_plugin_set_model_info(mi: GPtr<u8>) = cdecl 0x41B350 => unported;
        /// CVehicleModelInfo::UseCommonVehicleTexDicationary (0x4C75A0)
        pub fn use_common_vehicle_tex_dicationary() = cdecl 0x4C75A0 => crate::modelinfo::vehicle::use_common_vehicle_tex_dicationary;
        /// CVehicleModelInfo::StopUsingCommonVehicleTexDicationary (0x4C75C0)
        pub fn stop_using_common_vehicle_tex_dicationary() = cdecl 0x4C75C0 => crate::modelinfo::vehicle::stop_using_common_vehicle_tex_dicationary;
        /// CTxdStore::FindTxdSlot(name) (0x731850)
        pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => crate::txdstore::find_txd_slot_name;
        /// CTxdStore::AddTxdSlot (0x731C80)
        pub fn add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => crate::txdstore::add_txd_slot;
        /// CGarages::AddOne (0x4471E0)
        pub fn garages_add_one(x1: f32, y1: f32, z1: f32, fx: f32, fy: f32, x2: f32, y2: f32, z2: f32, ty: u32, a: u32, name: GPtr<u8>, door: u32) -> i32 = cdecl 0x4471E0 => unported;
        /// COcclusion::AddOne (0x71DCD0)
        pub fn occlusion_add_one(cx: f32, cy: f32, z: f32, w: f32, l: f32, h: f32, rz: f32, ry: f32, rx: f32, flags: u32, interior: u8) = cdecl 0x71DCD0 => unported;
        /// CAudioZones::RegisterAudioBox (0x508240)
        pub fn register_audio_box(name: GPtr<u8>, id: i32, on: u8, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) = cdecl 0x508240 => unported;
        /// CAudioZones::RegisterAudioSphere (0x5081C0)
        pub fn register_audio_sphere(name: GPtr<u8>, id: i32, on: u8, x: f32, y: f32, z: f32, r: f32) = cdecl 0x5081C0 => unported;
        /// CBox::Set (0x40EDE0)
        pub fn box_set(b: GPtr<u8>, a: GPtr<u8>, c: GPtr<u8>) = thiscall 0x40EDE0 => unported;
        /// CTimeCycle::AddOne (0x55FF40)
        pub fn time_cycle_add_one(b: GPtr<u8>, far_clip: i32, extra: i32, intensity: f32, falloff: f32, lod: f32) = cdecl 0x55FF40 => unported;
        /// CTheCarGenerators::CreateCarGenerator (0x6F31A0)
        pub fn create_car_generator(x: f32, y: f32, z: f32, angle: f32, model: i32, c1: u32, c2: u32, force: u32, alarm: u32, lock: u32, min: u32, max: u32, ipl: i32, ignore: u32) -> i32 = cdecl 0x6F31A0 => unported;
        /// CCarGenerator::SwitchOn (0x6F32C0)
        pub fn car_generator_switch_on(g: GPtr<u8>) = thiscall 0x6F32C0 => unported;
        /// CStuntJumpManager::AddOne (0x49CB40)
        pub fn stunt_jump_add_one(start: GPtr<u8>, end: GPtr<u8>, cam: GPtr<u8>, reward: i32) = cdecl 0x49CB40 => unported;
        /// CTheZones::CreateZone (0x5728A0)
        pub fn create_zone(name: GPtr<u8>, ty: i32, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32, level: i32, text: GPtr<u8>) = cdecl 0x5728A0 => unported;
        /// CCullZones::AddMirrorAttributeZone (0x72DC10)
        pub fn add_mirror_attribute_zone(c: GPtr<u8>, a: f32, b: f32, c2: f32, d: f32, e: f32, f2: f32, flags: u32, g: f32, h: f32, i: f32, j: f32) = cdecl 0x72DC10 => unported;
        /// CCullZones::AddCullZone (0x72DF70)
        pub fn add_cull_zone(c: GPtr<u8>, a: f32, b: f32, c2: f32, d: f32, e: f32, f2: f32, flags: u32, flags2: u32) = cdecl 0x72DF70 => unported;
        /// CPathFind stub (0x44D2E0, `ret`-only in 1.0 US)
        pub fn path_find_stub_44d2e0(p: GPtr<u8>) = thiscall 0x44D2E0 => unported;
    }
}
use call::*;

// ------------------------------------------------------------------------------- lines ---

/// Skips bytes `<= 0x20` (unsigned) up to the NUL.
fn skip_ws(mut p: GPtr<u8>) -> GPtr<u8> {
    while rd8(p, 0) <= 0x20 && rd8(p, 0) != 0 {
        p = p.byte_add(1);
    }
    p
}

/// `CFileLoader::LoadLine(FILE*)` (0x536F80): ReadLine into `ms_line` (0x200), control chars and ','
/// → ' ', returns the first byte > 0x20 (or the NUL).
#[hook(addr = 0x536F80, conv = cdecl, name = "CFileLoader::LoadLine-File", expect = "8B 44 24 04 56")]
pub fn load_line_file(file: GPtr<u8>) -> GPtr<u8> {
    let line = MS_LINE.ptr().cast::<u8>();
    if !read_line(file, line, 0x200) {
        return GPtr::NULL;
    }
    let mut k = 0isize;
    while rd8(line, k) != 0 {
        let c = rd8(line, k);
        if c < 0x20 || c == b',' {
            wr8(line, k, b' ');
        }
        k += 1;
    }
    skip_ws(line)
}

/// `CFileLoader::LoadLine(char*& buffer, int32& size)` (0x536FE0): copies one line into the scratch
/// buffer 0xC8E0C8 (control chars incl. '\r' and ',' → ' '; gr-deviations `FileLoader.cpp:250/251`,
/// keep-original), advances the cursor past the terminator.
#[hook(addr = 0x536FE0, conv = cdecl, name = "CFileLoader::LoadLine-Bufer", expect = "56 8B 74 24 0C")]
pub fn load_line_buffer(buffer: GPtr<GPtr<u8>>, size: GPtr<i32>) -> GPtr<u8> {
    let out0 = PC_SCRATCH.ptr().cast::<u8>();
    let mut out = out0;
    let mut src = buffer.read();
    if size.read() <= 0 || rd8(src, 0) == 0 {
        return GPtr::NULL;
    }
    loop {
        let c = rd8(src, 0);
        if c == b'\n' || c == 0 {
            break;
        }
        wr8(out, 0, if c < 0x20 || c == b',' { b' ' } else { c });
        let n = size.read().wrapping_sub(1);
        out = out.byte_add(1);
        src = src.byte_add(1);
        size.write(n);
        if n <= 0 {
            break;
        }
    }
    buffer.write(src.byte_add(1));
    wr8(out, 0, 0);
    skip_ws(out0)
}

/// `CFileLoader::LoadPathHeader(line, int32& type)` (0x5B41C0): `"%d %d %s"` (id, type, name);
/// returns the id (uninitialised local when the parse fails: 0 here).
#[hook(addr = 0x5B41C0, conv = cdecl, name = "CFileLoader::LoadPathHeader", expect = "8B 54 24 08 83 EC 58")]
pub fn load_path_header(line: GPtr<u8>, out_type: GPtr<u8>) -> i32 {
    let mut id = 0i32;
    let mut name = [0u8; 84];
    sscanf_n(line, s(0x868C4C), &[out_type, lp(&mut id), GPtr::from_raw(name.as_mut_ptr())]);
    id
}

/// `CFileLoader::LoadGarage` (0x5B4530): `"%f %f %f %f %f %f %f %f %d %d %s"`, 11 fields required.
#[hook(addr = 0x5B4530, conv = cdecl, name = "CFileLoader::LoadGarage", expect = "83 EC 30 8D 44 24 28")]
pub fn load_garage(line: GPtr<u8>) {
    let mut v = [0u32; 10];
    let mut name = [0u8; 8 + 64];
    let n = {
        let [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9] = &mut v;
        sscanf_n(
            line,
            s(0x868C80),
            &[lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(a6), lp(a7), lp(a8), lp(a9), GPtr::from_raw(name.as_mut_ptr())],
        )
    };
    if n == 11 {
        // AddOne(x1, y1, z1, frontX, frontY, x2, y2, z2, type, 0, name, door)
        garages_add_one(f(v[0]), f(v[1]), f(v[2]), f(v[3]), f(v[4]), f(v[5]), f(v[6]), f(v[7]), v[9], 0, GPtr::from_raw(name.as_mut_ptr()), v[8]);
    }
}

/// `CFileLoader::LoadOcclusionVolume(line, filename)` (0x5B4C80). gr-deviations `FileLoader.cpp:1424`.
#[hook(addr = 0x5B4C80, conv = cdecl, name = "CFileLoader::LoadOcclusionVolume", expect = "83 EC 2C 56 8D 44 24 08")]
pub fn load_occlusion_volume(line: GPtr<u8>, filename: GPtr<u8>) {
    let mut v = [0u32; 10];
    {
        let [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9] = &mut v;
        // rotY, rotX, flags (#8..#10) preset to 0
        *a7 = 0;
        *a8 = 0;
        *a9 = 0;
        sscanf_n(line, s(0x868D74), &[lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(a6), lp(a7), lp(a8), lp(a9)]);
    }
    // z = height * 0.5f + bottomZ (x87 PC=24)
    let z = X87::from_f32(f(v[5])).fmul_m(0.5).fadd_m(f(v[2])).to_f32();
    let mut len = 0isize;
    while rd8(filename, len) != 0 {
        len += 1;
    }
    let interior = rd8(filename, len - 7) == b'i' && rd8(filename, len - 6) == b'n' && rd8(filename, len - 5) == b't';
    occlusion_add_one(f(v[0]), f(v[1]), z, f(v[3]), f(v[4]), f(v[5]), f(v[6]), f(v[7]), f(v[8]), v[9], interior as u8);
}

/// `CFileLoader::LoadAudioZone` (0x5B4D70): box (9 fields) or sphere; the flag passed is
/// `flags != 0` (*new* vs gr's `== 1`).
#[hook(addr = 0x5B4D70, conv = cdecl, name = "CFileLoader::LoadAudioZone", expect = "83 EC 34 56 8B 74 24 3C")]
pub fn load_audio_zone(line: GPtr<u8>) {
    let mut name = [0u8; 16 + 64];
    let np = GPtr::from_raw(name.as_mut_ptr());
    let (mut id, mut flags) = (0i32, 0i32);
    let mut v = [0u32; 6];
    let mut radius = 0u32;
    let n = {
        let [a0, a1, a2, a3, a4, a5] = &mut v;
        sscanf_n(line, s(0x868DAC), &[np, lp(&mut id), lp(&mut flags), lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5)])
    };
    if n == 9 {
        register_audio_box(np, id, (flags != 0) as u8, f(v[0]), f(v[1]), f(v[2]), f(v[3]), f(v[4]), f(v[5]));
        return;
    }
    {
        let [a0, a1, a2, ..] = &mut v;
        sscanf_n(line, s(0x868D94), &[np, lp(&mut id), lp(&mut flags), lp(a0), lp(a1), lp(a2), lp(&mut radius)]);
    }
    register_audio_sphere(np, id, (flags != 0) as u8, f(v[0]), f(v[1]), f(v[2]), f(radius));
}

/// `CFileLoader::LoadTimeCyclesModifier` (0x5B81D0): falloff 100, lod 1, 1; with fewer than 12
/// fields the last float is the 11th.
#[hook(addr = 0x5B81D0, conv = cdecl, name = "CFileLoader::LoadTimeCyclesModifier", expect = "83 EC 60 8D 44 24 00")]
pub fn load_time_cycles_modifier(line: GPtr<u8>) {
    let mut a = [0u32; 6];
    let (mut far_clip, mut extra) = (0i32, 0i32);
    let mut intensity = 0u32;
    let mut falloff = 0x42C8_0000u32;
    let mut lod1 = 0x3F80_0000u32;
    let mut lod2 = 0x3F80_0000u32;
    let n = {
        let [a0, a1, a2, a3, a4, a5] = &mut a;
        sscanf_n(
            line,
            s(0x869A78),
            &[lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(&mut far_clip), lp(&mut extra), lp(&mut intensity), lp(&mut falloff), lp(&mut lod1), lp(&mut lod2)],
        )
    };
    if n < 12 {
        lod2 = lod1;
    }
    let mut v1 = [a[0], a[1], a[2]];
    let mut v2 = [a[3], a[4], a[5]];
    let mut bx = [0u32; 6];
    box_set(GPtr::from_raw(bx.as_mut_ptr()).cast(), GPtr::from_raw(v1.as_mut_ptr()).cast(), GPtr::from_raw(v2.as_mut_ptr()).cast());
    time_cycle_add_one(GPtr::from_raw(bx.as_mut_ptr()).cast(), far_clip, extra, f(intensity), f(falloff), f(lod2));
}

/// `CFileLoader::AddTexDictionaries(dict, base)` (0x5B3910): AddTextureCB (0x5B38F0) for every
/// texture of `base`.
#[hook(addr = 0x5B3910, conv = cdecl, name = "CFileLoader::AddTexDictionaries", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn add_tex_dictionaries(dict: GPtr<u8>, base: GPtr<u8>) {
    rw_tex_dictionary_for_all_textures(base, 0x5B38F0, dict);
}

/// The angle argument of CreateCarGenerator: `angle * 57.29578f` (x87 PC=24, stored as f32).
pub fn car_gen_angle(angle: f32) -> f32 {
    X87::from_f32(angle).fmul_m(f(0x4265_2EE0)).to_f32()
}

/// `CFileLoader::LoadCarGenerator(CFileCarGenerator*, iplId)` (0x537990): fields read as the
/// original pushes them (colours/delays zero-extended words, chances bytes, flags & 1 / & 2).
#[hook(addr = 0x537990, conv = cdecl, name = "CFileLoader::LoadCarGenerator-0", expect = "8B 44 24 04 8A 48 1C")]
pub fn load_car_generator_struct(g: GPtr<u8>, ipl: i32) {
    let flags = rd8(g, 0x1C) as u32;
    let idx = create_car_generator(
        f(rd32(g, 0)),
        f(rd32(g, 4)),
        f(rd32(g, 8)),
        car_gen_angle(f(rd32(g, 0xC))),
        rd32(g, 0x10) as i32,
        rd16(g, 0x14) as u16 as u32,
        rd16(g, 0x18) as u16 as u32,
        flags & 1,
        rd8(g, 0x20) as u32,
        rd8(g, 0x24) as u32,
        rd16(g, 0x28) as u16 as u32,
        rd16(g, 0x2C) as u16 as u32,
        ipl,
        flags & 2,
    );
    if idx >= 0 {
        car_generator_switch_on(GPtr::from_addr(0xC27AD0 + ((idx as u32 & 0xFFFF) << 5) as usize));
    }
}

orig! {
    fn load_car_generator_struct_call(g: GPtr<u8>, ipl: i32) = cdecl 0x537990 => load_car_generator_struct;
}

/// `CFileLoader::LoadCarGenerator(line, iplId)` (0x5B4740): 12 fields required.
#[hook(addr = 0x5B4740, conv = cdecl, name = "CFileLoader::LoadCarGenerator-1", expect = "83 EC 30 8D 44 24 2C")]
pub fn load_car_generator_line(line: GPtr<u8>, ipl: i32) {
    let mut g = [0u32; 12];
    let n = {
        let [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11] = &mut g;
        sscanf_n(
            line,
            s(0x868CD4),
            &[lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(a6), lp(a7), lp(a8), lp(a9), lp(a10), lp(a11)],
        )
    };
    if n == 12 {
        load_car_generator_struct_call(GPtr::from_raw(g.as_mut_ptr()).cast(), ipl);
    }
}

/// `CFileLoader::LoadStuntJump` (0x5B45D0): 16 fields; start box, end box, camera, reward.
#[hook(addr = 0x5B45D0, conv = cdecl, name = "CFileLoader::LoadStuntJump", expect = "81 EC 94 00 00 00")]
pub fn load_stunt_jump(line: GPtr<u8>) {
    let mut v = [0u32; 15];
    let mut reward = 0i32;
    let n = {
        let [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14] = &mut v;
        sscanf_n(
            line,
            s(0x868CA4),
            &[lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(a6), lp(a7), lp(a8), lp(a9), lp(a10), lp(a11), lp(a12), lp(a13), lp(a14), lp(&mut reward)],
        )
    };
    if n == 16 {
        // start box: min (#1, #2, #3), max (#4, #5, #6); end box: (#7, #8, #9), (#10, #11, #12);
        // camera (#13, #14, #15)
        let mut start = [v[0], v[1], v[2], v[3], v[4], v[5]];
        let mut end = [v[6], v[7], v[8], v[9], v[10], v[11]];
        let mut cam = [v[12], v[13], v[14]];
        stunt_jump_add_one(
            GPtr::from_raw(start.as_mut_ptr()).cast(),
            GPtr::from_raw(end.as_mut_ptr()).cast(),
            GPtr::from_raw(cam.as_mut_ptr()).cast(),
            reward,
        );
    }
}

/// `CFileLoader::LoadZone` (0x5B4AB0): 10 fields required.
#[hook(addr = 0x5B4AB0, conv = cdecl, name = "CFileLoader::LoadZone", expect = "83 EC 44 8D 44 24 08")]
pub fn load_zone(line: GPtr<u8>) {
    let mut name = [0u8; 24 + 64];
    let mut text = [0u8; 12 + 64];
    let (mut ty, mut level) = (0i32, 0i32);
    let mut v = [0u32; 6];
    let n = {
        let [a0, a1, a2, a3, a4, a5] = &mut v;
        sscanf_n(
            line,
            s(0x868D04),
            &[GPtr::from_raw(name.as_mut_ptr()), lp(&mut ty), lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(&mut level), GPtr::from_raw(text.as_mut_ptr())],
        )
    };
    if n == 10 {
        create_zone(GPtr::from_raw(name.as_mut_ptr()), ty, f(v[0]), f(v[1]), f(v[2]), f(v[3]), f(v[4]), f(v[5]), level, GPtr::from_raw(text.as_mut_ptr()));
    }
}

/// `CFileLoader::LoadCullZone` (0x5B4B40): mirror zone (14 fields) or cull zone (11 fields, the
/// second flags preset to 0).
#[hook(addr = 0x5B4B40, conv = cdecl, name = "CFileLoader::LoadCullZone", expect = "83 EC 3C 56 8B 74 24 44")]
pub fn load_cull_zone(line: GPtr<u8>) {
    let mut c = [0u32; 3];
    let mut v = [0u32; 6];
    let mut flags = 0u32;
    let mut m = [0u32; 4];
    let mut flags2 = 0u32;
    let n = {
        let [c0, c1, c2] = &mut c;
        let [a0, a1, a2, a3, a4, a5] = &mut v;
        let [m0, m1, m2, m3] = &mut m;
        sscanf_n(
            line,
            s(0x868D48),
            &[lp(c0), lp(c1), lp(c2), lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(&mut flags), lp(m0), lp(m1), lp(m2), lp(m3)],
        )
    };
    let cp = GPtr::from_raw(c.as_mut_ptr()).cast::<u8>();
    if n == 14 {
        add_mirror_attribute_zone(cp, f(v[0]), f(v[1]), f(v[2]), f(v[3]), f(v[4]), f(v[5]), flags, f(m[3]), f(m[0]), f(m[1]), f(m[2]));
        return;
    }
    {
        let [c0, c1, c2] = &mut c;
        let [a0, a1, a2, a3, a4, a5] = &mut v;
        sscanf_n(
            line,
            s(0x868D24),
            &[lp(c0), lp(c1), lp(c2), lp(a0), lp(a1), lp(a2), lp(a3), lp(a4), lp(a5), lp(&mut flags), lp(&mut flags2)],
        );
    }
    add_cull_zone(GPtr::from_raw(c.as_mut_ptr()).cast(), f(v[0]), f(v[1]), f(v[2]), f(v[3]), f(v[4]), f(v[5]), flags, flags2);
}

/// `CFileLoader::LoadTXDParent` (0x5B75E0): `"%s %s"`; the parent index is written into the child
/// TxdDef (+6) without a null check (a free slot writes address 6, as the original).
#[hook(addr = 0x5B75E0, conv = cdecl, name = "CFileLoader::LoadTXDParent", expect = "8B 54 24 04 83 EC 40")]
pub fn load_txd_parent(line: GPtr<u8>) {
    let mut child = [0u8; 32 + 64];
    let mut parent = [0u8; 32 + 64];
    let cp = GPtr::from_raw(child.as_mut_ptr());
    let pp = GPtr::from_raw(parent.as_mut_ptr());
    sscanf_n(line, s(0x869860), &[cp, pp]);
    let mut c = find_txd_slot(cp);
    if c == -1 {
        c = add_txd_slot(cp);
    }
    let mut p = find_txd_slot(pp);
    if p == -1 {
        p = add_txd_slot(pp);
    }
    let pool = crate::shared::TXD_POOL.get().cast::<u8>();
    let map = rdp(pool, 4);
    let def = if (rd8(map, c as isize) as i8) < 0 {
        GPtr::NULL
    } else {
        rdp(pool, 0).byte_add(12 * c as isize)
    };
    wr16(def, 6, p as i16);
}

// ------------------------------------------------------------------------ RW file loaders ---

/// `CFileLoader::LoadTexDictionary(filename)` (0x5B3860): the stream is closed once; no TXD →
/// RwTexDictionaryCreate (tail call).
#[hook(addr = 0x5B3860, conv = cdecl, name = "CFileLoader::LoadTexDictionary", expect = "8B 44 24 04 56")]
pub fn load_tex_dictionary(name: GPtr<u8>) -> GPtr<u8> {
    let stream = rw_stream_open(2, 1, name);
    let mut txd = GPtr::NULL;
    if !stream.is_null() {
        if rw_stream_find_chunk(stream, 0x16, GPtr::NULL, GPtr::NULL) != 0 {
            txd = rw_tex_dictionary_gta_stream_read(stream);
        }
        rw_stream_close(stream, GPtr::NULL);
        if !txd.is_null() {
            return txd;
        }
    }
    rw_tex_dictionary_create()
}

/// `CFileLoader::LoadAtomicFile(RwStream*, modelId)` (0x5371F0). gr-deviations `FileLoader.cpp:172`
/// (keep-original): the RW-object test has no null check and its failure path skips
/// StopUsingCommonVehicleTexDicationary.
#[hook(addr = 0x5371F0, conv = cdecl, name = "CFileLoader::LoadAtomicFile-stream", expect = "53 55 8B 6C 24 10")]
pub fn load_atomic_file_stream(stream: GPtr<u8>, id: i32) -> bool {
    let mi = v_as_atomic(model_info(id).cast());
    let mut common = false;
    if !mi.is_null() && rd8(mi, 0x13) & 0x80 != 0 {
        common = true;
        use_common_vehicle_tex_dicationary();
    }
    if rw_stream_find_chunk(stream, 0x10, GPtr::NULL, GPtr::NULL) != 0 {
        let clump = rp_clump_stream_read(stream);
        if clump.is_null() {
            if common {
                stop_using_common_vehicle_tex_dicationary();
            }
            return false;
        }
        ATOMIC_MODEL_ID.set(id);
        rp_clump_for_all_atomics(clump, 0x537150, clump);
        rp_clump_destroy(clump);
    }
    if rdp(mi, 0x1C).is_null() {
        return false;
    }
    if common {
        stop_using_common_vehicle_tex_dicationary();
    }
    true
}

/// `CFileLoader::LoadAtomicFile(filename)` (0x5B39D0).
#[hook(addr = 0x5B39D0, conv = cdecl, name = "CFileLoader::LoadAtomicFile-", expect = "8B 44 24 04 57")]
pub fn load_atomic_file(name: GPtr<u8>) {
    let stream = rw_stream_open(2, 1, name);
    if rw_stream_find_chunk(stream, 0x10, GPtr::NULL, GPtr::NULL) != 0 {
        let clump = rp_clump_stream_read(stream);
        if !clump.is_null() {
            rp_clump_for_all_atomics(clump, 0x5B3930, clump);
            rp_clump_destroy(clump);
        }
    }
    rw_stream_close(stream, GPtr::NULL);
}

/// `CFileLoader::StartLoadClumpFile(RwStream*, modelId)` (0x5373F0): the model type is queried before
/// the chunk search (gr: after).
#[hook(addr = 0x5373F0, conv = cdecl, name = "CFileLoader::StartLoadClumpFile", expect = "8B 44 24 08 8B 0C 85 C8 B0 A9 00")]
pub fn start_load_clump_file(stream: GPtr<u8>, id: i32) -> bool {
    let vehicle = v_get_model_type(model_info(id).cast()) == 6;
    if rw_stream_find_chunk(stream, 0x10, GPtr::NULL, GPtr::NULL) == 0 {
        return false;
    }
    if vehicle {
        use_common_vehicle_tex_dicationary();
    }
    let r = rp_clump_gta_stream_read1(stream);
    if vehicle {
        stop_using_common_vehicle_tex_dicationary();
    }
    r != 0
}

/// `CFileLoader::FinishLoadClumpFile(RwStream*, modelId)` (0x537450).
#[hook(addr = 0x537450, conv = cdecl, name = "CFileLoader::FinishLoadClumpFile", expect = "53 56 57 8B 7C 24 14")]
pub fn finish_load_clump_file(stream: GPtr<u8>, id: i32) -> bool {
    let vehicle = v_get_model_type(model_info(id).cast()) == 6;
    if vehicle {
        use_common_vehicle_tex_dicationary();
    }
    let clump = rp_clump_gta_stream_read2(stream);
    if vehicle {
        stop_using_common_vehicle_tex_dicationary();
    }
    if clump.is_null() {
        return false;
    }
    v_set_clump(model_info(id).cast(), clump);
    true
}

/// `CFileLoader::LoadClumpFile(RwStream*, modelId)` (0x5372D0): complex hierarchies collect every
/// clump of the stream into one; others read one clump (vehicles with the collision plugin's model
/// set and the common vehicle TXD); model 0x1FC (journey) gets 2 doors.
#[hook(addr = 0x5372D0, conv = cdecl, name = "CFileLoader::LoadClumpFile-1", expect = "53 55 8B 6C 24 10")]
pub fn load_clump_file_stream(stream: GPtr<u8>, id: i32) -> bool {
    let mi = model_info(id).cast::<u8>();
    let vehicle = v_get_model_type(mi) == 6;
    if rd8(mi, 0x13) & 2 != 0 {
        let parent = rp_clump_create();
        let frame = rw_frame_create();
        wrp(parent, 4, frame);
        if rw_stream_find_chunk(stream, 0x10, GPtr::NULL, GPtr::NULL) != 0 {
            loop {
                let child = rp_clump_stream_read(stream);
                if child.is_null() {
                    return false;
                }
                let cf = rw_frame_clone_and_link_clones(rdp(child, 4));
                rw_frame_add_child(frame, cf);
                rp_clump_for_all_atomics(child, 0x537290, parent);
                rp_clump_destroy(child);
                if rw_stream_find_chunk(stream, 0x10, GPtr::NULL, GPtr::NULL) == 0 {
                    break;
                }
            }
        }
        v_set_clump(mi, parent);
        return true;
    }
    if rw_stream_find_chunk(stream, 0x10, GPtr::NULL, GPtr::NULL) == 0 {
        return false;
    }
    if vehicle {
        collision_plugin_set_model_info(mi);
        use_common_vehicle_tex_dicationary();
    }
    let clump = rp_clump_stream_read(stream);
    if vehicle {
        collision_plugin_set_model_info(GPtr::NULL);
        stop_using_common_vehicle_tex_dicationary();
    }
    if clump.is_null() {
        return false;
    }
    v_set_clump(mi, clump);
    if id == 0x1FC {
        wr8(mi, 0x4C, 2);
    }
    true
}

/// `CFileLoader::LoadClumpFile(filename)` (0x5B3A30): every clump is given to the model named by its
/// frame (GetModelInfo(name, null)), else destroyed.
#[hook(addr = 0x5B3A30, conv = cdecl, name = "CFileLoader::LoadClumpFile-", expect = "8B 44 24 04 57")]
pub fn load_clump_file(name: GPtr<u8>) {
    let stream = rw_stream_open(2, 1, name);
    while rw_stream_find_chunk(stream, 0x10, GPtr::NULL, GPtr::NULL) != 0 {
        let clump = rp_clump_stream_read(stream);
        if clump.is_null() {
            continue;
        }
        let node = get_frame_node_name(rdp(clump, 4));
        let mi = get_model_info_full(node, GPtr::NULL);
        if !mi.is_null() {
            v_set_clump(mi, clump);
        } else {
            rp_clump_destroy(clump);
        }
    }
    rw_stream_close(stream, GPtr::NULL);
}

/// Shared tail of SetRelatedModelInfoCB / FindRelatedModelInfoCB.
fn attach_atomic(mi: GPtr<u8>, atomic: GPtr<u8>, damaged: bool, clump: GPtr<u8>, id: i32) {
    set_atomic_render_callback(atomic, 0);
    if damaged {
        let d = v_as_damage_atomic(mi);
        set_damaged_atomic(d, atomic);
    } else {
        v_set_atomic(mi, atomic);
    }
    rp_clump_remove_atomic(clump, atomic);
    let frame = rw_frame_create();
    rp_atomic_set_frame(atomic, frame);
    set_model_info_index(atomic, id);
}

/// `CFileLoader::SetRelatedModelInfoCB(RpAtomic*, RpClump*)` (0x537150): the atomic goes to
/// `gAtomicModelId`'s model (damaged variant by node name).
#[hook(addr = 0x537150, conv = cdecl, name = "CFileLoader::SetRelatedModelInfoCB", expect = "A1 40 18 B7 00")]
pub fn set_related_model_info_cb(atomic: GPtr<u8>, clump: GPtr<u8>) -> GPtr<u8> {
    let id = ATOMIC_MODEL_ID.get();
    let mi = v_as_atomic(model_info(id).cast());
    let mut name = [0u8; 24];
    let mut damaged = 0u8;
    let node = get_frame_node_name(rdp(atomic, 4));
    get_name_and_damage(node, GPtr::from_raw(name.as_mut_ptr()), lp(&mut damaged));
    attach_atomic(mi, atomic, damaged != 0, clump, ATOMIC_MODEL_ID.get());
    atomic
}

/// `CFileLoader::FindRelatedModelInfoCB(RpAtomic*, RpClump*)` (0x5B3930): the model is found by
/// the node name.
#[hook(addr = 0x5B3930, conv = cdecl, name = "CFileLoader::FindRelatedModelInfoCB", expect = "83 EC 1C 56 8B 74 24 24")]
pub fn find_related_model_info_cb(atomic: GPtr<u8>, clump: GPtr<u8>) -> GPtr<u8> {
    let mut name = [0u8; 24];
    let mut damaged = 0u8;
    let mut id = 0i32;
    let node = get_frame_node_name(rdp(atomic, 4));
    get_name_and_damage(node, GPtr::from_raw(name.as_mut_ptr()), lp(&mut damaged));
    let mi = get_model_info_full(GPtr::from_raw(name.as_mut_ptr()), lp(&mut id));
    if !mi.is_null() {
        let mi = v_as_atomic(mi);
        attach_atomic(mi, atomic, damaged != 0, clump, id);
    }
    atomic
}

/// `LoadingScreenLoadingFile(path)` (0x5B3680, spec CFileLoader.md): "Loading <basename>" into
/// gString (unbounded `sprintf`), then the loading screen.
#[hook(addr = 0x5B3680, conv = cdecl, name = "LoadingScreenLoadingFile", expect = "56 8B 74 24 08")]
pub fn loading_screen_loading_file(path: GPtr<u8>) {
    let p = strrchr(path, b'\\' as i32);
    let name = if p.is_null() { path } else { p.byte_add(1) };
    let g = G_STRING.ptr().cast::<u8>();
    sprintf1(g, s(0x868B40), name);
    loading_screen(s(0x863A6C), g, GPtr::NULL);
}

// --------------------------------------------------------------------------- IDE objects ---

/// `CFileLoader::LoadObject` (0x5B3C60): `"%d %s %s %f %d"`, else the old count-based formats.
/// Returns the id (-1 when neither parses).
#[hook(addr = 0x5B3C60, conv = cdecl, name = "CFileLoader::LoadObject", expect = "83 EC 48 56 8B 74 24 50")]
pub fn load_object(line: GPtr<u8>) -> i32 {
    let mut id = 0i32;
    let mut model = [0u8; 24 + 64];
    let mut txd = [0u8; 24 + 64];
    let mp = GPtr::from_raw(model.as_mut_ptr());
    let tp = GPtr::from_raw(txd.as_mut_ptr());
    let mut dist = 0u32;
    let mut flags = 0u32;
    let mut count = 0i32;
    let (mut d2, mut d3) = (0u32, 0u32);
    let n = sscanf_n(line, s(0x868B9C), &[lp(&mut id), mp, tp, lp(&mut dist), lp(&mut flags)]);
    if n != 5 || f(dist) < 4.0 {
        if sscanf_n(line, s(0x868B90), &[lp(&mut id), mp, tp, lp(&mut count)]) != 4 {
            return -1;
        }
        match count {
            1 => {
                sscanf_n(line, s(0x868B4C), &[lp(&mut id), mp, tp, lp(&mut count), lp(&mut dist), lp(&mut flags)]);
            }
            2 => {
                sscanf_n(line, s(0x868B60), &[lp(&mut id), mp, tp, lp(&mut count), lp(&mut dist), lp(&mut d2), lp(&mut flags)]);
            }
            3 => {
                sscanf_n(line, s(0x868B78), &[lp(&mut id), mp, tp, lp(&mut count), lp(&mut dist), lp(&mut d2), lp(&mut d3), lp(&mut flags)]);
            }
            _ => {}
        }
    }
    let mi = if flags & 0x1000 == 0 { add_atomic_model(id) } else { add_damage_atomic_model(id) };
    wr32(mi, 0x18, dist);
    wr32(mi, 4, get_uppercase_key(mp));
    set_tex_dictionary(mi, tp);
    set_atomic_model_info_flags(mi, flags);
    id
}

/// `CFileLoader::LoadTimeObject` (0x5B3DE0). gr-deviations `FileLoader.cpp:1777` (keep-original):
/// counts other than 1..3 keep the first parse; the locals are not initialised (0 here).
#[hook(addr = 0x5B3DE0, conv = cdecl, name = "CFileLoader::LoadTimeObject", expect = "83 EC 50 56 8B 74 24 58")]
pub fn load_time_object(line: GPtr<u8>) -> i32 {
    let mut id = 0i32;
    let mut model = [0u8; 24 + 64];
    let mut txd = [0u8; 24 + 64];
    let mp = GPtr::from_raw(model.as_mut_ptr());
    let tp = GPtr::from_raw(txd.as_mut_ptr());
    let (mut dist, mut flags, mut on, mut off) = (0u32, 0u32, 0u32, 0u32);
    let (mut count, mut d2, mut d3) = (0i32, 0u32, 0u32);
    let n = sscanf_n(line, s(0x868C00), &[lp(&mut id), mp, tp, lp(&mut dist), lp(&mut flags), lp(&mut on), lp(&mut off)]);
    if n != 7 || f(dist) < 4.0 {
        if sscanf_n(line, s(0x868B90), &[lp(&mut id), mp, tp, lp(&mut count)]) != 4 {
            return -1;
        }
        match count {
            1 => {
                sscanf_n(line, s(0x868BAC), &[lp(&mut id), mp, tp, lp(&mut count), lp(&mut dist), lp(&mut flags), lp(&mut on), lp(&mut off)]);
            }
            2 => {
                sscanf_n(line, s(0x868BC4), &[lp(&mut id), mp, tp, lp(&mut count), lp(&mut dist), lp(&mut d2), lp(&mut flags), lp(&mut on), lp(&mut off)]);
            }
            3 => {
                sscanf_n(line, s(0x868BE0), &[lp(&mut id), mp, tp, lp(&mut count), lp(&mut dist), lp(&mut d2), lp(&mut d3), lp(&mut flags), lp(&mut on), lp(&mut off)]);
            }
            _ => {}
        }
    }
    let mi = add_time_model(id);
    let ti = v_get_time_info(mi);
    wr32(mi, 0x18, dist);
    wr32(mi, 4, get_uppercase_key(mp));
    set_tex_dictionary(mi, tp);
    wr8(ti, 0, on as u8);
    wr8(ti, 1, off as u8);
    set_atomic_model_info_flags(mi, flags);
    let other = find_other_time_model(ti, mp);
    if !other.is_null() {
        wr16(other, 2, id as i16);
    }
    id
}

/// `CFileLoader::LoadWeaponObject` (0x5B3FB0): `"%d %s %s %s %d %f"`; weapon col model.
#[hook(addr = 0x5B3FB0, conv = cdecl, name = "CFileLoader::LoadWeaponObject", expect = "83 EC 4C 56 8D 44 24 08")]
pub fn load_weapon_object(line: GPtr<u8>) -> i32 {
    let mut id = 0i32;
    let mut model = [0u8; 24 + 64];
    let mut txd = [0u8; 24 + 64];
    let mut anim = [0u8; 16 + 64];
    let mut objs = 0i32;
    let mut dist = 0u32;
    let mp = GPtr::from_raw(model.as_mut_ptr());
    let tp = GPtr::from_raw(txd.as_mut_ptr());
    let ap = GPtr::from_raw(anim.as_mut_ptr());
    sscanf_n(line, s(0x868C18), &[lp(&mut id), mp, tp, ap, lp(&mut objs), lp(&mut dist)]);
    let mi = add_weapon_model(id);
    wr32(mi, 4, get_uppercase_key(mp));
    wr32(mi, 0x18, dist);
    set_tex_dictionary(mi, tp);
    v_set_anim_file(mi, ap);
    set_col_model(mi, s(0x968FD0), 0);
    id
}

/// `CFileLoader::LoadClumpObject` (0x5B4040): `"%d %s %s"`; bounding-box col model.
#[hook(addr = 0x5B4040, conv = cdecl, name = "CFileLoader::LoadClumpObject", expect = "83 EC 34 8D 44 24 1C")]
pub fn load_clump_object(line: GPtr<u8>) -> i32 {
    let mut id = -1i32;
    let mut model = [0u8; 24 + 64];
    let mut txd = [0u8; 24 + 64];
    let mp = GPtr::from_raw(model.as_mut_ptr());
    let tp = GPtr::from_raw(txd.as_mut_ptr());
    if sscanf_n(line, s(0x868C2C), &[lp(&mut id), mp, tp]) != 3 {
        return -1;
    }
    let mi = add_clump_model(id);
    wr32(mi, 4, get_uppercase_key(mp));
    set_tex_dictionary(mi, tp);
    set_col_model(mi, s(0x968A00), 0);
    id
}

/// `CFileLoader::LoadAnimatedClumpObject` (0x5B40C0): anim "null" by default, draw distance 2000;
/// flag bit 0x20 → +0x13 bit 2 set/cleared (the original tests the flags left in EDX by the
/// SetBaseModelInfoFlags call setup); anim != "null" → +0x13 bit 0.
#[hook(addr = 0x5B40C0, conv = cdecl, name = "CFileLoader::LoadAnimatedClumpObject", expect = "83 EC 4C A1 DC BB 85 00")]
pub fn load_animated_clump_object(line: GPtr<u8>) -> i32 {
    let mut id = -1i32;
    let mut model = [0u8; 24 + 64];
    let mut txd = [0u8; 24 + 64];
    let mut anim = [0u8; 16 + 64];
    anim[..5].copy_from_slice(b"null\0");
    let mut dist = 0x44FA_0000u32;
    let mut flags = 0u32;
    let mp = GPtr::from_raw(model.as_mut_ptr());
    let tp = GPtr::from_raw(txd.as_mut_ptr());
    let ap = GPtr::from_raw(anim.as_mut_ptr());
    if sscanf_n(line, s(0x868C38), &[lp(&mut id), mp, tp, ap, lp(&mut dist), lp(&mut flags)]) != 6 {
        return -1;
    }
    let mi = add_clump_model(id);
    wr32(mi, 4, get_uppercase_key(mp));
    set_tex_dictionary(mi, tp);
    wr32(mi, 0x18, dist);
    v_set_anim_file(mi, ap);
    set_base_model_info_flags(mi, flags);
    if flags & 0x20 != 0 {
        wr8(mi, 0x13, rd8(mi, 0x13) | 4);
    } else {
        wr8(mi, 0x13, rd8(mi, 0x13) & 0xFB);
    }
    if !cmpsb_eq(ap, b"null\0") {
        wr8(mi, 0x13, rd8(mi, 0x13) | 1);
    }
    id
}

// ----------------------------------------------------------------------------- collision ---

orig! {
    /// CMemoryMgr::Malloc (0x72F420)
    fn malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CCollisionData::CCollisionData (0x40F030)
    fn collision_data_ctor(cd: GPtr<u8>) -> GPtr<u8> = thiscall 0x40F030 => unported;
    /// CColSphere::Set(radius, center, material, flags, lighting) (0x40FD10)
    fn col_sphere_set(s: GPtr<u8>, radius: u32, center: GPtr<u8>, mat: u32, flags: u32, light: u32) = thiscall 0x40FD10 => unported;
    /// CColBox::Set(min, max, material, flags, lighting) (0x40EE70)
    fn col_box_set(b: GPtr<u8>, min: GPtr<u8>, max: GPtr<u8>, mat: u32, flags: u32, light: u32) = thiscall 0x40EE70 => unported;
    /// CCollisionData::GetNoOfShadowVerts (0x537510)
    fn get_no_of_shadow_verts(cd: GPtr<u8>) -> u32 = thiscall 0x537510 => unported;
    /// CColModel::operator new (0x40FC30)
    fn col_model_new(size: u32) -> GPtr<u8> = cdecl 0x40FC30 => unported;
    /// CColModel::CColModel (0x40FB60)
    fn col_model_ctor(cm: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// CPlantMgr::SetPlantFriendlyFlagInAtomicMI (0x5DB650)
    fn set_plant_friendly_flag(mi: GPtr<u8>) = cdecl 0x5DB650 => unported;
    /// CColStore::IncludeModelIndex (0x410820)
    fn col_store_include_model_index(slot: i32, id: i32) = cdecl 0x410820 => unported;
    /// CColAccel::addCacheCol (0x5B2C20)
    fn col_accel_add_cache_col(id: i32, cm: GPtr<u8>) = cdecl 0x5B2C20 => unported;
    fn load_col_v1_call(data: GPtr<u8>, cm: GPtr<u8>, name: GPtr<u8>) = cdecl 0x537580 => load_collision_model_3;
    fn load_col_v2_call(data: GPtr<u8>, size: u32, cm: GPtr<u8>, name: GPtr<u8>) = cdecl 0x537EE0 => load_collision_model_ver2;
    fn load_col_v3_call(data: GPtr<u8>, size: u32, cm: GPtr<u8>, name: GPtr<u8>) = cdecl 0x537CE0 => load_collision_model_ver3;
    fn load_col_v4_call(data: GPtr<u8>, size: u32, cm: GPtr<u8>, name: GPtr<u8>) = cdecl 0x537AE0 => load_collision_model_ver4;
}

// phase B/C target of an `orig!` declaration (unused in phase A)
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn load_collision_model_3(data: GPtr<u8>, cm: GPtr<u8>, _name: GPtr<u8>) {
    load_collision_model(data, cm)
}

/// `_ftol(f * 128.0f)` low word (vertex compression of the COLL loader; the product is exact).
pub fn compress_vertex_component(v: f32) -> i16 {
    X87::from_f32(v).fmul_m(128.0).ftol() as u16 as i16
}

/// `CFileLoader::LoadCollisionModel(uint8*, CColModel&)` (0x537580, COLL version 1). Ported from the
/// bytes (the entry is a protection chunk that pushes the EH handler): bounds (radius, centre, min,
/// max), `new CCollisionData`, spheres (i16 count, CColSphere::Set), lines skipped (count byte,
/// +6 and +0x10 cleared), boxes (i16 count, CColBox::Set), vertices (i32 count, `_ftol(v * 128)`),
/// faces (i16 count; a, b, c as words of three dwords, material +0xC, light +0xF). gr-deviations
/// `FileLoader.cpp:637/672` (harmless).
#[hook(addr = 0x537580, conv = cdecl, name = "CFileLoader::LoadCollisionModel", expect = "6A FF E9 DE 9A EC FF")]
pub fn load_collision_model(data: GPtr<u8>, cm: GPtr<u8>) {
    let mut p = data;
    wr32(cm, 0x24, rd32(p, 0));
    p = p.byte_add(4);
    for k in 0..3 {
        wr32(cm, 0x18 + 4 * k, rd32(p, 4 * k));
    }
    p = p.byte_add(12);
    for k in 0..3 {
        wr32(cm, 4 * k, rd32(p, 4 * k));
    }
    p = p.byte_add(12);
    for k in 0..3 {
        wr32(cm, 0xC + 4 * k, rd32(p, 4 * k));
    }
    p = p.byte_add(12);
    let mem = operator_new(0x30);
    let cd = if mem.is_null() { GPtr::NULL } else { collision_data_ctor(mem) };
    wrp(cm, 0x2C, cd);
    // spheres
    let n = rd16(p, 0);
    p = p.byte_add(4);
    wr16(cd, 0, n);
    if n > 0 {
        let a = malloc((n as i32 * 0x14) as u32, 0);
        wrp(cd, 8, a);
        let mut i = 0;
        while i < rd16(cd, 0) as i32 {
            col_sphere_set(
                a.byte_add(0x14 * i as isize),
                rd32(p, 0),
                p.byte_add(4),
                rd8(p, 0x10) as u32,
                rd8(p, 0x11) as u32,
                rd8(p, 0x13) as u32,
            );
            p = p.byte_add(0x14);
            i += 1;
        }
    } else {
        wrp(cd, 8, GPtr::NULL);
    }
    // lines: count byte, skipped
    let nl = rd8(p, 0);
    p = p.byte_add(4);
    wr8(cd, 6, nl);
    if (nl as i8) > 0 {
        p = p.byte_add(24 * nl as i8 as isize);
    } else {
        wrp(cd, 0x10, GPtr::NULL);
    }
    wr8(cd, 6, 0);
    wrp(cd, 0x10, GPtr::NULL);
    // boxes
    let n = rd16(p, 0);
    p = p.byte_add(4);
    wr16(cd, 2, n);
    if n > 0 {
        let a = malloc((n as i32 * 0x1C) as u32, 0);
        wrp(cd, 0xC, a);
        let mut i = 0;
        while i < rd16(cd, 2) as i32 {
            col_box_set(
                a.byte_add(0x1C * i as isize),
                p,
                p.byte_add(0xC),
                rd8(p, 0x18) as u32,
                rd8(p, 0x19) as u32,
                rd8(p, 0x1B) as u32,
            );
            p = p.byte_add(0x1C);
            i += 1;
        }
    } else {
        wrp(cd, 0xC, GPtr::NULL);
    }
    // vertices
    let nv = rd32(p, 0) as i32;
    p = p.byte_add(4);
    if nv > 0 {
        let a = malloc((nv.wrapping_mul(6)) as u32, 0);
        wrp(cd, 0x14, a);
        for i in 0..nv as isize {
            wr16(a, 6 * i, compress_vertex_component(f(rd32(p, 0))));
            wr16(a, 6 * i + 2, compress_vertex_component(f(rd32(p, 4))));
            wr16(a, 6 * i + 4, compress_vertex_component(f(rd32(p, 8))));
            p = p.byte_add(12);
        }
    } else {
        wrp(cd, 0x14, GPtr::NULL);
    }
    // faces
    let n = rd16(p, 0);
    p = p.byte_add(4);
    wr16(cd, 4, n);
    if n > 0 {
        let a = malloc((n as i32 * 8) as u32, 0);
        wrp(cd, 0x18, a);
        let mut i = 0i32;
        while i < rd16(cd, 4) as i32 {
            let t = a.byte_add(8 * i as isize);
            wr16(t, 0, rd16(p, 0));
            wr16(t, 2, rd16(p, 4));
            wr16(t, 4, rd16(p, 8));
            wr8(t, 6, rd8(p, 0xC));
            wr8(t, 7, rd8(p, 0xF));
            p = p.byte_add(0x10);
            i += 1;
        }
        wr16(cd, 4, i as i16);
    } else {
        wrp(cd, 0x18, GPtr::NULL);
    }
    wr8(cd, 7, rd8(cd, 7) & 0xFB);
    wrp(cd, 0x2C, GPtr::NULL);
    wrp(cd, 0x28, GPtr::NULL);
    wr32(rdp(cm, 0x2C), 0x24, 0);
    if rd16(cd, 0) != 0 || rd16(cd, 2) != 0 || rd16(cd, 4) != 0 {
        wr8(cm, 0x29, rd8(cm, 0x29) | 1);
    }
}

/// Header fields shared by COL2/3/4: bounds, flag bit 0 of cm+0x29, data copy into Malloc'd memory
/// behind a 0x30-byte CCollisionData (not constructed), counts and the face-group flag. Returns the
/// collision data (null when the file has no data).
fn col_v234_common(data: GPtr<u8>, size: u32, cm: GPtr<u8>, hdr_len: u32) -> GPtr<u8> {
    // header: box (0x00..0x18), sphere (0x18..0x28), counts at 0x28.., flags 0x30, offsets 0x34..
    for k in 0..4 {
        wr32(cm, 0x18 + 4 * k, rd32(data, 0x18 + 4 * k));
    }
    for k in 0..6 {
        wr32(cm, 4 * k, rd32(data, 4 * k));
    }
    let hflags = rd8(data, 0x30);
    let b = rd8(cm, 0x29);
    wr8(cm, 0x29, b ^ ((hflags >> 1 ^ b) & 1));
    let len = size.wrapping_sub(hdr_len);
    if len == 0 {
        return GPtr::NULL;
    }
    let p = malloc(len.wrapping_add(0x30), 0);
    wrp(cm, 0x2C, p);
    let src = data.byte_add(hdr_len as isize);
    for k in 0..len as isize {
        wr8(p, 0x30 + k, rd8(src, k));
    }
    let cd = rdp(cm, 0x2C);
    wr16(cd, 0, rd16(data, 0x28));
    wr16(rdp(cm, 0x2C), 2, rd16(data, 0x2A));
    wr8(rdp(cm, 0x2C), 6, rd8(data, 0x2E));
    wr16(rdp(cm, 0x2C), 4, rd16(data, 0x2C));
    p
}

fn col_ptr(cd: GPtr<u8>, off: isize, file_off: u32, base: GPtr<u8>, bias: isize) {
    if file_off == 0 {
        wrp(cd, off, GPtr::NULL);
    } else {
        wrp(cd, off, base.byte_add(file_off as i32 as isize + bias));
    }
}

/// `CFileLoader::LoadCollisionModelVer2` (0x537EE0, COL2): gr-deviations `FileLoader.cpp:719/729/
/// 730/776` (keep-original): no CCollisionData construction, +0x20 not written.
#[hook(addr = 0x537EE0, conv = cdecl, name = "CFileLoader::LoadCollisionModelVer2", expect = "83 EC 4C 53 56")]
pub fn load_collision_model_ver2(data: GPtr<u8>, size: u32, cm: GPtr<u8>, _name: GPtr<u8>) {
    let p = col_v234_common(data, size, cm, 0x4C);
    if !p.is_null() {
        let hflags = rd8(data, 0x30);
        let cd = || rdp(cm, 0x2C);
        wr8(cd(), 7, rd8(cd(), 7) & 0xFE);
        wr8(cd(), 7, rd8(cd(), 7) & 0xFB);
        let b = rd8(cd(), 7);
        wr8(cd(), 7, b ^ (((hflags >> 2) ^ b) & 2));
        col_ptr(cd(), 8, rd32(data, 0x34), p, -0x38);
        col_ptr(cd(), 0xC, rd32(data, 0x38), p, -0x38);
        col_ptr(cd(), 0x10, rd32(data, 0x3C), p, -0x38);
        col_ptr(cd(), 0x14, rd32(data, 0x40), p, -0x38);
        col_ptr(cd(), 0x18, rd32(data, 0x44), p, -0x38);
        wr32(cd(), 0x1C, 0);
        wr8(cd(), 7, rd8(cd(), 7) & 0xFB);
        wr32(cd(), 0x28, 0);
        wr32(cd(), 0x2C, 0);
        wr32(cd(), 0x24, 0);
        wr8(cm, 0x29, rd8(cm, 0x29) | 2);
    }
}

/// COL3/COL4 tail: +0x20 shadow faces, shadow pointers, has-shadow flag, shadow vertex count.
fn col_v34(data: GPtr<u8>, size: u32, cm: GPtr<u8>, hdr_len: u32, bias: isize) {
    let p = col_v234_common(data, size, cm, hdr_len);
    if p.is_null() {
        return;
    }
    let hflags = rd8(data, 0x30);
    let cd = || rdp(cm, 0x2C);
    wr8(cd(), 7, rd8(cd(), 7) & 0xFE);
    let b = rd8(cd(), 7);
    wr8(cd(), 7, b ^ (((hflags >> 2) ^ b) & 2));
    let n_shadow_faces = rd32(data, 0x4C) as i32;
    wr32(cd(), 0x20, n_shadow_faces as u32);
    col_ptr(cd(), 8, rd32(data, 0x34), p, bias);
    col_ptr(cd(), 0xC, rd32(data, 0x38), p, bias);
    col_ptr(cd(), 0x10, rd32(data, 0x3C), p, bias);
    col_ptr(cd(), 0x14, rd32(data, 0x40), p, bias);
    col_ptr(cd(), 0x18, rd32(data, 0x44), p, bias);
    let off_sv = rd32(data, 0x50);
    let off_sf = rd32(data, 0x54);
    col_ptr(cd(), 0x28, off_sv, p, bias);
    col_ptr(cd(), 0x2C, off_sf, p, bias);
    if off_sf != 0 && off_sv != 0 && n_shadow_faces > 0 {
        wr8(cd(), 7, rd8(cd(), 7) | 4);
    } else {
        wr8(cd(), 7, rd8(cd(), 7) & 0xFB);
    }
    let c = cd();
    if rd8(c, 7) & 4 == 0 {
        wr32(c, 0x24, 0);
    } else {
        let n = get_no_of_shadow_verts(c);
        wr32(c, 0x24, n);
    }
    wr32(cd(), 0x1C, 0);
    wr8(cm, 0x29, rd8(cm, 0x29) | 2);
}

/// `CFileLoader::LoadCollisionModelVer3` (0x537CE0, COL3): offsets biased by -0x44; has-shadow when
/// both shadow offsets are set and the face count is > 0 (signed).
#[hook(addr = 0x537CE0, conv = cdecl, name = "CFileLoader::LoadCollisionModelVer3", expect = "83 EC 58 53 55")]
pub fn load_collision_model_ver3(data: GPtr<u8>, size: u32, cm: GPtr<u8>, _name: GPtr<u8>) {
    col_v34(data, size, cm, 0x58, -0x44);
}

/// `CFileLoader::LoadCollisionModelVer4` (0x537AE0, COL4): as COL3 with a 0x5C-byte header (bias
/// -0x48).
#[hook(addr = 0x537AE0, conv = cdecl, name = "CFileLoader::LoadCollisionModelVer4", expect = "83 EC 5C 53 55")]
pub fn load_collision_model_ver4(data: GPtr<u8>, size: u32, cm: GPtr<u8>, _name: GPtr<u8>) {
    col_v34(data, size, cm, 0x5C, -0x48);
}

const FOURCC_COLL: u32 = 0x4C4C_4F43;
const FOURCC_COL2: u32 = 0x324C_4F43;
const FOURCC_COL3: u32 = 0x334C_4F43;
const FOURCC_COL4: u32 = 0x344C_4F43;

/// A new CColModel from its pool (constructed; null when the pool is full).
fn new_col_model() -> GPtr<u8> {
    let m = col_model_new(0x30);
    if m.is_null() { GPtr::NULL } else { col_model_ctor(m) }
}

/// `CFileLoader::LoadCollisionFile(uint8*, uint32, uint8 colId)` (0x538440): the model-info register
/// is not cleared per entry (ids >= 20000 reuse the previous one for the key test); stops at an
/// unknown fourcc or when 8 or fewer bytes remain (unsigned). gr-deviations `FileLoader.cpp:493/511`.
#[hook(addr = 0x538440, conv = cdecl, name = "CFileLoader::LoadCollisionFile-Buffer", expect = "64 A1 00 00 00 00")]
pub fn load_collision_file_buffer(buf: GPtr<u8>, size: u32, col_id: u8) -> bool {
    let mut mi: GPtr<u8> = GPtr::NULL;
    let mut p = buf;
    let mut remaining = size;
    let mut name = [0u8; 24];
    while remaining > 8 {
        let fourcc = rd32(p, 0);
        let fsize = rd32(p, 4);
        let version = match fourcc {
            FOURCC_COLL => 0,
            FOURCC_COL2 => 1,
            FOURCC_COL3 => 2,
            FOURCC_COL4 => 3,
            _ => return true,
        };
        let data = p.byte_add(8);
        for (k, b) in name.iter_mut().enumerate().take(22) {
            *b = rd8(data, k as isize);
        }
        let id = rd16(data, 0x16) as u16 as i32;
        let after = data.byte_add(0x18);
        if id < 20000 {
            mi = model_info(id).cast();
        }
        let np = GPtr::from_raw(name.as_mut_ptr());
        if mi.is_null() || rd32(mi, 4) != get_uppercase_key(np) {
            let pool = crate::shared::COL_POOL.get().cast::<u8>();
            let map = rdp(pool, 4);
            let def = |c: u8| {
                if (rd8(map, c as isize) as i8) < 0 { GPtr::<u8>::NULL } else { rdp(pool, 0).byte_add(0x2C * c as isize) }
            };
            let end = rd16(def(col_id), 0x24) as i32;
            let start = rd16(def(col_id), 0x22) as i32;
            mi = get_model_info_min_max(np, start, end);
        }
        if !mi.is_null() && rd8(mi, 0x12) & 0x80 != 0 {
            let mut col = rdp(mi, 0x14);
            if col.is_null() {
                col = new_col_model();
                set_col_model(mi, col, 1);
            }
            let len = fsize.wrapping_sub(0x18);
            match version {
                0 => load_col_v1_call(after, col, np),
                1 => load_col_v2_call(after, len, col, np),
                2 => load_col_v3_call(after, len, col, np),
                _ => load_col_v4_call(after, len, col, np),
            }
            wr8(col, 0x28, col_id);
            if v_get_model_type(mi) == 1 {
                set_plant_friendly_flag(mi);
            }
        }
        p = after.byte_add(fsize.wrapping_sub(0x18) as i32 as isize);
        remaining = remaining.wrapping_sub(8).wrapping_sub(fsize);
    }
    true
}

/// `CFileLoader::LoadCollisionFile(filename, uint8 colId)` (0x5B4E60). Keep-original quirks (spec
/// CFileLoader.deviations.md): the version is set only for COLL/COL2/COL3 (others keep the
/// previous entry's; there is no COL4 call); *new*: when a model has no col model yet, a new one is
/// created and attached but the data is loaded into the PREVIOUS entry's col model (EDI is not
/// updated, 0x5B4F2C-0x5B4F5E), which also gets the slot id. The first entry's version and target
/// are uninitialised locals in the original (0 / null here).
#[hook(addr = 0x5B4E60, conv = cdecl, name = "CFileLoader::LoadCollisionFile-File", expect = "64 A1 00 00 00 00")]
pub fn load_collision_file(filename: GPtr<u8>, col_id: u8) {
    let f = open_file(filename, s(0x859C6C));
    let mut hdr = [0u32; 2];
    let mut name = [0u8; 24];
    let mut version = 0;
    let mut col: GPtr<u8> = GPtr::NULL;
    let buf = COL_FILE_BUFFER.ptr().cast::<u8>();
    while read(f, GPtr::from_raw(hdr.as_mut_ptr()).cast(), 8) != 0 {
        match hdr[0] {
            FOURCC_COL2 => version = 1,
            FOURCC_COL3 => version = 2,
            FOURCC_COLL => version = 0,
            _ => {}
        }
        read(f, GPtr::from_raw(name.as_mut_ptr()), 0x18);
        read(f, buf, hdr[1].wrapping_sub(0x18));
        let np = GPtr::from_raw(name.as_mut_ptr());
        let mi = get_model_info_full(np, GPtr::NULL);
        if mi.is_null() || rd8(mi, 0x12) & 0x80 == 0 {
            continue;
        }
        let existing = rdp(mi, 0x14);
        if !existing.is_null() {
            col = existing;
        } else {
            let c = new_col_model();
            set_col_model(mi, c, 1);
        }
        match version {
            0 => load_col_v1_call(buf, col, np),
            1 => load_col_v2_call(buf, hdr[1].wrapping_sub(0x18), col, np),
            2 => load_col_v3_call(buf, hdr[1].wrapping_sub(0x18), col, np),
            _ => {}
        }
        wr8(col, 0x28, col_id);
    }
    close_file(f);
}

/// `CFileLoader::LoadCollisionFileFirstTime(uint8*, uint32, uint8 colId)` (0x5B5000): COLL/COL2/COL3
/// only; every found model is included in the col slot; LOD-flagged ones get a new col model
/// (loaded, attached, cached in CColAccel). The model-info local persists across entries.
#[hook(addr = 0x5B5000, conv = cdecl, name = "CFileLoader::LoadCollisionFileFirstTime", expect = "6A FF 68 1B D5 83 00")]
pub fn load_collision_file_first_time(buf: GPtr<u8>, size: u32, col_id: u8) -> bool {
    let mut mi: GPtr<u8> = GPtr::NULL;
    let mut p = buf;
    let mut remaining = size;
    let mut name = [0u8; 24];
    while remaining > 8 {
        let version = match rd32(p, 0) {
            FOURCC_COL2 => 1,
            FOURCC_COL3 => 2,
            FOURCC_COLL => 0,
            _ => return true,
        };
        let fsize = rd32(p, 4);
        let data = p.byte_add(8);
        for (k, b) in name.iter_mut().enumerate().take(22) {
            *b = rd8(data, k as isize);
        }
        let mut id = rd16(data, 0x16) as u16 as i32;
        let after = data.byte_add(0x18);
        if id < 20000 {
            mi = model_info(id).cast();
        }
        let np = GPtr::from_raw(name.as_mut_ptr());
        if mi.is_null() || rd32(mi, 4) != get_uppercase_key(np) {
            mi = get_model_info_full(np, lp(&mut id));
        }
        if !mi.is_null() {
            col_store_include_model_index(col_id as i32, id);
            if rd8(mi, 0x12) & 0x80 != 0 {
                let col = new_col_model();
                let len = fsize.wrapping_sub(0x18);
                match version {
                    0 => load_col_v1_call(after, col, np),
                    1 => load_col_v2_call(after, len, col, np),
                    _ => load_col_v3_call(after, len, col, np),
                }
                wr8(col, 0x28, col_id);
                set_col_model(mi, col, 1);
                col_accel_add_cache_col(id, col);
            }
        }
        remaining = remaining.wrapping_sub(8).wrapping_sub(fsize);
        p = after.byte_add(fsize.wrapping_sub(0x18) as i32 as isize);
    }
    true
}

// ------------------------------------------------------------------ vehicle / ped objects ---

orig! {
    /// cHandlingDataMgr::GetHandlingId (0x6F4FD0), this = mod_HandlingManager (0xC2B9C8)
    fn get_handling_id(mgr: GPtr<u8>, name: GPtr<u8>) -> i32 = thiscall 0x6F4FD0 => unported;
    /// CPedType::FindPedType (0x608790)
    fn find_ped_type(name: GPtr<u8>) -> i32 = cdecl 0x608790 => unported;
    /// CPedStats::GetPedStatType (0x6088D0)
    fn get_ped_stat_type(name: GPtr<u8>) -> i32 = cdecl 0x6088D0 => unported;
    /// CAnimManager::GetAnimGroupName (0x4D3A20)
    fn get_anim_group_name(i: i32) -> GPtr<u8> = cdecl 0x4D3A20 => unported;
    /// CPopulation::FindPedRaceFromName (0x5B6D40)
    fn find_ped_race_from_name(name: GPtr<u8>) -> u8 = cdecl 0x5B6D40 => unported;
    /// CAEPedSpeechAudioEntity::GetAudioPedType (0x4E3C60)
    fn get_audio_ped_type(name: GPtr<u8>) -> i32 = cdecl 0x4E3C60 => unported;
    /// CAEPedSpeechAudioEntity::GetVoice (0x4E3CD0)
    fn get_voice(name: GPtr<u8>, ty: i32) -> i16 = cdecl 0x4E3CD0 => unported;
}

/// `CFileLoader::LoadVehicleObject` (0x5B6F30). gr-deviations `FileLoader.cpp:1811/1845/1846/1854/
/// 1899/1900` (keep-original): wheel sizes are not defaulted (0 here; the original's are stack
/// contents for lines without them), f_heli → 3 (HELI), unknown type / class leave the field; the
/// first game-name character is never turned from '_' into ' '.
#[hook(addr = 0x5B6F30, conv = cdecl, name = "CFileLoader::LoadVehicleObject", expect = "81 EC A8 00 00 00")]
pub fn load_vehicle_object(line: GPtr<u8>) -> i32 {
    let mut id = -1i32;
    let mut model = [0u8; 24 + 64];
    let mut txd = [0u8; 24 + 64];
    let mut ty = [0u8; 8 + 64];
    let mut handling = [0u8; 16 + 64];
    let mut game_name = [0u8; 32 + 64];
    let mut anims = [0u8; 16 + 64];
    let mut class = [0u8; 16 + 64];
    let (mut frq, mut flags, mut comps) = (0u32, 0u32, 0u32);
    let mut misc = 0i32;
    let (mut wf, mut wr) = (0u32, 0u32);
    let mut upgrade_class = -1i32;
    let mut slot = find_txd_slot(s(0x86982C));
    if slot == -1 {
        slot = add_txd_slot(s(0x86982C));
    }
    let p = |b: &mut [u8]| GPtr::from_raw(b.as_mut_ptr());
    sscanf_n(
        line,
        s(0x8697FC),
        &[
            lp(&mut id),
            p(&mut model),
            p(&mut txd),
            p(&mut ty),
            p(&mut handling),
            p(&mut game_name),
            p(&mut anims),
            p(&mut class),
            lp(&mut frq),
            lp(&mut flags),
            lp(&mut comps),
            lp(&mut misc),
            lp(&mut wf),
            lp(&mut wr),
            lp(&mut upgrade_class),
        ],
    );
    let mi = add_vehicle_model(id);
    wr32(mi, 4, get_uppercase_key(p(&mut model)));
    set_tex_dictionary(mi, p(&mut txd));
    let pool = crate::shared::TXD_POOL.get().cast::<u8>();
    let ti = rd16(mi, 0xA) as isize;
    let def = if (rd8(rdp(pool, 4), ti) as i8) < 0 { GPtr::NULL } else { rdp(pool, 0).byte_add(12 * ti) };
    wr16(def, 6, slot as i16);
    v_set_anim_file(mi, p(&mut anims));
    if game_name[0] != 0 {
        let mut k = 0usize;
        loop {
            if game_name[k + 1] == b'_' {
                game_name[k + 1] = b' ';
            }
            k += 1;
            if game_name[k] == 0 {
                break;
            }
        }
    }
    strncpy(mi.byte_add(0x32), p(&mut game_name), 8);
    wr8(mi, 0x4E, flags as u8);
    wr32(mi, 0x54, comps);
    let tp = p(&mut ty);
    let wheels = |mi: GPtr<u8>, index: bool| {
        if index {
            wr16(mi, 0x48, misc as i16);
        }
        wr32(mi, 0x40, wf);
        wr32(mi, 0x44, wr);
    };
    let typed: Option<(u32, u8)> = if cmpsb_eq(tp, b"car\0") {
        Some((0, 1))
    } else if cmpsb_eq(tp, b"mtruck\0") {
        Some((1, 1))
    } else if cmpsb_eq(tp, b"quad\0") {
        Some((2, 1))
    } else if cmpsb_eq(tp, b"heli\0") {
        Some((3, 1))
    } else if cmpsb_eq(tp, b"plane\0") {
        Some((4, 1))
    } else if cmpsb_eq(tp, b"boat\0") {
        Some((5, 0))
    } else if cmpsb_eq(tp, b"train\0") {
        Some((6, 0))
    } else if cmpsb_eq(tp, b"f_heli\0") {
        Some((3, 0))
    } else if cmpsb_eq(tp, b"f_plane\0") {
        Some((8, 2))
    } else if cmpsb_eq(tp, b"bike\0") {
        Some((9, 3))
    } else if cmpsb_eq(tp, b"bmx\0") {
        Some((10, 3))
    } else if cmpsb_eq(tp, b"trailer\0") {
        Some((11, 1))
    } else {
        None
    };
    if let Some((t, w)) = typed {
        match w {
            1 => {
                wr32(mi, 0x3C, t);
                wheels(mi, true);
            }
            2 => {
                wr16(mi, 0x48, misc as i16);
                wr32(mi, 0x40, 0x3F80_0000);
                wr32(mi, 0x44, 0x3F80_0000);
                wr32(mi, 0x3C, t);
            }
            3 => {
                wr32(mi, 0x3C, t);
                field::<f32, u8>(mi, 0x58).write(misc as f32);
                wheels(mi, false);
            }
            _ => wr32(mi, 0x3C, t),
        }
    }
    let h = get_handling_id(s(0xC2B9C8), p(&mut handling));
    wr16(mi, 0x4A, h as i16);
    wr8(mi, 0x4F, upgrade_class as u8);
    let cp = p(&mut class);
    const CLASSES: [&[u8]; 12] = [
        b"normal\0",
        b"poorfamily\0",
        b"richfamily\0",
        b"executive\0",
        b"worker\0",
        b"big\0",
        b"taxi\0",
        b"moped\0",
        b"motorbike\0",
        b"leisureboat\0",
        b"workerboat\0",
        b"bicycle\0",
    ];
    if let Some(k) = CLASSES.iter().position(|c| cmpsb_eq(cp, c)) {
        wr8(mi, 0x4D, k as u8);
    } else if cmpsb_eq(cp, b"ignore\0") {
        wr8(mi, 0x4D, 0xFF);
        return id;
    }
    wr16(mi, 0x52, frq as i16);
    id
}

/// `CFileLoader::LoadPedObject` (0x5B7420): `"%d %s %s %s %s %s %x %x %s %d %d %s %s %s"`; the anim
/// group is searched by exact name (index = count when missing); radios stored +1; the second voice
/// is copied to +0x42.
#[hook(addr = 0x5B7420, conv = cdecl, name = "CFileLoader::LoadPedObject", expect = "81 EC 28 01 00 00")]
pub fn load_ped_object(line: GPtr<u8>) -> i32 {
    let count = GPtr::<i32>::from_addr(0xB4EA28).read();
    let mut id = -1i32;
    let mut model = [0u8; 24 + 64];
    let mut txd = [0u8; 24 + 64];
    let mut ped_type = [0u8; 24 + 64];
    let mut stat = [0u8; 24 + 64];
    let mut anim_group = [0u8; 24 + 64];
    let (mut cars, mut flags) = (0u32, 0u32);
    let mut anim_file = [0u8; 16 + 64];
    let (mut radio1, mut radio2) = (0i32, 0i32);
    let mut audio = [0u8; 20 + 64];
    let mut voice1 = [0u8; 52 + 64];
    let mut voice2 = [0u8; 52 + 64];
    let p = |b: &mut [u8]| GPtr::from_raw(b.as_mut_ptr());
    sscanf_n(
        line,
        s(0x869834),
        &[
            lp(&mut id),
            p(&mut model),
            p(&mut txd),
            p(&mut ped_type),
            p(&mut stat),
            p(&mut anim_group),
            lp(&mut cars),
            lp(&mut flags),
            p(&mut anim_file),
            lp(&mut radio1),
            lp(&mut radio2),
            p(&mut audio),
            p(&mut voice1),
            p(&mut voice2),
        ],
    );
    let mi = add_ped_model(id);
    wr32(mi, 4, get_uppercase_key(p(&mut model)));
    set_tex_dictionary(mi, p(&mut txd));
    v_set_anim_file(mi, p(&mut anim_file));
    set_col_model(mi, s(0x968DF0), 0);
    wr32(mi, 0x28, find_ped_type(p(&mut ped_type)) as u32);
    wr32(mi, 0x2C, get_ped_stat_type(p(&mut stat)) as u32);
    let mut group = count;
    let mut i = 0;
    while i < GPtr::<i32>::from_addr(0xB4EA28).read() {
        let name = get_anim_group_name(i);
        let mut k = 0isize;
        let eq = loop {
            let a = anim_group[k as usize];
            if a != rd8(name, k) {
                break false;
            }
            if a == 0 {
                break true;
            }
            k += 1;
        };
        if eq {
            group = i;
            break;
        }
        i += 1;
    }
    wr32(mi, 0x24, group as u32);
    wr16(mi, 0x30, cars as i16);
    wr16(mi, 0x32, flags as i16);
    wr8(mi, 0x39, (radio2 as u8).wrapping_add(1));
    wr8(mi, 0x38, (radio1 as u8).wrapping_add(1));
    wr8(mi, 0x3A, find_ped_race_from_name(p(&mut model)));
    let at = get_audio_ped_type(p(&mut audio));
    wr16(mi, 0x3C, at as i16);
    wr16(mi, 0x3E, get_voice(p(&mut voice1), at));
    wr16(mi, 0x40, get_voice(p(&mut voice2), at));
    wr16(mi, 0x42, rd16(mi, 0x3E));
    id
}

// ------------------------------------------------------------------------------- 2d effects ---

orig! {
    /// CTxdStore::PushCurrentTxd (0x7316A0)
    fn push_current_txd() = cdecl 0x7316A0 => unported;
    /// CTxdStore::PopCurrentTxd (0x7316B0)
    fn pop_current_txd() = cdecl 0x7316B0 => unported;
    /// CTxdStore::SetCurrentTxd (0x7319C0)
    fn set_current_txd(slot: i32) = cdecl 0x7319C0 => unported;
    /// CModelInfo::Get2dEffectStore (0x4C5A60)
    fn get_2d_effect_store() -> GPtr<u8> = cdecl 0x4C5A60 => unported;
    /// CBaseModelInfo::Add2dEffect (0x4C4D20)
    fn add_2d_effect(mi: GPtr<u8>, fx: GPtr<u8>) = thiscall 0x4C4D20 => unported;
    /// RwTextureRead (0x7F3AC0)
    fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
}

/// `_ftol` of an f32 local, low byte (Load2dEffect interior sizes).
fn ftol_byte(v: u32) -> u8 {
    X87::from_f32(f(v)).ftol() as u8
}

/// `CFileLoader::Load2dEffect(line)` (0x5B7670, spec CFileLoader.md; not reversed in gr). The store
/// count is bumped and Add2dEffect called before any field is written; per-type parses write
/// straight into the effect where the original does; every path pops the TXD once. Uninitialised
/// locals of malformed lines are 0 here (spec: unspecified).
#[hook(addr = 0x5B7670, conv = cdecl, name = "CFileLoader::Load2dEffect", expect = "81 EC CC 01 00 00")]
pub fn load_2d_effect(line: GPtr<u8>) {
    let mut id = 0i32;
    let (mut x, mut y, mut z) = (0u32, 0u32, 0u32);
    let mut ty = 0i32;
    sscanf_n(line, s(0x869A30), &[lp(&mut id), lp(&mut x), lp(&mut y), lp(&mut z), lp(&mut ty)]);
    push_current_txd();
    set_current_txd(find_txd_slot(s(0x85C648)));
    let mi = model_info(id).cast::<u8>();
    let store = get_2d_effect_store();
    let n = rd32(store, 0);
    let fx = store.byte_add(4 + 0x40 * n as i32 as isize);
    wr32(store, 0, n.wrapping_add(1));
    add_2d_effect(mi, fx);
    wr32(fx, 0, x);
    wr32(fx, 4, y);
    wr32(fx, 8, z);
    wr8(fx, 0xC, ty as u8);
    let hdr = |id: &mut i32, x: &mut u32, y: &mut u32, z: &mut u32, ty: &mut i32| [lp(id), lp(x), lp(y), lp(z), lp(ty)];
    match ty as u32 {
        0 => {
            let mut c = [0i32; 4];
            let [r, g, b, a] = &mut c;
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            sscanf_n(line, s(0x8699E4), &[h[0], h[1], h[2], h[3], h[4], lp(r), lp(g), lp(b), lp(a)]);
            let mut corona = [0u8; 100];
            let mut shadow = [0u8; 100];
            let mut q = line;
            loop {
                let ch = rd8(q, 0);
                q = q.byte_add(1);
                if ch == b'"' {
                    break;
                }
            }
            let mut k = 0usize;
            while rd8(q, 0) != b'"' {
                corona[k] = rd8(q, 0);
                k += 1;
                q = q.byte_add(1);
            }
            corona[k] = 0;
            q = q.byte_add(1);
            loop {
                let ch = rd8(q, 0);
                q = q.byte_add(1);
                if ch == b'"' {
                    break;
                }
            }
            let mut k = 0usize;
            while rd8(q, 0) != b'"' {
                shadow[k] = rd8(q, 0);
                k += 1;
                q = q.byte_add(1);
            }
            shadow[k] = 0;
            let mut d = [0i32; 9];
            {
                let [d5, d6, d7, d8, d9, d10, d11, d12, d13] = &mut d;
                sscanf_n(
                    q.byte_add(1),
                    s(0x8699BC),
                    &[fx.byte_add(0x14), fx.byte_add(0x18), fx.byte_add(0x1C), fx.byte_add(0x20), lp(d5), lp(d6), lp(d7), lp(d8), lp(d9), lp(d10), lp(d11), lp(d12), lp(d13)],
                );
            }
            wr8(fx, 0x10, c[0] as u8);
            wr8(fx, 0x11, c[1] as u8);
            wr8(fx, 0x12, c[2] as u8);
            wr8(fx, 0x13, c[3] as u8);
            wr8(fx, 0x29, d[0] as u8);
            wr8(fx, 0x26, d[1] as u8);
            wr8(fx, 0x27, d[2] as u8);
            wr8(fx, 0x28, d[3] as u8);
            wr16(fx, 0x24, d[4] as i16);
            wr8(fx, 0x2A, d[5] as u8);
            wr8(fx, 0x2B, d[6] as u8);
            wr8(fx, 0x2C, d[7] as u8);
            wr8(fx, 0x2D, d[8] as u8);
            wrp(fx, 0x30, rw_texture_read(GPtr::from_raw(corona.as_mut_ptr()), GPtr::NULL));
            wrp(fx, 0x34, rw_texture_read(GPtr::from_raw(shadow.as_mut_ptr()), GPtr::NULL));
            if d[4] as u8 & 4 != 0 {
                wr8(fx, 0x24, rd8(fx, 0x24) & 0xFD);
            }
        }
        1 => {
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            sscanf_n(line, s(0x8699A8), &[h[0], h[1], h[2], h[3], h[4], fx.byte_add(0x10)]);
        }
        3 => {
            let (mut atype, mut d16, mut d17) = (0i32, 0i32, 0i32);
            let mut script = [0u8; 100];
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            let mut t = [GPtr::NULL; 18];
            t[..5].copy_from_slice(&h);
            t[5] = lp(&mut atype);
            for k in 0..9 {
                t[6 + k] = fx.byte_add(0x10 + 4 * k as isize);
            }
            t[15] = lp(&mut d16);
            t[16] = lp(&mut d17);
            t[17] = GPtr::from_raw(script.as_mut_ptr());
            sscanf_n(line, s(0x869970), &t);
            wr8(fx, 0x34, atype as u8);
            let mut k = 0;
            loop {
                let c = script[k];
                wr8(fx, 0x38 + k as isize, c);
                k += 1;
                if c == 0 {
                    break;
                }
            }
            wr8(fx, 0x35, d16 as u8);
            wr8(fx, 0x36, d17 as u8);
        }
        5 => {
            // locals of conversions #7..#38 (see spec); #6 and #10 go into the effect
            let mut l = [-1i32; 38];
            l[13] = 50; // #14 status
            for k in [6usize, 7, 8, 10, 11] {
                l[k] = 0; // #7, #8, #9, #11, #12 not preset in the original (stack contents)
            }
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            let mut t = [GPtr::NULL; 38];
            t[..5].copy_from_slice(&h);
            t[5] = fx.byte_add(0x10);
            t[9] = fx.byte_add(0x30);
            let base = GPtr::from_raw(l.as_mut_ptr()).cast::<u8>();
            for (k, slot) in t.iter_mut().enumerate().skip(6) {
                if k != 9 {
                    *slot = base.byte_add(4 * k as isize);
                }
            }
            sscanf_n(line, s(0x8698F8), &t);
            let c = |n: usize| l[n - 1];
            wr8(fx, 0x12, ftol_byte(c(7) as u32));
            wr8(fx, 0x13, ftol_byte(c(8) as u32));
            wr8(fx, 0x14, ftol_byte(c(9) as u32));
            let stores: [(isize, usize); 26] = [
                (0x11, 13),
                (0x15, 11),
                (0x16, 15),
                (0x17, 16),
                (0x18, 19),
                (0x19, 20),
                (0x1A, 17),
                (0x1B, 18),
                (0x1C, 21),
                (0x1D, 22),
                (0x1E, 25),
                (0x1F, 26),
                (0x20, 23),
                (0x21, 24),
                (0x22, 27),
                (0x23, 31),
                (0x24, 35),
                (0x25, 28),
                (0x26, 32),
                (0x27, 36),
                (0x28, 29),
                (0x29, 33),
                (0x2A, 37),
                (0x2B, 30),
                (0x2C, 34),
                (0x2D, 38),
            ];
            for (off, n) in stores {
                wr8(fx, off, c(n) as u8);
            }
            wr8(fx, 0x2E, c(12) as u8);
            wr8(fx, 0x2F, c(14) as u8);
        }
        6 => {
            let (mut d13, mut d14, mut d16) = (0i32, 0i32, 0i32);
            let mut name = [0u8; 100];
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            let mut t = [GPtr::NULL; 16];
            t[..5].copy_from_slice(&h);
            for k in 0..7 {
                t[5 + k] = fx.byte_add(0x10 + 4 * k as isize);
            }
            t[12] = lp(&mut d13);
            t[13] = lp(&mut d14);
            t[14] = GPtr::from_raw(name.as_mut_ptr());
            t[15] = lp(&mut d16);
            sscanf_n(line, s(0x8698C4), &t);
            wr16(fx, 0x2C, d13 as i16);
            wr8(fx, 0x2E, d14 as u8);
            wr8(fx, 0x2F, d16 as u8);
            strncpy(fx.byte_add(0x30), GPtr::from_raw(name.as_mut_ptr()), 8);
        }
        7 => {
            let mut text = [[0u8; 33 + 64]; 4];
            for t in text.iter_mut() {
                t[0] = b' ';
            }
            let mut v = [0u32; 5];
            let mut d11 = 0i32;
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            {
                let [f6, f7, f8, f9, f10] = &mut v;
                let [t1, t2, t3, t4] = &mut text;
                sscanf_n(
                    line,
                    s(0x869A00),
                    &[h[0], h[1], h[2], h[3], h[4], lp(f6), lp(f7), lp(f8), lp(f9), lp(f10), lp(&mut d11), GPtr::from_raw(t1.as_mut_ptr()), GPtr::from_raw(t2.as_mut_ptr()), GPtr::from_raw(t3.as_mut_ptr()), GPtr::from_raw(t4.as_mut_ptr())],
                );
            }
            for (k, w) in v.iter().enumerate() {
                wr32(fx, 0x10 + 4 * k as isize, *w);
            }
            wr16(fx, 0x24, d11 as i16);
            let m = call_malloc(0x40, 0);
            wrp(fx, 0x28, m);
            for (k, t) in text.iter().enumerate() {
                for (j, b) in t.iter().enumerate().take(16) {
                    wr8(m, (0x10 * k + j) as isize, *b);
                }
            }
            wrp(fx, 0x2C, GPtr::NULL);
        }
        8 => {
            let mut v = 0i32;
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            sscanf_n(line, s(0x8698B0), &[h[0], h[1], h[2], h[3], h[4], lp(&mut v)]);
            wr32(fx, 0x10, v as u32);
        }
        9 => {
            let (mut dx, mut dy, mut usage) = (0u32, 0u32, 0i32);
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            sscanf_n(line, s(0x869898), &[h[0], h[1], h[2], h[3], h[4], lp(&mut dx), lp(&mut dy), lp(&mut usage)]);
            wr32(fx, 0x10, dx);
            wr32(fx, 0x14, dy);
            wr8(fx, 0x18, usage as u8);
        }
        10 => {
            let mut v = [0u32; 9];
            let mut dir = 0i32;
            let h = hdr(&mut id, &mut x, &mut y, &mut z, &mut ty);
            {
                let [a, b, c, d, e, g, i, j, k] = &mut v;
                sscanf_n(
                    line,
                    s(0x869868),
                    &[h[0], h[1], h[2], h[3], h[4], lp(a), lp(b), lp(c), lp(d), lp(e), lp(g), lp(i), lp(j), lp(k), lp(&mut dir)],
                );
            }
            for (k, w) in v.iter().enumerate() {
                wr32(fx, 0x10 + 4 * k as isize, *w);
            }
            wr8(fx, 0x34, (dir != 0) as u8);
        }
        _ => {}
    }
    pop_current_txd();
}

orig! {
    /// CMemoryMgr::Malloc (0x72F420)
    fn call_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// CFileLoader::LoadObjectInstance(CFileObjectInstance*, name) (0x538090, blocked: _CIacos)
    fn load_object_instance_call(inst: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x538090 => unported;
    /// CEntryExitManager::AddOne (0x43FA00)
    fn entry_exit_add_one(a: f32, b: f32, c: f32, d: f32, e: f32, f2: f32, g: f32, h: f32, i: f32, j: f32, k: f32, area: i32, flags: i32, extra: i32, t1: i32, t2: i32, sky: i32, title: GPtr<u8>) -> i32 = cdecl 0x43FA00 => unported;
    /// CPickups::GenerateNewOne(CVector, model, type, ammo, money, revenue, forced, msg) (0x456F20)
    fn generate_new_pickup(x: f32, y: f32, z: f32, model: u32, ty: u32, a: u32, b: u32, c: u32, d: u32) -> i32 = cdecl 0x456F20 => unported;
    /// CColAccel::isCacheLoading (0x5B2AC0; the original reaches it through the protection thunk
    /// 0x40220F, which computes the state cell and tail-jumps into the body)
    fn col_accel_is_cache_loading() -> bool = cdecl 0x5B2AC0 => unported;
    /// CColAccel::addIPLEntity (0x5B3040)
    fn col_accel_add_ipl_entity(list: GPtr<u8>, count: i32, idx: u32) = cdecl 0x5B3040 => unported;
    /// CColAccel::cacheIPLSection (0x5B2F10)
    fn col_accel_cache_ipl_section(list: GPtr<u8>, count: i32) = cdecl 0x5B2F10 => unported;
    /// CEntity::SetupBigBuilding (0x533150)
    fn setup_big_building(e: GPtr<u8>) = thiscall 0x533150 => unported;
    /// CBaseModelInfo::DeleteCollisionModel (0x4C4C40)
    fn delete_collision_model(mi: GPtr<u8>) = thiscall 0x4C4C40 => unported;
    /// CWorld::Add (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
}

/// `CFileLoader::LoadObjectInstance(line)` (0x538690): `"%d %s %d %f %f %f %f %f %f %f %d"` into a
/// CFileObjectInstance on the stack (pos, rotation, model +0x1C, flags +0x20, lod +0x24), then the
/// struct overload.
#[hook(addr = 0x538690, conv = cdecl, name = "CFileLoader::LoadObjectInstance-file", expect = "83 EC 40 8D 44 24 3C")]
pub fn load_object_instance_line(line: GPtr<u8>) -> GPtr<u8> {
    let mut inst = [0u32; 10];
    let mut name = [0u8; 24 + 64];
    let b = GPtr::from_raw(inst.as_mut_ptr()).cast::<u8>();
    sscanf_n(
        line,
        s(0x863A08),
        &[
            b.byte_add(0x1C),
            GPtr::from_raw(name.as_mut_ptr()),
            b.byte_add(0x20),
            b,
            b.byte_add(4),
            b.byte_add(8),
            b.byte_add(0xC),
            b.byte_add(0x10),
            b.byte_add(0x14),
            b.byte_add(0x18),
            b.byte_add(0x24),
        ],
    );
    load_object_instance_call(b, GPtr::from_raw(name.as_mut_ptr()))
}

/// `CFileLoader::LoadEntryExit` (0x5B8030): 18 fields (sky 2, two times 0 and 0x18 preset); the
/// title is the text after the last '"' cut off (`strrchr`), else null; the flags are OR-ed into
/// the entry's +0x30 one bit at a time (bits 1 and 7 as byte ORs, the others as word ORs).
#[hook(addr = 0x5B8030, conv = cdecl, name = "CFileLoader::LoadEntryExit", expect = "83 EC 64 8D 44 24 08")]
pub fn load_entry_exit(line: GPtr<u8>) {
    let mut v = [0u32; 11];
    let (mut area, mut flags, mut extra) = (0i32, 0i32, 0i32);
    let mut sky = 2i32;
    let mut t1 = 0i32;
    let mut t2 = 0x18i32;
    let mut name = [0u8; 32 + 64];
    let np = GPtr::from_raw(name.as_mut_ptr());
    {
        let [a, b, c, d, e, g, h, i, j, k, l] = &mut v;
        sscanf_n(
            line,
            s(0x869A40),
            &[lp(a), lp(b), lp(c), lp(d), lp(e), lp(g), lp(h), lp(i), lp(j), lp(k), lp(l), lp(&mut area), lp(&mut flags), np, lp(&mut extra), lp(&mut sky), lp(&mut t1), lp(&mut t2)],
        );
    }
    let q = strrchr(np, b'"' as i32);
    let title = if q.is_null() {
        GPtr::NULL
    } else {
        wr8(q, 0, 0);
        np.byte_add(1)
    };
    let idx = entry_exit_add_one(
        f(v[0]), f(v[1]), f(v[2]), f(v[3]), f(v[4]), f(v[5]), f(v[6]), f(v[7]), f(v[8]), f(v[9]), f(v[10]), area, flags, extra, t1, t2, sky, title,
    );
    let pool = GPtr::<GPtr<u8>>::from_addr(0x96A7D8).read();
    let e = if rd8(rdp(pool, 4), idx as isize) & 0x80 == 0 {
        rdp(pool, 0).byte_add(0x3C * idx as isize)
    } else {
        GPtr::NULL
    };
    for bit in 0..8u32 {
        if flags as u32 & (1 << bit) != 0 {
            if bit == 1 || bit == 7 {
                wr8(e, 0x30, rd8(e, 0x30) | (1 << bit) as u8);
            } else {
                wr16(e, 0x30, rd16(e, 0x30) | (1 << bit) as i16);
            }
        }
    }
}

/// Pickup model of IPL pickup type `t` (the jump table at 0x5B49D4 of LoadPickup); `None` = ignored.
pub fn pickup_model(t: i32) -> Option<u32> {
    Some(match t {
        4 => 0x14B,
        5 => 0x14E,
        6 => 0x14F,
        9 => 0x14D,
        10 => 0x150,
        11 => 0x151,
        12 => 0x152,
        13 => 0x153,
        14 => 0x155,
        15 => 0x158,
        16 => 0x156,
        17 => 0x16B,
        18 => 0x15A,
        19 => 0x15B,
        20 => 0x15C,
        21 => 0x15D,
        22 | 45 => 0x15F,
        23 => 0x174,
        24 => 0x160,
        25 => 0x161,
        26 => 0x163,
        27 => 0x164,
        28 => 0x165,
        29 => 0x166,
        31 => 0x169,
        32 | 44 => 0x16A,
        33 => 0x141,
        34 => 0x142,
        35 => 0x143,
        36 => 0x144,
        37 => 0x145,
        38 => 0x146,
        39 => 0x147,
        40 => 0x148,
        41 => 0x14A,
        43 => 0x157,
        46 => 0x167,
        47 => 0x168,
        48 => 0x16C,
        49 => 0x16D,
        50 => 0x16E,
        51 => 0x16F,
        52 => 0x170,
        53 => 0x171,
        54 => 0x172,
        55 => 0x173,
        _ => return None,
    })
}

/// `CFileLoader::LoadPickup` (0x5B47B0): `"%d %f %f %f"`; known types become a pickup of type 2.
#[hook(addr = 0x5B47B0, conv = cdecl, name = "CFileLoader::LoadPickup", expect = "83 EC 1C 8D 44 24 08")]
pub fn load_pickup(line: GPtr<u8>) {
    let mut t = 0i32;
    let (mut x, mut y, mut z) = (0u32, 0u32, 0u32);
    if sscanf_n(line, s(0x868CF8), &[lp(&mut t), lp(&mut x), lp(&mut y), lp(&mut z)]) != 4 {
        return;
    }
    if let Some(m) = pickup_model(t) {
        generate_new_pickup(f(x), f(y), f(z), m, 2, 0, 0, 0, 0);
    }
}

/// `CFileLoader::LinkLods(int32 extra)` (0x5B51E0): resolves the LOD indices of `gpLoadedBuildings`
/// (counting children in +0x34), sets up big buildings (children, or lod-distance * draw distance >
/// 300 under x87 PC=24), shares col models / flags with single-child LODs, then caches and adds every
/// entity to the world. With the col cache not loading, indices past the count are added to it first.
#[hook(addr = 0x5B51E0, conv = cdecl, name = "CFileLoader::LinkLods", expect = "53 56 8B 35 D8 C0 BC 00")]
pub fn link_lods(extra: i32) {
    let count = || GPtr::<i32>::from_addr(0xBCC0D8).read();
    let list = GPtr::<GPtr<u8>>::from_addr(0xBCC0E0);
    let n = count();
    for k in 0..n.max(0) {
        let e = list.wrapping_index(k).read();
        let lod = rd32(e, 0x30) as i32;
        if lod == -1 {
            wr32(e, 0x30, 0);
        } else {
            let l = list.wrapping_index(lod).read();
            wrp(e, 0x30, l);
            wr8(l, 0x34, rd8(l, 0x34).wrapping_add(1));
        }
    }
    if n.wrapping_add(extra) != 0 {
        let mut i: u32 = 0;
        loop {
            let skip = if col_accel_is_cache_loading() {
                i as i32 >= count()
            } else {
                if i as i32 >= count() {
                    col_accel_add_ipl_entity(list.cast(), count(), i);
                }
                false
            };
            if !skip {
                let e = list.wrapping_index(i as i32).read();
                let mi = model_info(rd16(e, 0x22) as i32).cast::<u8>();
                let big = rd8(e, 0x34) > 0 || {
                    let dist = X87::from_f32(f(GPtr::<u32>::from_addr(0xB6F118).read())).fmul_m(f(rd32(mi, 0x18)));
                    dist.to_f64() > 300.0
                };
                if big {
                    setup_big_building(e);
                }
                let lod = rdp(e, 0x30);
                if !lod.is_null() {
                    let lod_mi = model_info(rd16(lod, 0x22) as i32).cast::<u8>();
                    let ent_mi = model_info(rd16(e, 0x22) as i32).cast::<u8>();
                    let children = rd8(lod, 0x34);
                    if children == 1 {
                        if rd32(e, 0x1C) & 0x10_0000 != 0 {
                            wr32(lod, 0x1C, rd32(lod, 0x1C) | 0x10_0000);
                        }
                        let col = rdp(ent_mi, 0x14);
                        if rdp(lod_mi, 0x14) != col && !col.is_null() {
                            delete_collision_model(lod_mi);
                            set_col_model(lod_mi, rdp(ent_mi, 0x14), 0);
                        }
                    } else if rd8(ent_mi, 0x12) & 0x20 != 0 {
                        wr32(ent_mi, 0x18, 0x43C8_0000);
                    } else {
                        wr8(lod, 0x34, children.wrapping_sub(1));
                        wr32(e, 0x30, 0);
                    }
                }
            }
            i = i.wrapping_add(1);
            if i >= (count().wrapping_add(extra)) as u32 {
                break;
            }
        }
    }
    col_accel_cache_ipl_section(list.cast(), count());
    let mut k = 0;
    while k < count() {
        world_add(list.wrapping_index(k).read());
        k += 1;
    }
}

// ------------------------------------------------------------------------- file dispatchers ---

orig! {
    fn load_object_call(line: GPtr<u8>) -> i32 = cdecl 0x5B3C60 => load_object;
    fn load_time_object_call(line: GPtr<u8>) -> i32 = cdecl 0x5B3DE0 => load_time_object;
    fn load_weapon_object_call(line: GPtr<u8>) -> i32 = cdecl 0x5B3FB0 => load_weapon_object;
    fn load_clump_object_call(line: GPtr<u8>) -> i32 = cdecl 0x5B4040 => load_clump_object;
    fn load_animated_clump_object_call(line: GPtr<u8>) -> i32 = cdecl 0x5B40C0 => load_animated_clump_object;
    fn load_vehicle_object_call(line: GPtr<u8>) -> i32 = cdecl 0x5B6F30 => load_vehicle_object;
    fn load_ped_object_call(line: GPtr<u8>) -> i32 = cdecl 0x5B7420 => load_ped_object;
    fn load_path_header_call(line: GPtr<u8>, ty: GPtr<u8>) -> i32 = cdecl 0x5B41C0 => load_path_header;
    /// CFileLoader::LoadPedPathNode (0x5B41F0, deferred: its callee is a `ret 0x44` stub)
    fn load_ped_path_node(line: GPtr<u8>, id: i32, idx: i32) = cdecl 0x5B41F0 => unported;
    /// CFileLoader::LoadCarPathNode (0x5B4380, deferred: as above)
    fn load_car_path_node(line: GPtr<u8>, id: i32, idx: i32, water: u8) = cdecl 0x5B4380 => unported;
    fn load_2d_effect_call(line: GPtr<u8>) = cdecl 0x5B7670 => load_2d_effect;
    fn load_txd_parent_call(line: GPtr<u8>) = cdecl 0x5B75E0 => load_txd_parent;
    fn load_object_instance_line_call(line: GPtr<u8>) -> GPtr<u8> = cdecl 0x538690 => load_object_instance_line;
    fn load_zone_call(line: GPtr<u8>) = cdecl 0x5B4AB0 => load_zone;
    fn load_cull_zone_call(line: GPtr<u8>) = cdecl 0x5B4B40 => load_cull_zone;
    fn load_occlusion_volume_call(line: GPtr<u8>, name: GPtr<u8>) = cdecl 0x5B4C80 => load_occlusion_volume;
    fn load_audio_zone_call(line: GPtr<u8>) = cdecl 0x5B4D70 => load_audio_zone;
    fn load_entry_exit_call(line: GPtr<u8>) = cdecl 0x5B8030 => load_entry_exit;
    fn load_garage_call(line: GPtr<u8>) = cdecl 0x5B4530 => load_garage;
    fn load_pickup_call(line: GPtr<u8>) = cdecl 0x5B47B0 => load_pickup;
    fn load_car_generator_line_call(line: GPtr<u8>, ipl: i32) = cdecl 0x5B4740 => load_car_generator_line;
    fn load_stunt_jump_call(line: GPtr<u8>) = cdecl 0x5B45D0 => load_stunt_jump;
    fn load_time_cycles_modifier_call(line: GPtr<u8>) = cdecl 0x5B81D0 => load_time_cycles_modifier;
    fn link_lods_call(extra: i32) = cdecl 0x5B51E0 => link_lods;
    fn load_collision_file_call(name: GPtr<u8>, col: u8) = cdecl 0x5B4E60 => load_collision_file;
    fn load_atomic_file_call(name: GPtr<u8>) = cdecl 0x5B39D0 => load_atomic_file;
    fn load_clump_file_call(name: GPtr<u8>) = cdecl 0x5B3A30 => load_clump_file;
    fn load_object_types_call(name: GPtr<u8>) = cdecl 0x5B8400 => load_object_types;
    fn load_scene_call(name: GPtr<u8>) = cdecl 0x5B8700 => load_scene;
    fn load_tex_dictionary_call(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x5B3860 => load_tex_dictionary;
    fn loading_screen_loading_file_call(name: GPtr<u8>) = cdecl 0x5B3680 => loading_screen_loading_file;
    /// CIplStore::GetNewIplEntityIndexArray (0x404780)
    fn get_new_ipl_entity_index_array(n: i32) -> i32 = cdecl 0x404780 => unported;
    /// CIplStore::GetIplEntityIndexArray (0x4047B0)
    fn get_ipl_entity_index_array(idx: i32) -> GPtr<u8> = cdecl 0x4047B0 => unported;
    /// CIplStore::SetupRelatedIpls (0x404DE0)
    fn setup_related_ipls(name: GPtr<u8>, idx: i32, out: GPtr<u8>) -> i32 = cdecl 0x404DE0 => unported;
    /// CIplStore::RemoveRelatedIpls (0x405110)
    fn remove_related_ipls(idx: i32) = cdecl 0x405110 => unported;
    /// RwTexDictionaryGetCurrent (0x7F3A90)
    fn rw_tex_dictionary_get_current() -> GPtr<u8> = cdecl 0x7F3A90 => unported;
    /// RwTexDictionarySetCurrent (0x7F3A70)
    fn rw_tex_dictionary_set_current(d: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3A70 => unported;
    /// RwTexDictionaryDestroy (0x7F36A0)
    fn rw_tex_dictionary_destroy(d: GPtr<u8>) -> i32 = cdecl 0x7F36A0 => unported;
    /// CStreaming::AddImageToList (0x407610)
    fn add_image_to_list(name: GPtr<u8>, not_player: u8) -> i32 = cdecl 0x407610 => unported;
    /// ModelIndices::MatchAllModelStrings (0x5B57C0)
    fn match_all_model_strings() = cdecl 0x5B57C0 => unported;
    /// CObjectData::Initialise (0x5B5360)
    fn object_data_initialise(name: GPtr<u8>, b: u8) = cdecl 0x5B5360 => unported;
    /// CVehicleModelInfo::SetupCommonData (0x5B8F00)
    fn setup_common_data() = cdecl 0x5B8F00 => crate::modelinfo::vehicle::setup_common_data;
    /// CStreaming::Init2 (0x5B8AD0)
    fn streaming_init2() = cdecl 0x5B8AD0 => unported;
    /// CLoadingScreen::NewChunkLoaded (0x590D00)
    fn new_chunk_loaded() = cdecl 0x590D00 => unported;
    /// CColStore::LoadAllBoundingBoxes (0x4113D0)
    fn load_all_bounding_boxes() = cdecl 0x4113D0 => unported;
    /// virtual slot 0x34: ConvertAnimFileIndex()
    fn v_convert_anim_file_index(mi: GPtr<u8>) = thiscall vslot 0x34 => unported;
    /// CIplStore::LoadAllRemainingIpls (0x405780)
    fn load_all_remaining_ipls() = cdecl 0x405780 => unported;
    /// CColStore::BoundingBoxesPostProcess (0x410EC0)
    fn bounding_boxes_post_process() = cdecl 0x410EC0 => unported;
    /// CTrain::InitTrains (0x6F7440)
    fn init_trains() = cdecl 0x6F7440 => unported;
    /// CColStore::RemoveAllCollision (0x410E00)
    fn remove_all_collision() = cdecl 0x410E00 => unported;
}

/// The four-character section header test (`line[0..4] == tag`, byte by byte).
fn tag4(line: GPtr<u8>, tag: &[u8; 4]) -> bool {
    (0..4).all(|k| rd8(line, k as isize) == tag[k])
}

/// The path-section step shared by LoadObjectTypes and LoadScene: header, then 12 nodes.
fn path_line(line: GPtr<u8>, idx: &mut i32, ty: &mut i32, id: &mut i32) {
    if *idx == -1 {
        *id = load_path_header_call(line, lp(ty));
        *idx = 0;
        return;
    }
    match *ty {
        0 => load_ped_path_node(line, *id, *idx),
        1 => load_car_path_node(line, *id, *idx, 0),
        2 => load_car_path_node(line, *id, *idx, 1),
        _ => {}
    }
    *idx += 1;
    if *idx == 12 {
        *idx = -1;
    }
}

/// `CFileLoader::LoadObjectTypes(filename)` (0x5B8400): IDE sections objs/tobj/weap/hier/anim/cars/
/// peds/path/2dfx/txdp (four-byte tags), "end" closes; min/max ids are tracked and unused
/// (gr-deviations `FileLoader.cpp:2183/2192/2253/2256`, harmless); the file name is copied to an
/// unused local.
#[hook(addr = 0x5B8400, conv = cdecl, name = "CFileLoader::LoadObjectTypes", expect = "81 EC 10 01 00 00")]
pub fn load_object_types(filename: GPtr<u8>) {
    let f = open_file(filename, s(0x859C6C));
    let mut state = 0;
    let (mut path_idx, mut path_ty, mut path_id) = (-1i32, 0i32, 0i32);
    let mut line = load_line_file(f);
    while !line.is_null() {
        let c = rd8(line, 0);
        if c != 0 && c != b'#' {
            if state == 0 {
                state = if tag4(line, b"objs") {
                    1
                } else if tag4(line, b"tobj") {
                    3
                } else if tag4(line, b"weap") {
                    4
                } else if tag4(line, b"hier") {
                    5
                } else if tag4(line, b"anim") {
                    6
                } else if tag4(line, b"cars") {
                    7
                } else if tag4(line, b"peds") {
                    8
                } else if tag4(line, b"path") {
                    9
                } else if tag4(line, b"2dfx") {
                    10
                } else if tag4(line, b"txdp") {
                    11
                } else {
                    0
                };
            } else if rd8(line, 0) == b'e' && rd8(line, 1) == b'n' && rd8(line, 2) == b'd' {
                state = 0;
            } else {
                match state {
                    1 => {
                        load_object_call(line);
                    }
                    3 => {
                        load_time_object_call(line);
                    }
                    4 => {
                        load_weapon_object_call(line);
                    }
                    5 => {
                        load_clump_object_call(line);
                    }
                    6 => {
                        load_animated_clump_object_call(line);
                    }
                    7 => {
                        load_vehicle_object_call(line);
                    }
                    8 => {
                        load_ped_object_call(line);
                    }
                    9 => path_line(line, &mut path_idx, &mut path_ty, &mut path_id),
                    10 => load_2d_effect_call(line),
                    11 => load_txd_parent_call(line),
                    _ => {}
                }
            }
        }
        line = load_line_file(f);
    }
    close_file(f);
}

/// `CFileLoader::LoadScene(filename)` (0x5B8700): IPL sections inst/mult/zone/cull/path/occl/grge/
/// enex/pick/cars/jump/tcyc/auzo. *new* (keep-original): the "path" header sets state 1, whose only
/// effect is that the next non-"end" line stops reading the file (the path handler, state 7, is
/// unreachable). Instances go to gpLoadedBuildings, then the IPL entity index array, related IPLs,
/// LinkLods and RemoveRelatedIpls.
#[hook(addr = 0x5B8700, conv = cdecl, name = "CFileLoader::LoadScene", expect = "51 8B 44 24 08")]
pub fn load_scene(filename: GPtr<u8>) {
    let count = GPtr::<i32>::from_addr(0xBCC0D8);
    let list = GPtr::<GPtr<u8>>::from_addr(0xBCC0E0);
    let mut state = 0;
    count.write(0);
    let f = open_file(filename, s(0x859C6C));
    let mut line = load_line_file(f);
    while !line.is_null() {
        let c = rd8(line, 0);
        if c != 0 && c != b'#' {
            if state == 0 {
                const TAGS: [(&[u8; 4], i32); 13] = [
                    (b"inst", 2),
                    (b"mult", 3),
                    (b"zone", 4),
                    (b"cull", 5),
                    (b"path", 1),
                    (b"occl", 6),
                    (b"grge", 8),
                    (b"enex", 9),
                    (b"pick", 10),
                    (b"cars", 11),
                    (b"jump", 12),
                    (b"tcyc", 13),
                    (b"auzo", 14),
                ];
                if let Some((_, st)) = TAGS.iter().find(|(t, _)| tag4(line, t)) {
                    state = *st;
                }
            } else if rd8(line, 0) == b'e' && rd8(line, 1) == b'n' && rd8(line, 2) == b'd' {
                state = 0;
            } else {
                match state {
                    2 => {
                        let e = load_object_instance_line_call(line);
                        let n = count.read();
                        list.wrapping_index(n).write(e);
                        count.write(n + 1);
                    }
                    4 => load_zone_call(line),
                    5 => load_cull_zone_call(line),
                    6 => load_occlusion_volume_call(line, filename),
                    8 => load_garage_call(line),
                    9 => load_entry_exit_call(line),
                    10 => load_pickup_call(line),
                    11 => load_car_generator_line_call(line, 0),
                    12 => load_stunt_jump_call(line),
                    13 => load_time_cycles_modifier_call(line),
                    14 => load_audio_zone_call(line),
                    _ => {}
                }
                if state == 1 {
                    break;
                }
            }
        }
        line = load_line_file(f);
    }
    close_file(f);
    let n = count.read();
    let mut idx = -1;
    if n > 0 {
        idx = get_new_ipl_entity_index_array(n);
        let arr = get_ipl_entity_index_array(idx).cast::<GPtr<u8>>();
        let n = count.read();
        for k in 0..n {
            arr.wrapping_index(k).write(list.wrapping_index(k).read());
        }
    }
    let n = count.read();
    let extra = setup_related_ipls(filename, idx, list.wrapping_index(n).cast());
    link_lods_call(extra);
    remove_related_ipls(idx);
}

/// `CFileLoader::LoadLevel(filename)` (0x5B9030). gr-deviations `FileLoader.cpp:1304/1312/1358/1368/
/// 1373` (harmless); keep-original: IMG lines do not show the loading file and skip
/// "MODELS\GTA_INT.IMG" (19-byte compare), COLFILE uses `line + 10`, SPLASH is ignored, TEXDICTION
/// inlines LoadingScreenLoadingFile. The first IPL line runs the one-time setup (model strings,
/// object data, vehicle data, streaming init, bounding boxes, ConvertAnimFileIndex of every model).
#[hook(addr = 0x5B9030, conv = cdecl, name = "CFileLoader::LoadLevel", expect = "83 EC 44 53 55")]
pub fn load_level(filename: GPtr<u8>) {
    let mut prev = rw_tex_dictionary_get_current();
    let mut loaded = false;
    if prev.is_null() {
        prev = rw_tex_dictionary_create();
        rw_tex_dictionary_set_current(prev);
    }
    let f = open_file(filename, s(0x85A53C));
    let mut buf = [0u8; 64 + 256];
    let bp = GPtr::from_raw(buf.as_mut_ptr());
    let mut line = load_line_file(f);
    while !line.is_null() {
        if rd8(line, 0) != b'#' {
            if strncmp(s(0x869B70), line, 4) == 0 {
                break;
            }
            if strncmp(s(0x869B64), line, 10) == 0 {
                let src = line.byte_add(11);
                let mut k = 0;
                loop {
                    let c = rd8(src, k);
                    wr8(bp, k, c);
                    k += 1;
                    if c == 0 {
                        break;
                    }
                }
                let p = strrchr(bp, b'\\' as i32);
                let name = if p.is_null() { bp } else { p.byte_add(1) };
                let g = G_STRING.ptr().cast::<u8>();
                sprintf1(g, s(0x868B40), name);
                loading_screen(s(0x863A6C), g, GPtr::NULL);
                let txd = load_tex_dictionary_call(bp);
                rw_tex_dictionary_for_all_textures(txd, 0x5B38F0, prev);
                rw_tex_dictionary_destroy(txd);
            } else if strncmp(s(0x869B60), line, 3) == 0 {
                if !cmpsb_eq(line.byte_add(4), b"MODELS\\GTA_INT.IMG\0") {
                    add_image_to_list(line.byte_add(4), 1);
                }
            } else if strncmp(s(0x869B58), line, 7) == 0 {
                loading_screen_loading_file_call(line.byte_add(10));
                load_collision_file_call(line.byte_add(10), 0);
            } else if strncmp(s(0x869B4C), line, 9) == 0 {
                loading_screen_loading_file_call(line.byte_add(10));
                load_atomic_file_call(line.byte_add(10));
            } else if strncmp(s(0x869B40), line, 8) == 0 {
                loading_screen_loading_file_call(line.byte_add(9));
                load_clump_file_call(line.byte_add(9));
            } else if strncmp(s(0x869B3C), line, 3) == 0 {
                loading_screen_loading_file_call(line.byte_add(4));
                load_object_types_call(line.byte_add(4));
            } else if strncmp(s(0x869640), line, 3) == 0 {
                if !loaded {
                    match_all_model_strings();
                }
                let src = line.byte_add(4);
                let mut k = 0;
                loop {
                    let c = rd8(src, k);
                    wr8(bp, k, c);
                    k += 1;
                    if c == 0 {
                        break;
                    }
                }
                if !loaded {
                    loading_screen_loading_file_call(s(0x869B30));
                    object_data_initialise(s(0x869B20), 0);
                    loading_screen_loading_file_call(s(0x869B08));
                    setup_common_data();
                    loading_screen_loading_file_call(s(0x869AF8));
                    streaming_init2();
                    new_chunk_loaded();
                    loading_screen_loading_file_call(s(0x863DC0));
                    load_all_bounding_boxes();
                    for id in 0..crate::shared::NUM_MODEL_INFOS as i32 {
                        let mi = model_info(id);
                        if !mi.is_null() {
                            v_convert_anim_file_index(mi.cast());
                        }
                    }
                    loaded = true;
                }
                loading_screen_loading_file_call(bp);
                load_scene_call(bp);
            } else {
                strncmp(s(0x869AF0), line, 6);
            }
        }
        line = load_line_file(f);
    }
    close_file(f);
    rw_tex_dictionary_set_current(prev);
    if loaded {
        load_all_remaining_ipls();
        bounding_boxes_post_process();
        init_trains();
        remove_all_collision();
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LOAD_LINE_FILE,
    HOOK_LOAD_LINE_BUFFER,
    HOOK_LOAD_PATH_HEADER,
    HOOK_LOAD_GARAGE,
    HOOK_LOAD_OCCLUSION_VOLUME,
    HOOK_LOAD_AUDIO_ZONE,
    HOOK_LOAD_TIME_CYCLES_MODIFIER,
    HOOK_ADD_TEX_DICTIONARIES,
    HOOK_LOAD_CAR_GENERATOR_STRUCT,
    HOOK_LOAD_CAR_GENERATOR_LINE,
    HOOK_LOAD_STUNT_JUMP,
    HOOK_LOAD_ZONE,
    HOOK_LOAD_CULL_ZONE,
    HOOK_LOAD_TXD_PARENT,
    HOOK_LOAD_TEX_DICTIONARY,
    HOOK_LOAD_ATOMIC_FILE_STREAM,
    HOOK_LOAD_ATOMIC_FILE,
    HOOK_START_LOAD_CLUMP_FILE,
    HOOK_FINISH_LOAD_CLUMP_FILE,
    HOOK_LOAD_CLUMP_FILE_STREAM,
    HOOK_LOAD_CLUMP_FILE,
    HOOK_SET_RELATED_MODEL_INFO_CB,
    HOOK_FIND_RELATED_MODEL_INFO_CB,
    HOOK_LOADING_SCREEN_LOADING_FILE,
    HOOK_LOAD_OBJECT,
    HOOK_LOAD_TIME_OBJECT,
    HOOK_LOAD_WEAPON_OBJECT,
    HOOK_LOAD_CLUMP_OBJECT,
    HOOK_LOAD_ANIMATED_CLUMP_OBJECT,
    HOOK_LOAD_COLLISION_MODEL,
    HOOK_LOAD_COLLISION_MODEL_VER2,
    HOOK_LOAD_COLLISION_MODEL_VER3,
    HOOK_LOAD_COLLISION_MODEL_VER4,
    HOOK_LOAD_COLLISION_FILE_BUFFER,
    HOOK_LOAD_COLLISION_FILE,
    HOOK_LOAD_COLLISION_FILE_FIRST_TIME,
    HOOK_LOAD_VEHICLE_OBJECT,
    HOOK_LOAD_PED_OBJECT,
    HOOK_LOAD_2D_EFFECT,
    HOOK_LOAD_OBJECT_INSTANCE_LINE,
    HOOK_LOAD_ENTRY_EXIT,
    HOOK_LOAD_PICKUP,
    HOOK_LINK_LODS,
    HOOK_LOAD_OBJECT_TYPES,
    HOOK_LOAD_SCENE,
    HOOK_LOAD_LEVEL,
];
