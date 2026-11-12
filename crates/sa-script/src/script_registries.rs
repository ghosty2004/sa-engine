//! Script registries on the loading path (batch `mvpj-script-1`): the SWITCH jump table, the mission local
//! variable wipe, the connect-lod / special-anim-group / invisibility registrations and two small command
//! helpers. Ported from the 1.0 US bytes (`.HOODLUM` bodies behind the stubs); unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// `CTheScripts::LocalVariablesForCurrentMission` (0xA48960, 0x400 dwords)
const MISSION_LOCALS: usize = 0xA48960;
/// `CTheScripts::SwitchJumpTable` (0xA43CF8, 75 of (int32 value, int32 offset))
const SWITCH_JUMP_TABLE: usize = 0xA43CF8;
/// `CTheScripts::NumberOfEntriesInSwitchTable` (0xA43F50, u16)
const SWITCH_NUM_ENTRIES: usize = 0xA43F50;
/// `CTheScripts::SwitchDefaultAddress` (0xA43F54)
const SWITCH_DEFAULT_ADDRESS: usize = 0xA43F54;
/// `CTheScripts::SwitchDefaultExists` (0xA43F58, u8)
const SWITCH_DEFAULT_EXISTS: usize = 0xA43F58;
/// `CTheScripts::ValueToCheckInSwitchStatement` (0xA43F5C)
const SWITCH_VALUE: usize = 0xA43F5C;
/// `CTheScripts::NumberOfEntriesStillToReadForSwitch` (0xA43F60, u16)
const SWITCH_ENTRIES_TO_READ: usize = 0xA43F60;
/// The connect-lod table (10 pairs of int32 object handles)
const CONNECT_LODS: usize = 0xA44800;
/// The special anim groups attached to char models (8 of 0x14: model int32 +0, group name char[] +4)
const ATTACHED_ANIM_GROUPS: usize = 0xA44850;
/// The invisibility settings (0x14 entity pointers)
const INVISIBILITY_SETTINGS: usize = 0xA449E0;
/// `CPools::ms_pObjectPool` (0xB7449C; +0 objects, +4 flags bytes)
const OBJECT_POOL: usize = 0xB7449C;
/// `sizeof(CObject)`
const OBJECT_SIZE: i32 = 0x19C;

orig! {
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    fn streaming_set_mission_doesnt_require_model(id: i32) = cdecl 0x409C90 => unported;
    /// `CDecisionMakerTypes::AddDecisionMaker(CDecisionMaker*, eDecisionTypes, bool)` (0x607050, `ret 0xC`)
    fn decision_maker_types_add(t: GPtr<u8>, dm: GPtr<u8>, ty: u32, mission: u32) -> u32 = thiscall 0x607050 => unported;
}

fn ga<T: sa_core::mem::GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `CTheScripts::WipeLocalVariableMemoryForMissionScript()` (stub 0x464BB0 → 0x1569F10): the 0x400 mission local
/// variables zeroed (`rep stosd`).
#[hook(addr = 0x464bb0, conv = cdecl, name = "CTheScripts::WipeLocalVariableMemoryForMissionScript", expect = "E9 5B 53 10 01",
       unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn wipe_local_variable_memory_for_mission_script() {
    for i in 0..0x400 {
        ga::<u32>(MISSION_LOCALS).wrapping_index(i).write(0);
    }
}

/// `CTheScripts::AddToSwitchJumpTable(int32 value, int32 offset)` (0x470390): entry
/// `NumberOfEntriesInSwitchTable` (u16) = (value, offset), the count incremented (no bound check).
#[hook(addr = 0x470390, conv = cdecl, name = "CTheScripts::AddToSwitchJumpTable", expect = "66 8B 0D 50 3F A4 00",
       unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn add_to_switch_jump_table(value: i32, offset: i32) {
    let n = ga::<u16>(SWITCH_NUM_ENTRIES).read();
    let e = ga::<i32>(SWITCH_JUMP_TABLE + usize::from(n) * 8);
    e.write(value);
    e.wrapping_index(1).write(offset);
    ga::<u16>(SWITCH_NUM_ENTRIES).write(n.wrapping_add(1));
}

