//! `CCarEnterExit::SetAnimOffsetForEnterOrExitVehicle` (STD-6, batch `std6-veh-1`). Ported from the
//! 1.0 US bytes (0x64F860); gta-reversed `CarEnterExit.cpp` as reading aid (it zeroes the offsets of
//! empty sequences; the bytes leave them unchanged).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CAnimManager::GetAnimationBlockIndex(name)` (0x4D3990)
    fn get_animation_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CStreaming::RequestModel(id, flags)` (0x4087E0)
    fn request_model(id: i32, flags: u32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::LoadAllRequestedModels(priorityOnly)` (0x40EA10)
    fn load_all_requested_models(b: u32) = cdecl 0x40EA10 => unported;
    /// `CAnimManager::AddAnimBlockRef(index)` (0x4D3FB0), `RemoveAnimBlockRef(index)` (0x4D3FD0)
    fn add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CAnimManager::GetAnimAssociation(group, anim)` (0x4D3A60)
    fn get_anim_association(group: i32, anim: i32) -> GPtr<u8> = cdecl 0x4D3A60 => unported;
    /// `CAnimManager::UncompressAnimation(hierarchy)` (0x4D41C0)
    fn uncompress_animation(h: GPtr<u8>) = cdecl 0x4D41C0 => unported;
}

/// `CCarEnterExit::ms_bPedOffsetsCalculated` and the offset vectors: get-up, quick dragged out, bed L,
/// bed R, desk, chair.
const CALCULATED: usize = 0xC18C20;
const GET_UP: usize = 0xC18C3C;
const QUICK_DRAGGED: usize = 0xC18C48;

fn rd<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

fn g<T: sa_core::mem::GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

/// The first sequence of the static association's hierarchy (uncompressed first).
fn first_sequence(group: i32, anim: i32) -> GPtr<u8> {
    let h = GPtr::<u8>::from_addr(rd::<u32>(get_anim_association(group, anim), 0x10) as usize);
    let seq = GPtr::<u8>::from_addr(rd::<u32>(h, 4) as usize);
    uncompress_animation(h);
    seq
}

/// `CCarEnterExit::SetAnimOffsetForEnterOrExitVehicle()` (0x64F860): once. With the "int_house" and
/// "int_office" anim blocks streamed in (model ids block + 0x63E7, flags 8) and referenced: the get-up
/// offset (group 0, anim 0x70) = the first sequence's first key frame translation (+0x14) when it has
/// frames; the quick-dragged-out offset (-1.841797, -0.3261719, -0.01269531); the bed L / bed R / desk /
/// chair offsets ((0x54, 0x151), (0x54, 0x154), (0x55, 0x148), (0x54, 0x157)) = last minus first key
/// frame translation (frame stride 32 with translation, else 20) when there are frames. Then the block
/// references are released and the flag set.
#[hook(addr = 0x64F860, conv = cdecl, name = "CCarEnterExit::SetAnimOffsetForEnterOrExitVehicle", expect = "A0 20 8C C1 00 83 EC 0C",
       unverified = "std6-veh-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn set_anim_offset_for_enter_or_exit_vehicle() {
    if g::<u8>(CALCULATED).read() != 0 {
        return;
    }
    let house = get_animation_block_index(GPtr::from_addr(0x866D40));
    let office = get_animation_block_index(GPtr::from_addr(0x866D28));
    request_model(office + 0x63E7, 8);
    request_model(house + 0x63E7, 8);
    load_all_requested_models(0);
    add_anim_block_ref(office);
    add_anim_block_ref(house);
    let seq = first_sequence(0, 0x70);
    if rd::<i16>(seq, 6) > 0 {
        let f = GPtr::<u8>::from_addr(rd::<u32>(seq, 8) as usize);
        for k in 0..3 {
            g::<u32>(GET_UP + k * 4).write(rd::<u32>(f, 0x14 + k as isize * 4));
        }
    }
    for (k, v) in [0xBFEB_C001u32, 0xBEA7_0001, 0xBC4F_FFFD].into_iter().enumerate() {
        g::<u32>(QUICK_DRAGGED + k * 4).write(v);
    }
    for (group, anim, out) in [(0x54, 0x151, 0xC18C54usize), (0x54, 0x154, 0xC18C60), (0x55, 0x148, 0xC18C6C), (0x54, 0x157, 0xC18C78)] {
        let seq = first_sequence(group, anim);
        let n = rd::<i16>(seq, 6);
        if n > 0 {
            let frames = rd::<u32>(seq, 8);
            let stride = if (rd::<u8>(seq, 4) >> 1) & 1 != 0 { 32 } else { 20 };
            let last = GPtr::<u8>::from_addr((frames + (n as u32 - 1) * stride + 0x14) as usize);
            let first = GPtr::<u8>::from_addr((frames + 0x14) as usize);
            for k in 0..3 {
                g::<u32>(out + k * 4).write(rd::<u32>(last, k as isize * 4));
            }
            for k in 0..3 {
                let v = g::<f32>(out + k * 4).read() - rd::<f32>(first, k as isize * 4);
                g::<f32>(out + k * 4).write(v);
            }
        }
    }
    remove_anim_block_ref(house);
    remove_anim_block_ref(office);
    g::<u8>(CALCULATED).write(1);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SET_ANIM_OFFSET_FOR_ENTER_OR_EXIT_VEHICLE];

/// No globals.
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
