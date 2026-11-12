//! WRITER-MVP-C batch `mvpc-world-2`: entity effects and object pre-render on the boot/loading path
//! (`CEntity::DestroyEffects`, `CObject::PreRender`, `CEntryExitManager::Init`). Ported from the 1.0 US
//! bytes (SecuROM fragments inlined; gta-reversed a reading aid only). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr};
use crate::collision::fpu::Fpu;
use crate::collision::mvpc_coll1::local;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CScriptsForBrains` instance `ScriptsForBrains` (0xA90CF0)
const SCRIPTS_FOR_BRAINS: u32 = 0xA90CF0;
/// `g_fx` (`Fx_c`, 0xA9AE00)
const G_FX: u32 = 0xA9AE00;
/// `CEntryExitManager::mp_poolEntryExits`
const ENTRY_EXIT_POOL: usize = 0x96A7D8;

orig! {
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70, `ret 4`)
    fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `CScriptsForBrains::MarkAttractorScriptBrainWithThisNameAsNoLongerNeeded(const char*)` (0x46AAE0, `ret 4`)
    fn mark_attractor_brain_not_needed(b: u32, name: GPtr<u8>) = thiscall 0x46AAE0 => unported;
    /// `Fx_c::DestroyEntityFx(CEntity*)` (0x4A1280, `ret 4`)
    fn fx_destroy_entity_fx(fx: u32, e: GPtr<u8>) = thiscall 0x4A1280 => unported;
    /// `RpAtomicSetFrame(RpAtomic*, RwFrame*)` (0x74BF20)
    fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `RwFrameDestroy(RwFrame*)` (0x7F05A0)
    fn rw_frame_destroy(f: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
    /// `RpAtomicDestroy(RpAtomic*)` (0x749DC0)
    fn rp_atomic_destroy(a: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    /// `CEntity::TransformFromObjectSpace(CVector& out, const CVector& offset)` (0x5334F0, `ret 8`)
    fn transform_from_object_space(e: GPtr<u8>, out: GPtr<u8>, off: GPtr<u8>) -> GPtr<u8> = thiscall 0x5334F0 => unported;
    /// `CEntryExitManager::FindNearestEntryExit(const CVector2D&, float range, int32 ignoreIndex)` (0x43F4B0)
    fn find_nearest_entry_exit(pos: GPtr<u8>, range: u32, ignore: i32) -> i32 = cdecl 0x43F4B0 => unported;
    /// `CEntryExitManager::DeleteOne(int32)` (0x43FD50)
    fn entry_exit_delete_one(i: i32) = cdecl 0x43FD50 => unported;
    /// `CObject::GetLightingFromCollisionBelow()` (0x59FD00)
    fn get_lighting_from_collision_below(o: GPtr<u8>) = thiscall 0x59FD00 => unported;
    /// `CObject::DoBurnEffect()` (0x59FB50)
    fn do_burn_effect(o: GPtr<u8>) = thiscall 0x59FB50 => unported;
    /// `CVisibilityPlugins::GetClumpAlpha(RpClump*)` (0x732B20)
    fn get_clump_alpha(c: GPtr<u8>) -> i32 = cdecl 0x732B20 => unported;
    /// `CVisibilityPlugins::SetClumpAlpha(RpClump*, int32)` (0x732B00)
    fn set_clump_alpha(c: GPtr<u8>, a: i32) = cdecl 0x732B00 => unported;
    /// `CEntity::PreRender()` (0x535FA0)
    fn entity_pre_render(e: GPtr<u8>) = thiscall 0x535FA0 => crate::world::entity_prerender::pre_render;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `RwMatrixScale(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F22C0)
    fn rw_matrix_scale(m: GPtr<u8>, v: GPtr<u8>, op: u32) -> GPtr<u8> = cdecl 0x7F22C0 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    fn entity_update_rp_hanim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CQuadTreeNode::operator new(uint32)` (0x552C80)
    fn quad_tree_new(size: u32) -> GPtr<u8> = cdecl 0x552C80 => unported;
    /// `CQuadTreeNode::CQuadTreeNode(const CRect&, int32 level)` (0x552830, `ret 8`)
    fn quad_tree_ctor(n: GPtr<u8>, rect: GPtr<u8>, level: i32) -> GPtr<u8> = thiscall 0x552830 => unported;
    /// CRT `operator new` (0x82119A)
    fn crt_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CPool<CEntryExit>::CPool(int32 size, const char* name)` (0x43F720, a `.HOODLUM` stub, `ret 8`)
    fn entry_exit_pool_ctor(p: GPtr<u8>, size: i32, name: u32) -> GPtr<u8> = thiscall 0x43F720 => unported;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    fn file_open(path: u32, mode: u32) -> GPtr<u8> = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    fn file_close(f: GPtr<u8>) -> u32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `sscanf` (0x8220AD), one target
    fn sscanf1(s: GPtr<u8>, fmt: u32, a: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sscanf` (0x8220AD), two targets
    fn sscanf2(s: GPtr<u8>, fmt: u32, a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sscanf` (0x8220AD), six targets
    #[allow(clippy::too_many_arguments)]
    fn sscanf6(s: GPtr<u8>, fmt: u32, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, e: GPtr<u8>, f: GPtr<u8>) -> i32
        = cdecl 0x8220AD => unported;
    /// CRT `sscanf` (0x8220AD), nine targets
    #[allow(clippy::too_many_arguments)]
    fn sscanf9(s: GPtr<u8>, fmt: u32, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, e: GPtr<u8>, f: GPtr<u8>,
        g: GPtr<u8>, h: GPtr<u8>, i: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sprintf` (0x821BB5)
    fn sprintf1(buf: GPtr<u8>, fmt: u32, a: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// `FurnitureManager_c::GetGroupId(const char*)` (0x5BFB40, `ret 4`)
    fn furniture_get_group_id(m: GPtr<u8>, name: GPtr<u8>) -> i32 = thiscall 0x5BFB40 => unported;
    /// `FurnitureManager_c::GetSubGroupId(const char*)` (0x5BFBF0, `ret 4`)
    fn furniture_get_sub_group_id(m: GPtr<u8>, name: GPtr<u8>) -> i32 = thiscall 0x5BFBF0 => unported;
    /// `FurnitureGroup_c::AddSubGroup(int32 id, ...7 ints)` (0x5910E0, `ret 0x20`)
    #[allow(clippy::too_many_arguments)]
    fn furniture_group_add_sub_group(g: GPtr<u8>, id: i32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, h: u32) -> u8
        = thiscall 0x5910E0 => unported;
    /// `FurnitureGroup_c::AddFurniture(int32 subGroupId, int16 modelId, ...4 ints)` (0x5C0230, `ret 0x18`)
    #[allow(clippy::too_many_arguments)]
    fn furniture_group_add_furniture(g: GPtr<u8>, sub: u32, model: u32, a: u32, b: u32, c: u32, d: u32) -> u8
        = thiscall 0x5C0230 => unported;
    /// `CModelInfo::GetModelInfo(const char* name, int32* index)` (0x4C5940)
    fn model_info_by_name(name: GPtr<u8>, idx: GPtr<u8>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
}

/// `FurnitureManager_c::LoadFurniture()` (0x5C0280, thiscall): reads `data\furnitur.dat` line by line
/// (`CFileLoader::LoadLine`), skipping empty and `#` lines; the first word (`%s`) decides, compared over
/// the whole literal including its terminator (`repe cmpsb`): `GROUP:` → `"%s %s"`, the group id
/// (`GetGroupId(name)`, kept for the following lines; -1 before any group); `SUBGROUP:` →
/// `"%s %s %d %d %d %d %d %d %d"`, `GetSubGroupId(name)` (kept), `groups[group].AddSubGroup(id, the seven
/// numbers)`; `ITEM:` → `"%s %s %d %d %d %d"`, `CModelInfo::GetModelInfo(model name, &index)`; found →
/// `groups[group].AddFurniture(subgroup, index, the four numbers)`, else a `"Cannot find model %s"`
/// message is formatted into a local buffer and dropped. Closes the file; EAX = `CloseFile`'s result.
/// The locals live in one frame-shaped buffer so the original's overlapping string slots overlap alike.
#[hook(addr = 0x5C0280, conv = thiscall, name = "FurnitureManager_c::LoadFurniture", this = "FurnitureManager_c",
       expect = "81 EC 9C 01 00 00", route = "split", hazards = "heap-writes,uninit-locals",
       recs = "grove-walk", note = "file read (data/furnitur.dat); fields sscanf does not match keep the previous line's values")]