/// `CTheScripts::UseSwitchJumpTable(int32* offset)` (stub 0x4703C0 → 0x1561B80). *offset = 0; a binary search
/// with u8 bounds lo = 0, hi = (u8)(count - 1) for `ValueToCheckInSwitchStatement` while hi - lo > 1 (mid =
/// (hi + lo) / 2; equal → that offset; below → hi = mid; above → lo = mid); then entry hi, entry lo, else
/// `SwitchDefaultAddress` (read through a disguised pointer to 0xA43F54). Then the entries-to-read (u16),
/// value, default address, count (u16) and default-exists (u8) are cleared.
///
/// Stack: the original keeps lo in the low byte of its own argument slot; not reproduced.
#[hook(addr = 0x4703c0, conv = cdecl, name = "CTheScripts::UseSwitchJumpTable", expect = "E9 BB 17 0F 01",
       unverified = "mvpj-script-1", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "writes a byte into its own argument slot")]
pub fn use_switch_jump_table(offset: GPtr<i32>) {
    offset.write(0);
    let mut hi = (ga::<u8>(SWITCH_NUM_ENTRIES).read()).wrapping_sub(1);
    let value = ga::<i32>(SWITCH_VALUE).read();
    let mut lo: u8 = 0;
    let ent = |i: u8| ga::<i32>(SWITCH_JUMP_TABLE + usize::from(i) * 8);
    let r = loop {
        if i32::from(hi) - i32::from(lo) <= 1 {
            break if ent(hi).read() == value {
                ent(hi).wrapping_index(1).read()
            } else if ent(lo).read() == value {
                ent(lo).wrapping_index(1).read()
            } else {
                ga::<i32>(SWITCH_DEFAULT_ADDRESS).read()
            };
        }
        let mid = ((i32::from(hi) + i32::from(lo)) / 2) as u8;
        let v = ent(mid).read();
        if value == v {
            break ent(mid).wrapping_index(1).read();
        }
        if value < v {
            hi = mid;
        } else {
            lo = mid;
        }
    };
    offset.write(r);
    ga::<u16>(SWITCH_ENTRIES_TO_READ).write(0);
    ga::<i32>(SWITCH_VALUE).write(0);
    ga::<i32>(SWITCH_DEFAULT_ADDRESS).write(0);
    ga::<u16>(SWITCH_NUM_ENTRIES).write(0);
    ga::<u8>(SWITCH_DEFAULT_EXISTS).write(0);
}

/// `FUN_004700E0(int32 index)` (cdecl): `CStreaming::SetMissionDoesntRequireModel(index + 0x6676)` (the
/// streamed script's resource id; tail call, the argument slot rewritten).
#[hook(addr = 0x4700e0, conv = cdecl, name = "FUN_004700e0", expect = "8B 44 24 04 05 76 66 00 00",
       route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "tail call with the argument slot rewritten to index + 0x6676")]
pub fn streamed_script_doesnt_require(index: i32) {
    streaming_set_mission_doesnt_require_model(index.wrapping_add(0x6676));
}

/// `FUN_006070F0(int32 index, int32 group, bool mission)` (thiscall on `CDecisionMakerTypes`, `ret 0xC` through
/// the tail call): index -1 → the default ped decision maker (+0xCB50) when `group` is 0, else the default
/// group one (+0xE824); otherwise entry `index` (+4 + index*0x99C); tail call `AddDecisionMaker(that, group,
/// mission)` (EAX passed through).
#[hook(addr = 0x6070f0, conv = thiscall, name = "FUN_006070f0", expect = "8B 44 24 04 83 F8 FF",
       unverified = "mvpj-script-1", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "tail call with the first argument slot rewritten")]
pub fn decision_maker_types_add_by_index(this: GPtr<u8>, index: i32, group: u32, mission: u32) -> u32 {
    let dm = if index == -1 {
        if group == 0 { this.byte_add(0xCB50) } else { this.byte_add(0xE824) }
    } else {
        this.byte_add(index as isize * 0x99C + 4)
    };
    decision_maker_types_add(this, dm, group, mission)
}

