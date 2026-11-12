//! `CTheScripts::Init` and the SCM segment readers it calls (STD-6, batch `std6-script-1`). Ported from
//! the 1.0 US bytes (0x468D50, the `.HOODLUM` bodies 0x156F5A0 / 0x156F450 / 0x1562910 / 0x1562530
//! behind the stubs 0x486720 / 0x470750 / 0x470960 / 0x474730, and 0x6FA6F0); gta-reversed
//! `TheScripts.cpp` and `sa-formats/src/scm` (segment layout) as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};

orig! {
    /// `CRunningScript::Init()` (0x4648E0)
    fn running_script_init(s: GPtr<u8>) = thiscall 0x4648E0 => unported;
    /// `CStuckCarCheck::Init()` (0x4639E0), `CScriptsForBrains::Init()` (0x46A8C0),
    /// `CScriptResourceManager::Initialise()` (0x470480)
    fn stuck_car_check_init(this: GPtr<u8>) = thiscall 0x4639E0 => unported;
    fn scripts_for_brains_init(this: GPtr<u8>) = thiscall 0x46A8C0 => unported;
    fn script_resource_manager_initialise(this: GPtr<u8>) = thiscall 0x470480 => unported;
    /// `CMenuManager::CheckMissionPackValidMenu()` (0x57D720)
    fn check_mission_pack_valid_menu(menu: GPtr<u8>) -> u8 = thiscall 0x57D720 => unported;
    /// `CFileMgr::SetDirMyDocuments()` (0x538860), `SetDir(dir)` (0x5387D0), `OpenFile` (0x538900),
    /// `Read(file, buf, size)` (0x538950), `CloseFile` (0x5389D0)
    fn set_dir_my_documents() = cdecl 0x538860 => unported;
    fn set_dir(dir: GPtr<u8>) = cdecl 0x5387D0 => unported;
    fn open_file(name: GPtr<u8>, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    fn file_read(f: i32, buf: GPtr<u8>, size: u32) -> i32 = cdecl 0x538950 => unported;
    fn close_file(f: i32) -> i32 = cdecl 0x5389D0 => unported;
    /// CRT `sprintf(buf, fmt, n)` (0x821BB5)
    fn crt_sprintf(buf: GPtr<u8>, fmt: GPtr<u8>, n: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CText::Load(keepMissionPack)` (0x6A01A0)
    fn text_load(this: GPtr<u8>, b: u32) = thiscall 0x6A01A0 => unported;
    /// `CTheScripts::ReadObjectNamesFromScript()` (0x486720), `UpdateObjectIndices()` (0x486780),
    /// `ReadMultiScriptFileOffsetsFromScript()` (0x4867C0)
    fn read_object_names_call() = cdecl 0x486720 => read_object_names_from_script;
    fn update_object_indices() = cdecl 0x486780 => unported;
    fn read_multi_script_file_offsets() = cdecl 0x4867C0 => unported;
    /// `CStreamedScripts::ReadStreamedScriptData()` (0x470750)
    fn read_streamed_script_data_call(this: GPtr<u8>) = thiscall 0x470750 => unported;
    /// `CSprite2d::Delete()` (0x727240)
    fn sprite_delete(this: GPtr<u8>) = thiscall 0x727240 => unported;
    /// `CTxdStore::FindTxdSlot(name)` (0x731850), `RemoveTxd(slot)` (0x731E90)
    fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => unported;
    fn remove_txd(slot: i32) = cdecl 0x731E90 => unported;
    /// `CTheScripts::ClearAllSuppressedCarModels()` (0x46A7C0), `ClearAllVehicleModelsBlockedByScript()`
    /// (0x46A840), `InitialiseAllConnectLodObjects()` (0x470960),
    /// `InitialiseSpecialAnimGroupsAttachedToCharModels()` (0x474730)
    fn clear_all_suppressed_car_models() = cdecl 0x46A7C0 => unported;
    fn clear_all_vehicle_models_blocked_by_script() = cdecl 0x46A840 => unported;
    fn initialise_all_connect_lod_objects_call() = cdecl 0x470960 => unported;
    fn initialise_special_anim_groups_call() = cdecl 0x474730 => unported;
    /// `CScripted2dEffects::Init()` (0x6FA6F0), `CTaskSequences::Init()` (0x632D90), `CPedGroups::Init()`
    /// (0x5FB8C0), `CInformFriendsEventQueue::Init()` (0x4B2990), `CInformGroupEventQueue::Init()`
    /// (0x4B2AD0), `CDecisionMakerTypes::GetInstance()` (0x4684F0)
    fn scripted_2d_effects_init_call() = cdecl 0x6FA6F0 => unported;
    fn task_sequences_init() = cdecl 0x632D90 => unported;
    fn ped_groups_init() = cdecl 0x5FB8C0 => unported;
    fn inform_friends_event_queue_init() = cdecl 0x4B2990 => unported;
    fn inform_group_event_queue_init() = cdecl 0x4B2AD0 => unported;
    fn decision_maker_types_get_instance() -> u32 = cdecl 0x4684F0 => unported;
    /// `C2dEffect` default initialiser (0x6F9EB0, thiscall)
    fn effect_2d_init(this: GPtr<u8>) = thiscall 0x6F9EB0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
}

/// `CTheScripts::ScriptSpace` and `CGame::bMissionPackGame`.
const SCRIPT_SPACE: usize = 0xA49960;
const MISSION_PACK: usize = 0xB72910;
/// `RsGlobal.maximumWidth`
const MAXIMUM_WIDTH: usize = 0xC17044;

fn g<T: GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

fn w8(a: usize, v: u8) {
    g::<u8>(a).write(v)
}

fn w16(a: usize, v: u16) {
    g::<u16>(a).write(v)
}

fn w32(a: usize, v: u32) {
    g::<u32>(a).write(v)
}

fn zero_dwords(a: usize, n: usize) {
    for i in 0..n {
        w32(a + i * 4, 0);
    }
}

fn rgba(a: usize, c: [u8; 4]) {
    for (k, b) in c.into_iter().enumerate() {
        w8(a + k, b);
    }
}

/// `CTheScripts::Init()` (0x468D50; the protection chunk 0x401CF8 supplies the `ScriptSpace` dword
/// count 0x106B2): clears the script space and the mission locals, rebuilds the idle list of the 96
/// running scripts (each `CRunningScript::Init`ed, linked in front), clears the mission cleanup list,
/// the upside-down / stuck car checks, the brains, the resource manager and the entities waiting for a
/// brain; loads "main.scm" from "data\\script" (or, for a mission pack, MPACK//MPACK%d//SCR.SCM from the
/// documents folder while the menu reports a valid pack, then `CText::Load(false)`), 0x30D40 bytes;
/// resets the store / mission globals, the used-object table (then reads the object names and updates
/// their indices), the multi-script table and counters (then the mission offsets; the streamed script
/// sizes unless a mission pack), the script flags, spheres, intro texts (0.48 x 1.12 letters, white,
/// wrap = centre size = the screen width, ...), intro rectangles, the 128 script sprites, the "script"
/// TXD, building swaps, invisibility settings, suppressed / blocked models, LOD connections, special
/// anim groups, effect systems, search lights, checkpoints and sequences; then the 2D effects, task
/// sequences, ped groups, the inform-event queues and `CDecisionMakerTypes::GetInstance()` (tail).
#[hook(addr = 0x468D50, conv = cdecl, name = "CTheScripts::Init", expect = "83 EC 18 53 56 57 33 C0 E9 9B 8F F9 FF",
       unverified = "std6-script-1", route = "L2", hazards = "heap-writes,entry-protection-read", recs = "new-game")]
pub fn the_scripts_init() {
    zero_dwords(SCRIPT_SPACE, 0x106B2);
    zero_dwords(0xA48960, 0x400);
    w32(0xA8B42C, 0);
    let mut prev = 0u32;
    let mut s = 0xA8B430usize;
    while s < 0xA90830 {
        running_script_init(g(s));
        w32(s, prev);
        w32(s + 4, 0);
        if prev != 0 {
            w32(prev as usize + 4, s as u32);
        }
        prev = s as u32;
        s += 0xE0;
    }
    w32(0xA8B428, prev);
    w8(0xA90AA8, 0);
    for i in 0..0x4B {
        w8(0xA90850 + i * 8, 0);
        w32(0xA90854 + i * 8, 0);
    }
    for i in 0..6 {
        w32(0xA4892C + i * 8, 0xFFFF_FFFF);
        w32(0xA48930 + i * 8, 0);
    }
    stuck_car_check_init(g(0xA90AB0));
    scripts_for_brains_init(g(0xA90CF0));
    script_resource_manager_initialise(g(0xA485A8));
    let mut e = 0xA476B4usize;
    while e < 0xA47B64 {
        w32(e - 4, 0);
        w16(e, 0xFFFF);
        e += 8;
    }
    let empty = g::<u8>(0x858B54);
    if g::<u8>(MISSION_PACK).read() != 0 {
        loop {
            if check_mission_pack_valid_menu(g(0xBA6748)) == 0 {
                break;
            }
            set_dir_my_documents();
            crt_sprintf(g(0xB71670), g(0x859D78), g::<u8>(MISSION_PACK).read() as u32);
            let f = open_file(g(0xB71670), g(0x859C6C));
            let mut n = 0;
            if f > 0 {
                n = file_read(f, g(SCRIPT_SPACE), 0x30D40);
                close_file(f);
            }
            if n >= 1 {
                text_load(g(0xC1B340), 0);
                break;
            }
        }
        set_dir(empty);
    }
    if g::<u8>(MISSION_PACK).read() == 0 {
        set_dir(g(0x859D6C));
        let f = open_file(g(0x859D60), g(0x859C6C));
        file_read(f, g(SCRIPT_SPACE), 0x30D40);
        close_file(f);
    }
    set_dir(empty);
    w32(0xA447EC, 0xFFFF_FFFF);
    w8(0xA447F0, 1);
    w32(0xA476AC, 0);
    w32(0xA476A8, 0xFFFF_FFFF);
    w32(0xA476A4, 0xFFFF_FFFF);
    let mut o = 0xA44B88usize;
    while o < 0xA476BC {
        zero_dwords(o - 0x18, 6);
        w32(o, 0);
        o += 0x1C;
    }
    w16(0xA44B6C, 0);
    read_object_names_call();
    update_object_indices();
    w8(0xA444B1, 0);
    w8(0xA447E8, 1);
    zero_dwords(0xA444C8, 0xC8);
    w32(0xA444C4, 0);
    w32(0xA444C0, 0);
    w16(0xA444BC, 0);
    w16(0xA444B8, 0);
    w32(0xA444B4, 0);
    read_multi_script_file_offsets();
    if g::<u8>(MISSION_PACK).read() == 0 {
        read_streamed_script_data_call(g(0xA47B60));
    }
    w32(0xA4448C, 0xFFFF_FFFF);
    w8(0xA444B0, 0);
    w32(0xA444AC, 0);
    w8(0xA444A8, 0);
    w8(0xA444A7, 1);
    for a in 0xA444A1..=0xA444A6 {
        w8(a, 0);
    }
    w8(0xA444A0, 1);
    w32(0xA4449C, 0);
    w32(0xA44498, 0);
    w8(0xA44496, 1);
    w8(0xA44495, 1);
    w8(0xA44494, 1);
    w32(0xA44490, 0);
    w8(0xB6EC2C, 0);
    w8(0xA4448B, 1);
    w8(0xA4448A, 0);
    w8(0xA44489, 1);
    w8(0xA44488, 1);
    // ScriptSphereArray (16 x 0x18)
    let mut b = 0xA91268usize;
    while b + 2 < 0xA913EA {
        w32(b + 8, 0);
        w8(b, 0);
        w16(b + 2, 1);
        w32(b + 4, 0);
        w32(b + 0x14, 0);
        w32(b + 0xC, 0);
        w32(b + 0x10, 0);
        b += 0x18;
    }
    // IntroTextLines (96 x 0x44)
    let mut t = 0xA913E8usize;
    while t + 0xA < 0xA92D72 {
        w32(t, 0x3EF5_C28F);
        w32(t + 4, 0x3F8F_5C29);
        rgba(t + 8, [0xE1, 0xE1, 0xE1, 0xFF]);
        let width = g::<i32>(MAXIMUM_WIDTH).read() as f32;
        g::<f32>(t + 0x10).write(width);
        g::<f32>(t + 0x14).write(width);
        w8(t + 0xC, 0);
        w8(t + 0x24, 0);
        w8(t + 0xD, 0);
        w8(t + 0xE, 0);
        w8(t + 0xF, 0);
        rgba(t + 0x18, [0x80, 0x80, 0x80, 0x80]);
        w8(t + 0x1C, 1);
        rgba(t + 0x1D, [0, 0, 0, 0xFF]);
        w8(t + 0x21, 2);
        w8(t + 0x22, 0);
        w8(t + 0x23, 0);
        w32(t + 0x28, 1);
        w32(t + 0x2C, 0);
        w32(t + 0x30, 0);
        w32(t + 0x34, 0);
        w32(t + 0x38, 0);
        w32(t + 0x3C, 0xFFFF_FFFF);
        w32(t + 0x40, 0xFFFF_FFFF);
        t += 0x44;
    }
    w16(0xA44B68, 0);
    w8(0xA44B67, 0);
    w8(0xA44B66, 0);
    w16(0xA44B64, 0);
    w16(0xA44B60, 0);
    // IntroRectangles (128 x 0x3C)
    let mut r = 0xA92D68usize;
    while r + 0x1E < 0xA94B86 {
        w32(r, 0);
        w8(r + 4, 0);
        w16(r + 6, 0xFFFF);
        zero_dwords(r + 8, 5);
        rgba(r + 0x1C, [0xFF; 4]);
        w8(r + 0x20, 0);
        w32(r + 0x38, 3);
        r += 0x3C;
    }
    w16(0xA44B5C, 0);
    for i in 0..0x80 {
        sprite_delete(g(0xA94B68 + i * 4));
    }
    let slot = find_txd_slot(g(0x859D4C));
    if slot != -1 {
        let pool = GPtr::<u8>::from_addr(g::<u32>(0xC8800C).read() as usize);
        let flags = GPtr::<u8>::from_addr(pool.byte_add(4).cast::<u32>().read() as usize);
        let entry = if flags.byte_add(slot as isize).read() & 0x80 != 0 {
            0u32
        } else {
            pool.cast::<u32>().read().wrapping_add(slot as u32 * 12)
        };
        if g::<u32>(entry as usize).read() != 0 {
            remove_txd(slot);
        }
    }
    let mut bs = 0xA44A30usize;
    while bs + 4 < 0xA44B60 {
        w32(bs, 0);
        w32(bs + 4, 0xFFFF_FFFF);
        w32(bs + 8, 0xFFFF_FFFF);
        bs += 0xC;
    }
    zero_dwords(0xA449E0, 0x14);
    clear_all_suppressed_car_models();
    clear_all_vehicle_models_blocked_by_script();
    initialise_all_connect_lod_objects_call();
    initialise_special_anim_groups_call();
    let mut fx = 0xA44110usize;
    while fx + 2 < 0xA44212 {
        w8(fx, 0);
        w16(fx + 2, 1);
        w32(fx + 4, 0);
        fx += 8;
    }
    let mut sl = 0xA94D68usize;
    while sl + 0x10 < 0xA95158 {
        w8(sl, 0);
        w8(sl + 1, 0);
        w8(sl + 2, 1);
        w8(sl + 3, 0);
        w16(sl + 4, 1);
        zero_dwords(sl + 8, 20);
        sl += 0x7C;
    }
    w16(0xA90830, 0);
    let mut cp = 0xA44070usize;
    while cp + 2 < 0xA44112 {
        w8(cp, 0);
        w16(cp + 2, 1);
        w32(cp + 4, 0);
        cp += 8;
    }
    w16(0xA44068, 0);
    let mut sq = 0xA43F68usize;
    while sq + 2 < 0xA4406A {
        w8(sq, 0);
        w16(sq + 2, 1);
        sq += 4;
    }
    scripted_2d_effects_init_call();
    task_sequences_init();
    ped_groups_init();
    inform_friends_event_queue_init();
    inform_group_event_queue_init();
    decision_maker_types_get_instance();
}

/// `CTheScripts::ReadObjectNamesFromScript()` (stub 0x486720, body 0x156F5A0): from the first SCM
/// segment (at `[ScriptSpace + 3]`): `NumberOfUsedObjects` = the low word of the count (+8), and that many
/// 24-byte names (from +0xC) copied into `UsedObjectArray` (0x1C each, from index 0) with model index
/// 0. (The body's destination pointer comes from a protection call; gta-reversed and the table layout
/// put it at the array's first entry.)
#[hook(addr = 0x486720, conv = cdecl, name = "CTheScripts::ReadObjectNamesFromScript", expect = "E9 7B 8E 0E 01",
       unverified = "std6-script-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn read_object_names_from_script() {
    let off = g::<u32>(SCRIPT_SPACE + 3).read() as usize;
    let count = g::<u16>(SCRIPT_SPACE + off + 8).read();
    w16(0xA44B6C, count);
    let mut src = SCRIPT_SPACE + off + 0xC;
    for i in 0..count as usize {
        let dst = 0xA44B70 + i * 0x1C;
        for k in 0..0x18 {
            w8(dst + k, g::<u8>(src + k).read());
        }
        w32(dst + 0x18, 0);
        src += 0x18;
    }
}

/// `CStreamedScripts::ReadStreamedScriptData()` (stub 0x470750, body 0x156F450, thiscall): from the third
/// SCM segment: the largest streamed script size (+0xA40); then for each script registered from
/// script.img (count +0xA44) the next record (name[20], offset, size) is matched by `_stricmp` against
/// the registered names (+8 + j * 0x20) and the match (or entry -1 when none) gets the size (+0x1C) and
/// the record's index (+6).
///
/// Not registered: 0x470750 is hooked by sa-script (batch of WRITER-MVP-J) (duplicate port left unregistered when STD-6 merged both).
pub fn read_streamed_script_data(this: GPtr<u8>) {
    let s1 = g::<u32>(SCRIPT_SPACE + 3).read();
    let s2 = g::<u32>((SCRIPT_SPACE as u32).wrapping_add(3).wrapping_add(s1) as usize).read();
    let s3 = g::<u32>((SCRIPT_SPACE as u32).wrapping_add(3).wrapping_add(s2) as usize).read();
    let mut src = (SCRIPT_SPACE as u32 + 8).wrapping_add(s3) as usize;
    this.byte_add(0xA40).cast::<u32>().write(g::<u32>(src).read());
    src += 8;
    let count = || this.byte_add(0xA44).cast::<u16>().read();
    let mut k: u32 = 0;
    while (k as u16) < count() {
        let mut name = [0u8; 0x14];
        for (i, c) in name.iter_mut().enumerate() {
            *c = g::<u8>(src + i).read();
        }
        src += 0x14;
        let mut found: i32 = -1;
        let mut j = 0u32;
        while (j as u16) < count() {
            if crt_stricmp(this.byte_add(8 + j as isize * 0x20), GPtr::from_raw(name.as_mut_ptr())) == 0 {
                found = j as i32;
                break;
            }
            j += 1;
        }
        let e = this.byte_add(found as isize * 0x20);
        e.byte_add(0x1C).cast::<u32>().write(g::<u32>(src + 4).read());
        src += 8;
        e.byte_add(6).cast::<u16>().write(k as u16);
        k += 1;
    }
}

/// `CTheScripts::InitialiseAllConnectLodObjects()` (stub 0x470960, body 0x1562910): the 10 pairs at
/// 0xA44800 = (-1, -1).
///
/// Not registered: 0x470960 is hooked by sa-script (batch of WRITER-MVP-J) (duplicate port left unregistered when STD-6 merged both).
pub fn initialise_all_connect_lod_objects() {
    for i in 0..10 {
        w32(0xA44800 + i * 8, 0xFFFF_FFFF);
        w32(0xA44804 + i * 8, 0xFFFF_FFFF);
    }
}

/// `CTheScripts::InitialiseSpecialAnimGroupsAttachedToCharModels()` (stub 0x474730, body 0x1562530): the
/// 8 entries at 0xA44850 (0x14 each): model -1, first name byte 0.
///
/// Not registered: 0x474730 is hooked by sa-script (batch of WRITER-MVP-J) (duplicate port left unregistered when STD-6 merged both).
pub fn initialise_special_anim_groups_attached_to_char_models() {
    for i in 0..8 {
        w32(0xA44850 + i * 0x14, 0xFFFF_FFFF);
        w8(0xA44854 + i * 0x14, 0);
    }
}

/// `CScripted2dEffects::Init()` (0x6FA6F0): for the 64 effects: owner flag 0 (0xC3A1A0), type 1
/// (0xC3A120), slot -1 (0xC3A020), 4 + 4 ped slots -1 (0xC3A200, 0x24 each) and their flag (+0x20) 0, the
/// effect (0xC3BB00, 0x94 each) reset (0x6F9EB0), user flag 0 (0xC39FE0) and radius -1.0 (0xC39EE0).
///
/// Not registered: 0x6FA6F0 is hooked by sa-script (batch of WRITER-MVP-J) (duplicate port left unregistered when STD-6 merged both).
pub fn scripted_2d_effects_init() {
    for i in 0..0x40usize {
        w8(0xC3A1A0 + i, 0);
        w16(0xC3A120 + i * 2, 1);
        w32(0xC3A020 + i * 4, 0xFFFF_FFFF);
        let p = 0xC3A200 + i * 0x24;
        for k in 0..4 {
            w32(p + k * 4, 0xFFFF_FFFF);
            w32(p + k * 4 + 0x10, 0xFFFF_FFFF);
        }
        w8(p + 0x20, 0);
        effect_2d_init(g(0xC3BB00 + i * 0x94));
        w8(0xC39FE0 + i, 0);
        w32(0xC39EE0 + i * 4, 0xBF80_0000);
    }
}

orig! {
    /// `CTheScripts::StartNewScript(ip)` (0x464C20)
    fn start_new_script(ip: GPtr<u8>) -> GPtr<u8> = cdecl 0x464C20 => unported;
}

/// `CTheScripts::StartTestScript()` (stub 0x464D40, body 0x1565610): `StartNewScript(ScriptSpace)`.
#[hook(addr = 0x464D40, conv = cdecl, name = "CTheScripts::StartTestScript", expect = "E9 CB 08 10 01",
       unverified = "std6-script-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn start_test_script() -> GPtr<u8> {
    start_new_script(g(SCRIPT_SPACE))
}

/// `CTheScripts::IsPlayerOnAMission()` (stub 0x464D50, body 0x156AA10): `OnAMissionFlag` set and the
/// script-space dword it indexes 1. AL only; the upper bytes of EAX keep the flag's value.
#[hook(addr = 0x464D50, conv = cdecl, name = "CTheScripts::IsPlayerOnAMission", expect = "E9 BB 5C 10 01",
       unverified = "std6-script-1", route = "L1-pure", hazards = "ax-al-return")]
pub fn is_player_on_a_mission() -> u32 {
    let f = g::<u32>(0xA476AC).read();
    let on = f != 0 && g::<u32>(SCRIPT_SPACE + f as usize).read() == 1;
    (f & 0xFFFF_FF00) | on as u32
}

/// `CRunningScript::AddScriptToList(listHead)` (stub 0x464C00, body 0x1561990, thiscall): linked in
/// front of `*listHead`.
#[hook(addr = 0x464C00, conv = thiscall, name = "CRunningScript::AddScriptToList", expect = "E9 8B CD 0F 01", preserve = "ecx",
       unverified = "std6-script-1", route = "L1-pure", hazards = "none")]
pub fn add_script_to_list(this: GPtr<u8>, head: GPtr<u32>) {
    let a = this.addr() as u32;
    this.cast::<u32>().write(head.read());
    this.byte_add(4).cast::<u32>().write(0);
    let first = head.read();
    if first != 0 {
        g::<u32>(first as usize + 4).write(a);
    }
    head.write(a);
}

/// `CRunningScript::RemoveScriptFromList(listHead)` (stub 0x464BD0, body 0x156DD60, thiscall, `ret 4`; STD-7):
/// `prev (+4) ? prev->next = next : *listHead = next`; `next (+0) ? next->prev = prev`.
#[hook(addr = 0x464BD0, conv = thiscall, name = "CRunningScript::RemoveScriptFromList", expect = "E9 8B 91 10 01",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn remove_script_from_list(this: GPtr<u8>, head: GPtr<u32>) {
    let prev = this.byte_add(4).cast::<u32>().read();
    let next = this.cast::<u32>().read();
    if prev != 0 {
        g::<u32>(prev as usize).write(next);
    } else {
        head.write(next);
    }
    let next = this.cast::<u32>().read();
    if next != 0 {
        g::<u32>(next as usize + 4).write(this.byte_add(4).cast::<u32>().read());
    }
}

/// `CRunningScript::IsPedDead(CPed*)` (stub 0x464D70, body 0x156E7B0, thiscall, `ret 4`; STD-7): true when the
/// ped state (+0x530) is 0x37, 0x36 or 0x38 (dead, die, die by stealth). AL only (AH keeps the state's byte 1).
#[hook(addr = 0x464D70, conv = thiscall, name = "CRunningScript::IsPedDead", expect = "E9 3B 9A 10 01",
       route = "L2", hazards = "ax-al-return", recs = "new-game")]
pub fn is_ped_dead(_this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let st = ped.byte_add(0x530).cast::<u32>().read();
    u8::from(st == 0x37 || st == 0x36 || st == 0x38)
}

/// `CTheScripts::ReinitialiseSwitchStatementData()` (stub 0x470370, body 0x156E5F0): the switch
/// statement state cleared (0xA43F60 word, 0xA43F5C dword, 0xA43F58 byte, 0xA43F54 dword, 0xA43F50 word).
#[hook(addr = 0x470370, conv = cdecl, name = "CTheScripts::ReinitialiseSwitchStatementData", expect = "E9 7B E2 0F 01",
       unverified = "std6-script-1", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn reinitialise_switch_statement_data() {
    w16(0xA43F60, 0);
    w32(0xA43F5C, 0);
    w8(0xA43F58, 0);
    w32(0xA43F54, 0);
    w16(0xA43F50, 0);
}

orig! {
    /// `CWorld::FindObjectsKindaColliding(pos, radius, b2d, count, max, out, b, v, p, o, d)` (0x568B80)
    fn find_objects_kinda_colliding(pos: GPtr<u8>, radius: u32, b2d: u32, count: GPtr<i16>, max: i32, out: GPtr<u32>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x568B80 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560), `CSimpleTransform::UpdateMatrix(matrix)` (0x54F1B0)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CCollision::ProcessColModels(m1, cm1, m2, cm2, points, lines, depths, b)` (0x4185C0)
    fn process_col_models(m1: GPtr<u8>, c1: GPtr<u8>, m2: GPtr<u8>, c2: GPtr<u8>, pts: u32, lines: u32, depths: u32, b: u32) -> i32 = cdecl 0x4185C0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), `CPed::CanBeDeleted()` (0x5DF100), `CVehicle::CanBeDeleted()` (0x6D1180)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    fn ped_can_be_deleted(p: GPtr<u8>) -> u8 = thiscall 0x5DF100 => unported;
    fn vehicle_can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CPopulation::RemovePed(ped)` (0x610F20)
    fn population_remove_ped(p: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// `CEntity::CleanUpOldReference(slot)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u32>) = thiscall 0x571A00 => unported;
    /// `CVehicle::RemovePassenger(ped)` (0x6D1610)
    fn vehicle_remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CCarCtrl::RemoveFromInterestingVehicleList(v)` (0x423ED0), `CWorld::Remove(e)` (0x563280)
    fn remove_from_interesting_vehicle_list(v: GPtr<u8>) = cdecl 0x423ED0 => unported;
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// The entity's virtual deleting destructor (vtable slot 0)
    fn entity_vdtor(e: GPtr<u8>, flags: u32) = thiscall vslot 0 => unported;
}

/// Ensures the entity has a matrix (allocated and filled from its placement when missing).
fn ensure_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if e.byte_add(0x14).cast::<u32>().read() == 0 {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), GPtr::from_addr(e.byte_add(0x14).cast::<u32>().read() as usize));
    }
    GPtr::from_addr(e.byte_add(0x14).cast::<u32>().read() as usize)
}

/// `CTheScripts::ClearSpaceForMissionEntity(pos, entity)` (0x486B00): up to 16 vehicles / peds within the
/// entity model's collision radius (`FindObjectsKindaColliding`, peds in vehicles dropped); for each
/// other one whose collision overlaps the entity's (`ProcessColModels` into the static points 0xA951F8,
/// with the entity's suspension lines switched off for the test and restored after it): a deletable
/// unlocked vehicle loses its driver and passengers (`RemovePed`) and is removed and deleted; a
/// deletable non-player ped is removed.
#[hook(addr = 0x486B00, conv = cdecl, name = "CTheScripts::ClearSpaceForMissionEntity", expect = "A0 78 57 A9 00 83 EC 50",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn clear_space_for_mission_entity(pos: GPtr<u8>, ours: GPtr<u8>) {
    if g::<u8>(0xA95778).read() & 1 == 0 {
        w32(0xA95778, g::<u32>(0xA95778).read() | 1);
    }
    let mi = g::<u32>(0xA9B0C8 + 4 * ours.byte_add(0x22).cast::<i16>().read() as isize as usize).read() as usize;
    let radius = g::<u32>(g::<u32>(mi + 0x14).read() as usize + 0x24).read();
    let mut ents = [0u32; 16];
    let mut count: i16 = 0;
    find_objects_kinda_colliding(pos, radius, 0, GPtr::from_raw(&mut count as *mut i16), 0x10, GPtr::from_raw(ents.as_mut_ptr()), 0, 1, 1, 0, 0);
    if count <= 0 {
        return;
    }
    let n = count as usize;
    for e in ents.iter_mut().take(n) {
        let p = GPtr::<u8>::from_addr(*e as usize);
        if p != ours && p.byte_add(0x36).read() & 7 == 3 && p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
            *e = 0;
        }
    }
    for &ea in ents.iter().take(n) {
        let e = GPtr::<u8>::from_addr(ea as usize);
        if e == ours || e.is_null() {
            continue;
        }
        let cm = entity_get_col_model(ours);
        let cd = GPtr::<u8>::from_addr(cm.byte_add(0x2C).cast::<u32>().read() as usize);
        if cd.is_null() {
            continue;
        }
        let lines = cd.byte_add(6).read() as i8;
        let saved = sa_math::x87::X87::from_i32(lines as i32).to_f32();
        if lines > 0 {
            cd.byte_add(6).write(0);
        }
        let em = ensure_matrix(e);
        let om = ensure_matrix(ours);
        let hits = process_col_models(om, cm, em, entity_get_col_model(e), 0xA951F8, 0, 0, 0);
        if saved > 0.0 {
            cd.byte_add(6).write(sa_math::x87::X87::from_f32(saved).ftol() as u8);
        }
        if hits <= 0 {
            continue;
        }
        match e.byte_add(0x36).read() & 7 {
            2 => {
                if e.byte_add(0x428).read() & 8 != 0 || vehicle_can_be_deleted(e) == 0 {
                    continue;
                }
                let slot = e.byte_add(0x460).cast::<u32>();
                let driver = slot.read();
                if driver != 0 {
                    population_remove_ped(GPtr::from_addr(driver as usize));
                    let d = slot.read();
                    if d != 0 {
                        clean_up_old_reference(GPtr::from_addr(d as usize), slot);
                    }
                    slot.write(0);
                }
                let mut k = 0u32;
                while k < e.byte_add(0x488).read() as u32 {
                    let p = e.byte_add(0x464 + k as isize * 4).cast::<u32>().read();
                    if p != 0 {
                        vehicle_remove_passenger(e, GPtr::from_addr(p as usize));
                        population_remove_ped(GPtr::from_addr(p as usize));
                    }
                    k += 1;
                }
                remove_from_interesting_vehicle_list(e);
                world_remove(e);
                entity_vdtor(e, 1);
            }
            3 if ped_is_player(e) == 0 && ped_can_be_deleted(e) != 0 => {
                population_remove_ped(e);
            }
            _ => {}
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CLEAR_SPACE_FOR_MISSION_ENTITY,
    HOOK_REINITIALISE_SWITCH_STATEMENT_DATA,
    HOOK_START_TEST_SCRIPT,
    HOOK_IS_PLAYER_ON_A_MISSION,
    HOOK_ADD_SCRIPT_TO_LIST,
    HOOK_REMOVE_SCRIPT_FROM_LIST,
    HOOK_IS_PED_DEAD,
    HOOK_THE_SCRIPTS_INIT,
    HOOK_READ_OBJECT_NAMES_FROM_SCRIPT,
];