pub fn furniture_load_furniture(this: GPtr<u8>) -> u32 {
    let mut frame = [0u32; 0x1A0 / 4];
    let base = local(&mut frame);
    // address S - k of the original frame
    let at = |k: isize| base.byte_add(0x1A0 - k);
    let f = file_open(0x86AF80, 0x859C6C);
    let mut group: i32 = -1;
    let mut sub: u32 = 0xFFFF_FFFF;
    let mut line = load_line(f);
    while !line.is_null() {
        let c = rd::<u8>(line, 0);
        if c != b'#' && c != 0 {
            let tok = at(0x160);
            sscanf1(line, 0x85C714, tok);
            let is = |lit: usize, n: usize| (0..n).all(|i| rd::<u8>(tok, i as isize) == g::<u8>(lit + i).read());
            if is(0x86AF78, 7) {
                sscanf2(line, 0x869860, tok, at(0x100));
                group = furniture_get_group_id(this, at(0x100));
            } else if is(0x86AF6C, 10) {
                sscanf9(line, 0x86AF50, tok, at(0xC0), at(0x16C), at(0x17C), at(0x170), at(0x184), at(0x168), at(0x18C),
                    at(0x174));
                let id = furniture_get_sub_group_id(this, at(0xC0));
                sub = id as u32;
                let v = |k| rd::<u32>(at(k), 0);
                furniture_group_add_sub_group(this.byte_add(group as isize * 12), id, v(0x16C), v(0x17C), v(0x170), v(0x184),
                    v(0x168), v(0x18C), v(0x174));
            } else if is(0x86AF48, 6) {
                sscanf6(line, 0x86AF34, tok, at(0x140), at(0x188), at(0x190), at(0x164), at(0x194));
                let mi = model_info_by_name(at(0x140), at(0x180));
                if !mi.is_null() {
                    let v = |k| rd::<u32>(at(k), 0);
                    furniture_group_add_furniture(this.byte_add(group as isize * 12), sub, v(0x180), v(0x188), v(0x190),
                        v(0x164), v(0x194));
                } else {
                    sprintf1(at(0x80), 0x86AF1C, at(0x140));
                }
            }
        }
        line = load_line(f);
    }
    file_close(f)
}