/// `CTheScripts::AddToInvisibilitySwapArray(CEntity*, bool remove)` (stub 0x481200 → 0x156DF60). Nothing when
/// the entity's +0x2E byte is set. The entity in the 0x14 slots → cleared when `remove`. Not there and not
/// `remove` → stored in the first empty slot (if any).
#[hook(addr = 0x481200, conv = cdecl, name = "CTheScripts::AddToInvisibilitySwapArray", expect = "E9 5B CD 0E 01",
       unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn add_to_invisibility_swap_array(e: GPtr<u8>, remove: u8) {
    if e.byte_add(0x2E).read() != 0 {
        return;
    }
    let slot = |i: usize| ga::<u32>(INVISIBILITY_SETTINGS).wrapping_index(i as i32);
    let ea = e.addr() as u32;
    if let Some(i) = (0..0x14).find(|&i| slot(i).read() == ea) {
        if remove != 0 {
            slot(i).write(0);
        }
        return;
    }
    if remove != 0 {
        return;
    }
    if let Some(i) = (0..0x14).find(|&i| slot(i).read() == 0) {
        slot(i).write(ea);
    }
}

/// The object pool handle of `o`: (index << 8) + its flags byte, index = (o - objects) / 0x19C.
fn object_handle(o: GPtr<u8>) -> i32 {
    let pool = ga::<GPtr<u8>>(OBJECT_POOL).read();
    let objects = pool.cast::<GPtr<u8>>().read();
    let flags = pool.cast::<GPtr<u8>>().wrapping_index(1).read();
    let idx = (o.addr().wrapping_sub(objects.addr()) as i32) / OBJECT_SIZE;
    (idx << 8) + i32::from(flags.byte_add(idx as isize).read())
}

/// `CTheScripts::AddToListOfConnectedLodObjects(CObject* hi, CObject* lo)` (stub 0x470980 → 0x15661A0): the
/// pool handles of both; the pair already in the 10 slots → nothing; else stored in the LAST free slot (first
/// handle -1) when there is one.
#[hook(addr = 0x470980, conv = cdecl, name = "CTheScripts::AddToListOfConnectedLodObjects", expect = "E9 1B 58 0F 01",
       unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn add_to_list_of_connected_lod_objects(a: GPtr<u8>, b: GPtr<u8>) {
    let h1 = object_handle(a);
    let h2 = object_handle(b);
    let mut free = -1i32;
    for i in 0..10 {
        let e = ga::<i32>(CONNECT_LODS + i as usize * 8);
        let first = e.read();
        if first == h1 && e.wrapping_index(1).read() == h2 {
            return;
        }
        if first == -1 {
            free = i;
        }
    }
    if free != -1 {
        let e = ga::<i32>(CONNECT_LODS + free as usize * 8);
        e.write(h1);
        e.wrapping_index(1).write(h2);
    }
}

/// `CTheScripts::AddToListOfSpecialAnimGroupsAttachedToCharModels(int32 model, const char* group)` (stub
/// 0x474750 → 0x1565D70): an entry of this model with the same name (`strcmp` inlined) → nothing; else the
/// LAST free entry (model -1; entries of this model with another name are not free) gets the model and the
/// name (`strcpy` inlined).
#[hook(addr = 0x474750, conv = cdecl, name = "CTheScripts::AddToListOfSpecialAnimGroupsAttachedToCharModels",
       expect = "E9 1B 16 0F 01", unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn add_to_list_of_special_anim_groups_attached_to_char_models(model: i32, name: GPtr<u8>) {
    let mut free = -1i32;
    for i in 0..8 {
        let e = ga::<u8>(ATTACHED_ANIM_GROUPS + i as usize * 0x14);
        let m = e.cast::<i32>().read();
        if m == model {
            let mut k = 0;
            loop {
                let c = e.byte_add(4 + k).read();
                if c != name.byte_add(k).read() {
                    break;
                }
                if c == 0 {
                    return;
                }
                k += 1;
            }
        } else if m == -1 {
            free = i;
        }
    }
    if free == -1 {
        return;
    }
    let e = ga::<u8>(ATTACHED_ANIM_GROUPS + free as usize * 0x14);
    e.cast::<i32>().write(model);
    let mut k = 0;
    loop {
        let c = name.byte_add(k).read();
        e.byte_add(4 + k).write(c);
        if c == 0 {
            break;
        }
        k += 1;
    }
}

