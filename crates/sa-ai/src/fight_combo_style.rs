//! `CTaskSimpleFight`'s combo-style chooser 0x61C7F0 (batch `std8-ai-1`), ported from the 1.0 US bytes (not in
//! gta-reversed). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CAnimManager::GetAnimBlockName(AssocGroupId)` (0x4D3A30)
    fn anim_block_name(grp: u32) -> u32 = cdecl 0x4D3A30 => unported;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    fn animation_block(name: u32) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    fn request_model(model: i32, flags: u32) = cdecl 0x4087E0 => unported;
}

/// `CAnimManager::ms_aAnimAssocGroups` (pointer at 0xB4EA34; 0x14 bytes per group, the block pointer at +0)
const ASSOC_GROUPS: usize = 0xB4EA34;
/// `CAnimManager::ms_aAnimBlocks` (0xB5D4A0, 0x20 bytes each; +0x10 the loaded flag)
const ANIM_BLOCKS: u32 = 0xB5D4A0;
/// `CTaskSimpleFight::m_aComboData` (0xC170D0, 0x88 bytes per style from style 4; +0 the anim group, +0x85 flags)
const COMBO_DATA: u32 = 0xC170D0;

/// The animation block of group `grp`: the group's block pointer, else `GetAnimationBlock(GetAnimBlockName(grp))`.
fn group_block(grp: u32) -> GPtr<u8> {
    let b = rd::<GPtr<u8>>(g::<GPtr<u8>>(ASSOC_GROUPS).read(), grp as isize * 0x14);
    if b.is_null() {
        animation_block(anim_block_name(grp))
    } else {
        b
    }
}

fn block_index(b: GPtr<u8>) -> i32 {
    (b.addr() as u32).wrapping_sub(ANIM_BLOCKS) as i32 >> 5
}

/// `FUN_0061c7f0(CPed* ped, int8 command)` (0x61C7F0, thiscall `ret 8`; `CTaskSimpleFight`: +0xA block-ref held,
/// +0xC combo anim group, +0x26 the current command). Returns a fighting style in AL.
///
/// `command < 0`: with a group other than 0x21 and no reference held, a reference on the group's animation block
/// when it is loaded (+0xA = 1); returns 0. Commands other than 0, 2 and 11..14 return 0. Otherwise the style is the
/// current weapon's (`GetWeaponInfo(type, 1)` +0x6E); command 12, or style 4 with command 0 / 2, takes the ped's
/// own style (+0x72D). Command 0 with a style whose combo flags (+0x85) lack bit 2 returns 4. The style's combo
/// group (`m_aComboData[style - 4]`): 0x21 returns the style; a different group drops the held reference (+0xA = 0)
/// and switches +0xC; the same group with the reference held returns the style. Then the group's block: loaded →
/// `AddAnimBlockRef`, +0xA = 1, the style; not loaded → `CStreaming::RequestModel(block + 0x63E7, 8)`, a +0x26
/// command in 11..14 reset to 11, and 4.
#[hook(addr = 0x61C7F0, conv = thiscall, name = "FUN_0061c7f0", expect = "53 8A 5C 24 0C 84 DB 57",
       unverified = "std8-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk",
       note = "AL result; the original leaves EAX's upper bytes from the weapon-info pointer or the callee")]
pub fn fight_choose_combo_style(this: GPtr<u8>, ped: GPtr<u8>, command: u32) -> u8 {
    let cmd = command as u8 as i8;
    if cmd < 0 {
        let grp = rd::<u32>(this, 0xC);
        if grp != 0x21 && rd::<u8>(this, 0xA) == 0 {
            let b = group_block(grp);
            if rd::<u8>(b, 0x10) != 0 {
                add_anim_block_ref(block_index(b));
                wr::<u8>(this, 0xA, 1);
            }
        }
        return 0;
    }
    if !(cmd == 2 || cmd == 0 || (11..=14).contains(&cmd)) {
        return 0;
    }
    let slot = i32::from(rd::<i8>(ped, 0x718));
    let wi = weapon_info(rd::<u32>(ped, 0x5A0 + slot as isize * 0x1C), 1);
    let mut style = rd::<u8>(wi, 0x6E);
    if cmd == 0xC || (style == 4 && (cmd == 2 || cmd == 0)) {
        style = rd::<u8>(ped, 0x72D);
    }
    let combo =
        GPtr::<u8>::from_addr(COMBO_DATA.wrapping_add(((style as i8 as i32 - 4) * 0x88) as u32) as usize);
    if cmd == 0 && rd::<u8>(combo, 0x85) & 4 == 0 {
        return 4;
    }
    let cg = rd::<u32>(combo, 0);
    if cg == 0x21 {
        return style;
    }
    let cur = rd::<u32>(this, 0xC);
    if cg != cur {
        if rd::<u8>(this, 0xA) != 0 {
            let b = rd::<GPtr<u8>>(g::<GPtr<u8>>(ASSOC_GROUPS).read(), cur as isize * 0x14);
            remove_anim_block_ref(block_index(b));
            wr::<u8>(this, 0xA, 0);
        }
        wr::<u32>(this, 0xC, rd::<u32>(combo, 0));
    } else if rd::<u8>(this, 0xA) != 0 {
        return style;
    }
    let b = group_block(rd::<u32>(this, 0xC));
    let idx = block_index(b);
    if rd::<u8>(b, 0x10) != 0 {
        add_anim_block_ref(idx);
        wr::<u8>(this, 0xA, 1);
        return style;
    }
    request_model(idx + 0x63E7, 8);
    let c = rd::<i8>(this, 0x26);
    if (11..=14).contains(&c) {
        wr::<u8>(this, 0x26, 0xB);
    }
    4
}

pub const HOOKS: &[HookReg] = &[HOOK_FIGHT_CHOOSE_COMBO_STYLE];