/// `CEntity::DestroyEffects()` (0x533BF0, thiscall): for every 2d effect of the model info (count +0xD,
/// read once): type 3 (ped attractor) with script type 5 → `MarkAttractorScriptBrainWithThisName...
/// NoLongerNeeded(effect + 0x38)`; type 1 (particle) → `g_fx.DestroyEntityFx(this)`; type 7 (roadsign)
/// with an atomic (+0x2C) → detach and destroy its frame (`RpAtomicSetFrame(a, 0)`, `RwFrameDestroy`)
/// when it has one, `RpAtomicDestroy`, +0x2C = 0; type 6 (enex) → the effect position in world space,
/// `FindNearestEntryExit((x, y), 1.5, -1)`; found → its flags (+0x30 of the pool slot; a free slot gives
/// a null object, read as such) with bit 0x2000 get 0x8000 set, else `DeleteOne`.
#[hook(addr = 0x533BF0, conv = thiscall, name = "CEntity::DestroyEffects", this = "CEntity", expect = "83 EC 20 53 8B D9 0F BF",
       route = "L2", hazards = "none", recs = "grove-walk,driving")]
pub fn destroy_effects(this: GPtr<u8>) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(this, 0x22) as isize as usize).read();
    let n = rd::<u8>(mi, 0xD) as i32;
    let mut i = 0;
    while i < n {
        let fx = get_2d_effect(mi, i);
        match rd::<u8>(fx, 0xC) {
            3 => {
                if rd::<u8>(fx, 0x34) == 5 {
                    mark_attractor_brain_not_needed(SCRIPTS_FOR_BRAINS, fx.byte_add(0x38));
                }
            }
            1 => fx_destroy_entity_fx(G_FX, this),
            7 => {
                let a = rd::<GPtr<u8>>(fx, 0x2C);
                if !a.is_null() {
                    let frame = rd::<GPtr<u8>>(a, 4);
                    if !frame.is_null() {
                        rp_atomic_set_frame(a, GPtr::NULL);
                        rw_frame_destroy(frame);
                    }
                    rp_atomic_destroy(a);
                    wr::<u32>(fx, 0x2C, 0);
                }
            }
            6 => {
                let mut wpos = [0u32; 3];
                transform_from_object_space(this, local(&mut wpos), fx);
                let mut p2 = [wpos[0], wpos[1]];
                let idx = find_nearest_entry_exit(local(&mut p2), 0x3FC0_0000, -1);
                if idx != -1 {
                    let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
                    let flags = rd::<GPtr<u8>>(pool, 4);
                    let e = if rd::<u8>(flags, idx as isize) & 0x80 != 0 {
                        GPtr::<u8>::NULL
                    } else {
                        rd::<GPtr<u8>>(pool, 0).byte_add(idx as isize * 0x3C)
                    };
                    let f = rd::<u16>(e, 0x30);
                    if f & 0x2000 != 0 {
                        wr::<u16>(e, 0x30, f | 0x8000);
                    } else {
                        entry_exit_delete_one(idx);
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// `CObject::PreRender()` (0x59FD50, thiscall). Object flags +0x140: 0x800000 →
/// `GetLightingFromCollisionBelow`; burn time +0x174 above `CTimer::m_snTimeInMilliseconds` (unsigned)
/// → `DoBurnEffect`. Not attached (+0xFC null): `m_fContactSurfaceBrightness` (+0x12C) =
/// `(lo * k) * (1 - dn) + (hi * k) * dn` (x87; lo/hi = the nibbles of +0x149, k = 0x858F10,
/// dn = `CCustomBuildingDNPipeline::m_fDNBalanceParam` 0x8D12C0). A clump fading (0x400000) loses 16 of
/// its alpha (floored at 0). `CEntity::PreRender`. When the scale (+0x15C) is not exactly 1.0f (bit
/// compare) or 0x20000 is set: the frame (read through the RW object before the null test) gets the
/// entity's matrix, `RwMatrixScale(.., (s, s, s), 1)`, `RwFrameUpdateObjects`, and 0x20000 is set and
/// then cleared again (the original's int-to-float compare of the bit against 1.0 always passes).
/// A clump → `UpdateRpHAnim`.
#[hook(addr = 0x59FD50, conv = thiscall, name = "CObject::PreRender", this = "CObject", expect = "83 EC 10 56 8B F1 F7 86",
       unverified = "mvpc-world-2", route = "L2", hazards = "none", recs = "grove-walk,driving")]
pub fn object_pre_render(this: GPtr<u8>) {
    if rd::<u32>(this, 0x140) & 0x800000 != 0 {
        get_lighting_from_collision_below(this);
    }
    if rd::<u32>(this, 0x174) > g::<u32>(0xB7CB84).read() {
        do_burn_effect(this);
    }
    if rd::<u32>(this, 0xFC) == 0 {
        let b = rd::<u8>(this, 0x149);
        let k = g::<f32>(0x858F10).read();
        let mut f = Fpu::new();
        f.fld(g::<f32>(0x8D12C0).read());
        f.fild((b & 0xF) as i32);
        f.fmul_m(k);
        f.fld(g::<f32>(0x858624).read());
        f.fsub_st(2);
        f.fmulp(1);
        f.fild((b >> 4) as i32);
        f.fmul_m(k);
        f.fmul_st(2);
        f.faddp(1);
        wr::<f32>(this, 0x12C, f.fstp());
    }
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() && rd::<u8>(rw, 0) == 2 && rd::<u32>(this, 0x140) & 0x400000 != 0 {
        let a = get_clump_alpha(rw) - 16;
        set_clump_alpha(rd::<GPtr<u8>>(this, 0x18), if a < 0 { 0 } else { a });
    }
    entity_pre_render(this);
    if rd::<u32>(this, 0x15C) != 0x3F80_0000 || rd::<u32>(this, 0x140) & 0x20000 != 0 {
        let s = rd::<u32>(this, 0x15C);
        let mut sv = [s, s, s];
        let rw = rd::<GPtr<u8>>(this, 0x18);
        let frame = rd::<GPtr<u8>>(rw, 4);
        let m = frame.byte_add(0x10);
        if !rw.is_null() {
            let cm = rd::<GPtr<u8>>(this, 0x14);
            if !cm.is_null() {
                matrix_update_rw_matrix(cm, m);
            } else {
                simple_transform_update_rw_matrix(this.byte_add(4), m);
            }
        }
        rw_matrix_scale(m, local(&mut sv), 1);
        rw_frame_update_objects(frame);
        let fl = rd::<u32>(this, 0x140) | 0x20000;
        wr::<u32>(this, 0x140, fl);
        wr::<u32>(this, 0x140, fl & 0xFFFD_FFFF);
    }
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() && rd::<u8>(rw, 0) == 2 {
        entity_update_rp_hanim(this);
    }
}

/// `CEntryExitManager::Init()` (0x43F880, a `.HOODLUM` stub into 0x156A6F0; SEH frame): the quad tree
/// root (`CQuadTreeNode::operator new(0x28)`, constructed over the rect (-3000, 3000, 3000, -3000) with
/// 4 levels; null when the allocation fails) into `mp_QuadTree` (0x96A7D0); 0x96A7CC, byte 0x96A7C8,
/// 0x96A7C4 and byte 0x96A730 cleared; the entry-exit pool (CRT `new(0x14)`, `CPool(0x190, name
/// 0x8595F4)`, null on failure) into 0x96A7D8 and its byte +0x11 set.
#[hook(addr = 0x43F880, conv = cdecl, name = "CEntryExitManager::Init", expect = "E9 6B AE 12 01",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates the quad tree and the entry-exit pool (one-time init)")]
pub fn entry_exit_manager_init() {
    let mut rect = [0xC53B_8000u32, 0x453B_8000, 0x453B_8000, 0xC53B_8000];
    let n = quad_tree_new(0x28);
    let root = if n.is_null() { GPtr::NULL } else { quad_tree_ctor(n, local(&mut rect), 4) };
    g::<GPtr<u8>>(0x96A7D0).write(root);
    g::<u32>(0x96A7CC).write(0);
    g::<u8>(0x96A7C8).write(0);
    g::<u32>(0x96A7C4).write(0);
    g::<u8>(0x96A730).write(0);
    let p = crt_new(0x14);
    let pool = if p.is_null() { GPtr::NULL } else { entry_exit_pool_ctor(p, 0x190, 0x8595F4) };
    g::<GPtr<u8>>(0x96A7D8).write(pool);
    wr::<u8>(pool, 0x11, 1);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DESTROY_EFFECTS,
    HOOK_OBJECT_PRE_RENDER,
    HOOK_ENTRY_EXIT_MANAGER_INIT,
    HOOK_FURNITURE_LOAD_FURNITURE,
];