/// `CScripted2dEffects` arrays (0x40 entries each): +0xC39EE0 float, +0xC39FE0 u8, +0xC3A020 int32, +0xC3A120
/// u16, +0xC3A1A0 u8, +0xC3A200 (0x24 bytes), +0xC3BB00 (`C2dEffect`-like, 0x94 bytes)
const S2D_FLOATS: usize = 0xC39EE0;
const S2D_BYTES_A: usize = 0xC39FE0;
const S2D_INTS: usize = 0xC3A020;
const S2D_WORDS: usize = 0xC3A120;
const S2D_BYTES_B: usize = 0xC3A1A0;
const S2D_RECORDS: usize = 0xC3A200;
const S2D_EFFECTS: usize = 0xC3BB00;
/// `CTheScripts::ScriptsForBrains` (0xA90CF0, 0x46 brains of 0x14)
const SCRIPTS_FOR_BRAINS: usize = 0xA90CF0;
/// `CTheScripts::EntitiesWaitingForScriptBrain` (0xA476B0, 150 of (CEntity*, int16 brain))
const ENTITIES_WAITING_FOR_BRAIN: usize = 0xA476B0;
/// `CWorld::PlayerInFocus` (0xB7CD74)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
/// `CWorld::Players` (0xB7CD98, 0x190 each; +0 the ped)
const PLAYERS: usize = 0xB7CD98;
/// `CStreaming::ms_aInfoForModel[0x6676].m_nLoadState` (0x964E08; 0x14 per model)
const SCRIPT_MODEL_LOAD_STATE: usize = 0x964E08;
/// 5.0f (0x859D9C): the default brain activation radius
const K_DEFAULT_BRAIN_RADIUS: usize = 0x859D9C;
/// `CTheScripts::ScriptSpace` (0xA49960)
const SCRIPT_SPACE: usize = 0xA49960;

orig! {
    /// `FUN_006F9EB0` (thiscall): resets one 0x94-byte scripted 2d effect record
    fn scripted_2d_effect_reset(e: GPtr<u8>) = thiscall 0x6F9EB0 => unported;
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250)
    fn find_player_centre_of_world(id: i32) -> GPtr<u8> = cdecl 0x56E250 => unported;
    /// `CScriptsForBrains::StartNewStreamedScriptBrain(uint8, CEntity*, bool)` (0x46B270, `ret 0xC`)
    fn brains_start_new_streamed_script_brain(t: GPtr<u8>, idx: u32, e: GPtr<u8>, b: u32) = thiscall 0x46B270 => unported;
    /// `CScriptsForBrains::StartOrRequestNewStreamedScriptBrain(uint8, CEntity*, int8, bool)` (0x46CD80, `ret 0x10`)
    fn brains_start_or_request(t: GPtr<u8>, idx: u32, e: GPtr<u8>, attach: u32, b: u32) = thiscall 0x46CD80 => unported;
    /// `CScriptsForBrains::IsObjectWithinBrainActivationRange(CObject*, const CVector&)` (0x46B3D0, `ret 8`)
    fn brains_is_object_within_range(t: GPtr<u8>, o: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x46B3D0 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
}

/// `CScripted2dEffects::Init()` (0x6FA6F0): for each of the 0x40 entries: byte B = 0, word = 1, int = -1, the
/// 0x24-byte record's eight dwords = -1 and its +0x20 byte = 0, `FUN_006F9EB0` on the 0x94-byte effect (the
/// original relies on it preserving ECX), byte A = 0, float = -1.0.
#[hook(addr = 0x6fa6f0, conv = cdecl, name = "CScripted2dEffects::Init", expect = "53 55 56 57 33 DB 33 F6",
       unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the callee FUN_006F9EB0 must preserve ECX in the original's loop")]
pub fn scripted_2d_effects_init() {
    for i in 0..0x40usize {
        ga::<u8>(S2D_BYTES_B + i).write(0);
        ga::<u16>(S2D_WORDS + i * 2).write(1);
        ga::<i32>(S2D_INTS + i * 4).write(-1);
        let r = ga::<i32>(S2D_RECORDS + i * 0x24);
        for k in 0..4 {
            r.wrapping_index(k).write(-1);
            r.wrapping_index(k + 4).write(-1);
        }
        ga::<u8>(S2D_RECORDS + i * 0x24 + 0x20).write(0);
        scripted_2d_effect_reset(ga::<u8>(S2D_EFFECTS + i * 0x94));
        ga::<u8>(S2D_BYTES_A + i).write(0);
        ga::<u32>(S2D_FLOATS + i * 4).write(0xBF80_0000);
    }
}

/// `CScriptsForBrains::AddNewScriptBrain(int16 script, int16 model, uint16 priority, int8 attach, int8 type,
/// float radius)` (0x46A930, thiscall, `ret 0x18`): the first of the 0x46 brains (0x14 bytes) with +0 == -1
/// gets +0 script, +0xE priority, +3 type, +0xC model, +2 attach, +4 = 1 and +8 the radius when above 0 (or
/// NaN), else 5.0; none free → nothing.
#[hook(addr = 0x46a930, conv = thiscall, name = "CScriptsForBrains::AddNewScriptBrain", expect = "53 32 DB 32 D2 84 DB",
       unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn scripts_for_brains_add_new_script_brain(this: GPtr<u8>, script: u32, model: u32, priority: u32, attach: u32, ty: u32, radius: f32) {
    let Some(i) = (0..0x46isize).find(|&i| this.byte_add(i * 0x14).cast::<i16>().read() == -1) else {
        return;
    };
    let b = this.byte_add(i * 0x14);
    b.cast::<u16>().write(script as u16);
    b.byte_add(0xE).cast::<u16>().write(priority as u16);
    b.byte_add(3).write(ty as u8);
    b.byte_add(0xC).cast::<u16>().write(model as u16);
    b.byte_add(2).write(attach as u8);
    b.byte_add(4).write(1);
    let use_default = !sa_math::x87::X87::from_f32(radius).fcom(sa_math::x87::X87::zero()).test_parity(0x41);
    let r = if use_default { ga::<f32>(K_DEFAULT_BRAIN_RADIUS).read() } else { radius };
    b.byte_add(8).cast::<f32>().write(r);
}

/// `CTheScripts::ProcessWaitingForScriptBrainArray()` (0x46CF00). With the focus player's ped: C =
/// `FindPlayerCentreOfWorld(focus)`; for each of the 150 waiting entities, by its brain's type (+2, i8): 0/3
/// (peds) → the ped's brain (+0x794): its script's model (0x6676 + script) loaded → `StartNewStreamedScriptBrain(
/// brain, ped, false)`, else `RequestModel(.., 4)` (a dead type-1 test of the original changes nothing); 1/4
/// (objects) → object flags +0x140 bits 20..21: 1 and `IsObjectWithinBrainActivationRange(obj, C)`, or 2 →
/// `StartOrRequestNewStreamedScriptBrain(obj +0x168, obj, type, false)`; 2 and others → nothing.
///
/// The brain index and type are passed as dwords whose upper bytes are stale stack in the original
/// (zero-extended here).
#[hook(addr = 0x46cf00, conv = cdecl, name = "CTheScripts::ProcessWaitingForScriptBrainArray", expect = "0F B6 05 74 CD B7 00",
       unverified = "mvpj-script-1", route = "L2", hazards = "uninit-locals", recs = "grove-walk",
       note = "byte arguments pushed as dwords with stale upper bytes")]
pub fn process_waiting_for_script_brain_array() {
    let focus = ga::<u8>(PLAYER_IN_FOCUS).read();
    if ga::<u32>(PLAYERS + usize::from(focus) * 0x190).read() == 0 {
        return;
    }
    let c = find_player_centre_of_world(i32::from(focus));
    let mut centre = [c.cast::<u32>().read(), c.cast::<u32>().wrapping_index(1).read(), c.cast::<u32>().wrapping_index(2).read()];
    let brains = ga::<u8>(SCRIPTS_FOR_BRAINS);
    for k in 0..150usize {
        let ent = ga::<GPtr<u8>>(ENTITIES_WAITING_FOR_BRAIN + k * 8);
        let e = ent.read();
        if e.is_null() {
            continue;
        }
        let idx = isize::from(ga::<i16>(ENTITIES_WAITING_FOR_BRAIN + k * 8 + 4).read());
        let ty = brains.byte_add(idx * 0x14 + 2).read() as i8;
        match ty {
            0 | 3 => {
                let bi = e.byte_add(0x794).read();
                let script = i32::from(brains.byte_add(isize::from(bi) * 0x14).cast::<i16>().read());
                if ga::<u8>(SCRIPT_MODEL_LOAD_STATE).byte_add(script as isize * 0x14).read() == 1 {
                    brains_start_new_streamed_script_brain(brains, u32::from(bi), e, 0);
                } else {
                    streaming_request_model(script + 0x6676, 4);
                }
            }
            1 | 4 => {
                let t = (e.byte_add(0x140).cast::<u32>().read() >> 20) & 3;
                let go = match t {
                    1 => brains_is_object_within_range(brains, e, GPtr::from_raw(centre.as_mut_ptr()).cast()) != 0,
                    2 => true,
                    _ => false,
                };
                if go {
                    brains_start_or_request(brains, u32::from(e.byte_add(0x168).read()), e, u32::from(ty as u8), 0);
                }
            }
            _ => {}
        }
    }
}

/// `CStreamedScripts::ReadStreamedScriptData()` (stub 0x470750 → 0x156F450, thiscall). The streamed-script
/// section of `ScriptSpace` is found through the header's three chained jump offsets (+3); +0xA40 = its
/// first dword; then for each of the +0xA44 (u16) entries (0x14-byte name, a skipped dword, the size): the
/// first script (0x20 bytes from +0, name at +8) whose name matches (`_stricmp`) — index -1, i.e. `this - 0x20`,
/// when none (original bug, kept) — gets +0x1C = size and +6 = the entry index.
#[hook(addr = 0x470750, conv = thiscall, name = "CStreamedScripts::ReadStreamedScriptData", expect = "E9 FB EC 0F 01",
       unverified = "mvpj-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn streamed_scripts_read_streamed_script_data(this: GPtr<u8>) {
    let hdr = |off: u32| ga::<u8>(SCRIPT_SPACE + 3).byte_add(off as isize).cast::<u32>().read();
    let o1 = hdr(0);
    let o2 = hdr(o1);
    let o3 = hdr(o2);
    let mut p = ga::<u8>(SCRIPT_SPACE + 8).byte_add(o3 as isize);
    this.byte_add(0xA40).cast::<u32>().write(p.cast::<u32>().read());
    p = p.byte_add(8);
    let count = || this.byte_add(0xA44).cast::<u16>().read();
    if count() == 0 {
        return;
    }
    let mut i: u32 = 0;
    loop {
        let mut name = [0u8; 0x14];
        for (k, c) in name.iter_mut().enumerate() {
            *c = p.byte_add(k as isize).read();
        }
        p = p.byte_add(0x14);
        let np: GPtr<u8> = GPtr::from_raw(name.as_mut_ptr());
        let mut j: i32 = -1;
        for s in 0..i32::from(count()) {
            if crt_stricmp(this.byte_add(8 + s as isize * 0x20), np) == 0 {
                j = s;
                break;
            }
        }
        let e = this.byte_add(j as isize * 0x20);
        e.byte_add(0x1C).cast::<u32>().write(p.byte_add(4).cast::<u32>().read());
        p = p.byte_add(8);
        e.byte_add(6).cast::<u16>().write(i as u16);
        i = i.wrapping_add(1);
        if (i as u16) >= count() {
            break;
        }
    }
}

/// `CPools::ms_pPedPool` (0xB74490; 0x7C4 per ped), `ms_pVehiclePool` (0xB74494; 0xA18 per vehicle)
const PED_POOL: usize = 0xB74490;
const VEHICLE_POOL: usize = 0xB74494;
/// -100.0f (0x859014): the "find the ground" z sentinel
const K_GROUND_Z_SENTINEL: usize = 0x859014;

orig! {
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660, ST0)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0, ST0)
    fn entity_distance_to_base(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CEntity::Teleport(CVector, bool)` (vtable +0x38, `ret 0x10`)
    fn v_entity_teleport(e: GPtr<u8>, x: u32, y: u32, z: u32, b: u32) = thiscall vslot 0x38 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u8>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsLeader(const CPed*)` (0x5F69C0, `ret 4`)
    fn membership_is_leader(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CPedGroup::Teleport(const CVector&)` (0x5F7AD0, `ret 4`)
    fn group_teleport(g: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x5F7AD0 => unported;
    /// `CColStore::HasCollisionLoaded(const CVector&, int32 area)` (0x410CE0)
    fn col_store_has_collision_loaded(pos: GPtr<u8>, area: u32) -> u8 = cdecl 0x410CE0 => unported;
    /// `CIplStore::HaveIplsLoaded(const CVector&, int32 area)` (0x405600)
    fn ipl_store_have_ipls_loaded(pos: GPtr<u8>, area: u32) -> u8 = cdecl 0x405600 => unported;
    /// `CEntity::GetIsStatic()` (0x4633E0)
    fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CAutomobile::PlaceOnRoadProperly()` (0x6AF420)
    fn automobile_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6AF420 => unported;
    /// `CBike::PlaceOnRoadProperly()` (0x6BEEB0)
    fn bike_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6BEEB0 => unported;
}

/// `CRunningScript::SetCharCoordinates(CPed*, float x, float y, float z, bool warpGang, bool offset)` (0x464DC0,
/// thiscall, `ret 0x18`; `this` unused). x87. The ped's vehicle (+0x58C when +0x46C bit 8). z not above -100
/// → z = f32 `FindGroundZForCoord(x, y)`. In a vehicle: z = f32(vehicle `GetDistanceFromCentreOfMassToBaseOfModel`
/// + z), the vehicle's `Teleport((x, y, z), false)`, `ClearSpaceForMissionEntity((x, y, z), vehicle)`. On foot: with
/// `offset`, z = f32(ped distance + z); `ClearSpaceForMissionEntity((x, y, z), ped)`; a group leader with `warpGang`
/// → the group's `Teleport`, else the ped's `Teleport((x, y, z), false)`.
///
/// The original keeps z in its own argument slot (popped by `ret 0x18`).
#[hook(addr = 0x464dc0, conv = thiscall, name = "CRunningScript::SetCharCoordinates", expect = "83 EC 0C 56 57 8B 7C 24 18",
       unverified = "mvpj-script-2", route = "L2", hazards = "stack-residue,heap-writes", recs = "grove-walk",
       note = "teleports peds/vehicles (world lists)")]
pub fn running_script_set_char_coordinates(_this: GPtr<u8>, ped: GPtr<u8>, px: f32, py: f32, pz: f32, warp_gang: u8, offset: u8) {
    use sa_math::x87::X87;
    let veh = if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
        ped.byte_add(0x58C).cast::<GPtr<u8>>().read()
    } else {
        GPtr::NULL
    };
    let mut z = pz;
    if !X87::from_f32(z).fcom(X87::from_f32(ga::<f32>(K_GROUND_Z_SENTINEL).read())).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(px.to_bits(), py.to_bits())).to_f32();
    }
    if !veh.is_null() {
        z = X87::from_f64_exact(entity_distance_to_base(veh)).fadd_m(z).to_f32();
        v_entity_teleport(veh, px.to_bits(), py.to_bits(), z.to_bits(), 0);
        let mut p = [px, py, z];
        clear_space_for_mission_entity(GPtr::from_raw(p.as_mut_ptr()).cast(), veh);
        return;
    }
    if offset != 0 {
        z = X87::from_f64_exact(entity_distance_to_base(ped)).fadd_m(z).to_f32();
    }
    let mut p = [px, py, z];
    clear_space_for_mission_entity(GPtr::from_raw(p.as_mut_ptr()).cast(), ped);
    let g = get_peds_group(ped);
    if !g.is_null() && membership_is_leader(g.byte_add(8), ped) != 0 && warp_gang != 0 {
        let mut q = [px, py, z];
        group_teleport(g, GPtr::from_raw(q.as_mut_ptr()).cast());
        return;
    }
    v_entity_teleport(ped, px.to_bits(), py.to_bits(), z.to_bits(), 0);
}

/// An entity's position (matrix +0x30, else the placement +4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The pool slot of `handle` (index = handle >> 8, its flags byte must equal the handle's low byte), else NULL.
fn pool_at(pool: usize, size: isize, handle: i32) -> GPtr<u8> {
    let p = ga::<GPtr<u8>>(pool).read();
    let idx = handle >> 8;
    let flags = p.byte_add(4).cast::<GPtr<u8>>().read();
    if flags.byte_add(idx as isize).read() != handle as u8 {
        return GPtr::NULL;
    }
    p.cast::<GPtr<u8>>().read().byte_add(idx as isize * size)
}

/// `CMissionCleanup::CheckIfCollisionHasLoadedForMissionObjects()` (stub 0x4652D0 → 0x1569D40, thiscall). Over the
/// 0x4B entries (type byte +0, handle +4): vehicles (1), peds (2) and objects (3) that still exist (pool flags),
/// with entity flag +0x1C bit 18 ("waiting for collision") and whose collision (`CColStore::HasCollisionLoaded`)
/// and IPLs (`CIplStore::HaveIplsLoaded`) at their position and area (+0x2F) are loaded lose that flag and join
/// the moving list unless static; vehicles of type 0 or 11 (+0x590) then `CAutomobile::PlaceOnRoadProperly`, type
/// 9 `CBike::PlaceOnRoadProperly`.
#[hook(addr = 0x4652d0, conv = thiscall, name = "CMissionCleanup::CheckIfCollisionHasLoadedForMissionObjects",
       expect = "E9 6B 4A 10 01", unverified = "mvpj-script-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "moving-list and entity flag writes")]
pub fn mission_cleanup_check_if_collision_has_loaded(this: GPtr<u8>) {
    for i in 0..0x4Bisize {
        let e = this.byte_add(i * 8);
        let ty = e.read();
        let h = e.byte_add(4).cast::<i32>().read();
        let ent = match ty {
            1 => pool_at(VEHICLE_POOL, 0xA18, h),
            2 => pool_at(PED_POOL, 0x7C4, h),
            3 => pool_at(OBJECT_POOL, 0x19C, h),
            _ => continue,
        };
        if ent.is_null() {
            continue;
        }
        let fl = ent.byte_add(0x1C).cast::<u32>();
        if fl.read() & 0x40000 == 0 {
            continue;
        }
        if col_store_has_collision_loaded(entity_pos(ent), u32::from(ent.byte_add(0x2F).read())) == 0 {
            continue;
        }
        if ipl_store_have_ipls_loaded(entity_pos(ent), u32::from(ent.byte_add(0x2F).read())) == 0 {
            continue;
        }
        fl.write(fl.read() & 0xFFFB_FFFF);
        if entity_get_is_static(ent) == 0 {
            physical_add_to_moving_list(ent);
        }
        if ty == 1 {
            match ent.byte_add(0x590).cast::<u32>().read() {
                0 | 11 => automobile_place_on_road_properly(ent),
                9 => bike_place_on_road_properly(ent),
                _ => {}
            }
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RUNNING_SCRIPT_SET_CHAR_COORDINATES,
    HOOK_MISSION_CLEANUP_CHECK_IF_COLLISION_HAS_LOADED,
    HOOK_SCRIPTED_2D_EFFECTS_INIT,
    HOOK_SCRIPTS_FOR_BRAINS_ADD_NEW_SCRIPT_BRAIN,
    HOOK_PROCESS_WAITING_FOR_SCRIPT_BRAIN_ARRAY,
    HOOK_STREAMED_SCRIPTS_READ_STREAMED_SCRIPT_DATA,
    HOOK_WIPE_LOCAL_VARIABLE_MEMORY_FOR_MISSION_SCRIPT,
    HOOK_ADD_TO_SWITCH_JUMP_TABLE,
    HOOK_USE_SWITCH_JUMP_TABLE,
    HOOK_STREAMED_SCRIPT_DOESNT_REQUIRE,
    HOOK_DECISION_MAKER_TYPES_ADD_BY_INDEX,
    HOOK_ADD_TO_INVISIBILITY_SWAP_ARRAY,
    HOOK_ADD_TO_LIST_OF_CONNECTED_LOD_OBJECTS,
    HOOK_ADD_TO_LIST_OF_SPECIAL_ANIM_GROUPS_ATTACHED_TO_CHAR_MODELS,
];
