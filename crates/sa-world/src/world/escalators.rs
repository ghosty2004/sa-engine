//! `CEscalator` / `CEscalators` and `CStuntJumpManager` leaves (PORT-WORLD). Ported from the 1.0 US
//! bytes (gta-reversed `Escalator.cpp`, `Escalators.cpp`, `StuntJumpManager.cpp` as reading aids).
//!
//! `CEscalator` (0x150), `CEscalators::aEscalators[32]` at 0xC6E9A8: +0x30 `CMatrix`, +0x70/+0x74, +0x78
//! `m_bExist`, +0x79 `m_bObjectsCreated`, +0x7C/+0x80/+0x84 step counts, +0xA8 `CObject*[42]`.
//! `CStuntJump` (0x44) pool `mp_poolStuntJumps` (0xA9A888).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr};
use super::pickups::{entity_vdelete, world_remove};

pub const NUM_ESCALATORS: usize = 32;
pub const ESCALATOR_SIZE: usize = 0x150;
const ESCALATORS: usize = 0xC6E9A8;

const _: () = assert!(ESCALATOR_SIZE == sa_core::layouts::size::CEscalator);

sa_core::globals! {
    list GLOBALS;
    /// the byte `CEscalator::SwitchOff` sets around each object deletion (0xC6E98C)
    pub static ESCALATOR_DELETING @ 0xC6E98C: u8 = 0;
    /// `CStuntJumpManager::mp_poolStuntJumps`
    pub static POOL_STUNT_JUMPS @ 0xA9A888: GPtr<u8> = GPtr::NULL;
    /// `CStuntJumpManager::m_jumpState`... (0xA9A88C, gr `mp_Active`)
    pub static SJ_ACTIVE @ 0xA9A88C: u32 = 0;
    /// `CStuntJumpManager::m_bStuntJumpsEnabled`
    pub static SJ_ENABLED @ 0xA9A890: u8 = 0;
    /// `CStuntJumpManager::m_bHitReward`
    pub static SJ_HIT_REWARD @ 0xA9A891: u8 = 0;
    /// `CStuntJumpManager::m_iTimer`
    pub static SJ_TIMER @ 0xA9A894: u32 = 0;
    /// `CStuntJumpManager::m_jumpState`
    pub static SJ_STATE @ 0xA9A898: u32 = 0;
    /// `CStuntJumpManager::m_iNumJumps`
    pub static SJ_NUM_JUMPS @ 0xA9A89C: i32 = 0;
    /// `CStuntJumpManager::m_iNumCompleted`
    pub static SJ_NUM_COMPLETED @ 0xA9A8A0: u32 = 0;
}

pub use crate::bindings::stunt_jump_pool_new;
pub use crate::bindings::matrix_dtor;
pub use crate::bindings::escalator_switch_off as switch_off_call;
pub use sa_core::bindings::crt_new as operator_new;
orig! {
    /// `CEscalator::AddThisOne(...)` (0x717970)
    pub fn add_this_one(e: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) = thiscall 0x717970 => unported;
    /// `CEscalator::Update` (0x717D30)
    pub fn escalator_update(e: GPtr<u8>) = thiscall 0x717D30 => unported;
    /// `CPool<CStuntJump>::CPool(int32 size, const char* name)` (`thunk_FUN_0156fa80`, 0x49C8F0)
    pub fn stunt_jump_pool_ctor(p: GPtr<u8>, size: i32, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x49C8F0 => unported;
    /// `CPool<CStuntJump>::Clear` (0x49C9A0)
    pub fn stunt_jump_pool_clear(p: GPtr<u8>) = thiscall 0x49C9A0 => unported;
}

fn escalator(i: usize) -> GPtr<u8> {
    g::<u8>(ESCALATORS + i * ESCALATOR_SIZE)
}

/// `CEscalator::CEscalator` (0x717110, thiscall): +0x70, +0x74, the object pointers and +0x79 = 0.
#[hook(addr = 0x717110, conv = thiscall, name = "CEscalator::Constructor", expect = "8B D1 33 C0 57", this = "CEscalator")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x70, 0);
    wr::<u32>(this, 0x74, 0);
    wr::<u8>(this, 0x79, 0);
    for k in 0..42 {
        wr::<u32>(this, 0xA8 + 4 * k, 0);
    }
    this
}

/// `CEscalator::~CEscalator` (0x717130, thiscall): `~CMatrix` of +0x30 (tail jump).
#[hook(addr = 0x717130, conv = thiscall, name = "CEscalator::Destructor", expect = "83 C1 30 E9 98 3B E8 FF", this = "CEscalator")]
pub fn destructor(this: GPtr<u8>) {
    matrix_dtor(this.byte_add(0x30));
}

/// `CEscalator::SwitchOff` (0x717860, thiscall): with objects created, removes and deletes every object
/// of the three step counts (re-read each step; 0xC6E98C = 1 around the delete), then +0x79 = 0.
#[hook(addr = 0x717860, conv = thiscall, name = "CEscalator::SwitchOff", expect = "56 8B F1 8A 46 79", this = "CEscalator")]
pub fn switch_off(this: GPtr<u8>) {
    if rd::<u8>(this, 0x79) == 0 {
        return;
    }
    let total = |t: GPtr<u8>| rd::<i32>(t, 0x7C).wrapping_add(rd::<i32>(t, 0x80)).wrapping_add(rd::<i32>(t, 0x84));
    let mut i: i32 = 0;
    while i < total(this) {
        let slot = this.byte_add(0xA8 + 4 * i as isize).cast::<GPtr<u8>>();
        let o = slot.read();
        if !o.is_null() {
            world_remove(o);
            ESCALATOR_DELETING.set(1);
            let o = slot.read();
            if !o.is_null() {
                entity_vdelete(o, 1);
            }
            ESCALATOR_DELETING.set(0);
            slot.write(GPtr::NULL);
        }
        i += 1;
    }
    wr::<u8>(this, 0x79, 0);
}

/// `CEscalators::Shutdown` (0x717940): `SwitchOff` and `m_bExist` = 0 for every escalator.
#[hook(addr = 0x717940, conv = cdecl, name = "CEscalators::Shutdown", expect = "56 BE 20 EA C6 00")]
pub fn shutdown() {
    for i in 0..NUM_ESCALATORS {
        switch_off_call(escalator(i));
        wr::<u8>(escalator(i), 0x78, 0);
    }
}

/// `CEscalators::Init` (0x717C50): as `Shutdown`, then `SwitchOff` of every escalator again.
#[hook(addr = 0x717C50, conv = cdecl, name = "CEscalators::Init", expect = "56 BE 20 EA C6 00")]
pub fn init() {
    shutdown();
    for i in 0..NUM_ESCALATORS {
        switch_off_call(escalator(i));
    }
}

/// `CEscalator::AddThisOne(const CVector& start, const CVector& bottom, const CVector& top, const CVector& end, bool
/// moveDown, CEntity* entity)` (0x717970, thiscall, `ret 0x18`). Exists; the four points are copied to +0 / +0xC /
/// +0x18 / +0x24 and lowered by the step model's (`MI_ESCALATORSTEP`) bounding box max z. Plane counts (the bytes'
/// x87 order: start's lowered z stays extended for the first distance's z term): +0x7C = `ftol(|bottom - top| * 2.5 +
/// 1)`, +0x80 = `1 - ftol(|start - bottom| * -0.3125)`, +0x84 = `1 - ftol(|top - end| * -0.3125)`. The rotation
/// (+0x30) from the normalised xy of the unlowered `start - bottom` `d`: right `(d.y, -d.x, 0)`, forward `(d.x, d.y,
/// 0)`, up `(0, 0, 1)`, position 0 (the matrix's padding and attach fields untouched). +0x7A = moveDown; the bounding
/// sphere (+0x90) centre `(start + end) / 2`, radius the distance to start; +0xA4 = entity, whose
/// `RegisterReference(&+0xA4)` is called (on a null entity as well).
#[hook(addr = 0x717970, conv = thiscall, name = "CEscalator::AddThisOne", this = "CEscalator",
       expect = "83 EC 24 8B 44 24 28 53", unverified = "frontier2-a19", route = "L2", hazards = "heap-writes",
       note = "IPL / script escalators (CEscalators::AddOne)")]
#[allow(clippy::too_many_arguments)]
pub fn escalator_add_this_one(this: GPtr<u8>, start: GPtr<u8>, bottom: GPtr<u8>, top: GPtr<u8>, end: GPtr<u8>,
                              move_down: u32, entity: GPtr<u8>) {
    use sa_core::bindings::{entity_register_reference, vector_normalise};
    use sa_math::fpu::{ftol32, x};
    let f = |o: isize| rd::<f32>(this, o);
    wr::<u8>(this, 0x78, 1);
    for (o, src) in [(0, start), (0xC, bottom), (0x18, top), (0x24, end)] {
        for k in 0..3 {
            wr::<u32>(this, o + 4 * k, rd::<u32>(src, 4 * k));
        }
    }
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * g::<u16>(0x8C_D674).read() as usize).read();
    let h = x(rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x14));
    let start_z = x(f(8)).fsub(h);
    wr::<f32>(this, 8, start_z.to_f32());
    for o in [0x14, 0x20, 0x2C] {
        wr::<f32>(this, o, x(f(o)).fsub(h).to_f32());
    }
    let sb = [x(f(0)).fsub_m(f(0xC)).to_f32(), x(f(4)).fsub_m(f(0x10)).to_f32()];
    let sbz = start_z.fsub_m(f(0x14));
    let (ax, ay, az) = (x(f(0xC)).fsub_m(f(0x18)), x(f(0x10)).fsub_m(f(0x1C)), x(f(0x14)).fsub_m(f(0x20)));
    let te = [0, 4, 8].map(|k| x(f(0x18 + k)).fsub_m(f(0x24 + k)).to_f32());
    let mid = az.fmul(az).fadd(ay.fmul(ay)).fadd(ax.fmul(ax)).fsqrt();
    wr::<i32>(this, 0x7C, ftol32(mid.fmul_m(2.5).fadd_m(1.0)));
    let lo = sbz.fmul(sbz).fadd(x(sb[1]).fmul_m(sb[1])).fadd(x(sb[0]).fmul_m(sb[0])).fsqrt();
    wr::<i32>(this, 0x80, 1i32.wrapping_sub(ftol32(lo.fmul_m(-0.3125))));
    let hi = x(te[2]).fmul_m(te[2]).fadd(x(te[1]).fmul_m(te[1])).fadd(x(te[0]).fmul_m(te[0])).fsqrt();
    wr::<i32>(this, 0x84, 1i32.wrapping_sub(ftol32(hi.fmul_m(-0.3125))));
    let a = |k: isize| rd::<f32>(start, k);
    let b = |k: isize| rd::<f32>(bottom, k);
    let mut d = [x(a(0)).fsub_m(b(0)).to_f32(), x(a(4)).fsub_m(b(4)).to_f32(), 0.0f32];
    vector_normalise(GPtr::from_raw(&mut d));
    wr::<f32>(this, 0x34, x(d[0]).fchs().to_f32());
    wr::<f32>(this, 0x40, d[0]);
    wr::<f32>(this, 0x44, d[1]);
    wr::<f32>(this, 0x30, d[1]);
    wr::<u8>(this, 0x7A, move_down as u8);
    for o in [0x50, 0x54, 0x48, 0x38, 0x68, 0x64, 0x60] {
        wr::<u32>(this, o, 0);
    }
    wr::<f32>(this, 0x58, 1.0);
    let cz = x(f(0x2C)).fadd_m(f(8)).to_f32();
    wr::<f32>(this, 0x90, x(f(0x24)).fadd_m(f(0)).fmul_m(0.5).to_f32());
    wr::<f32>(this, 0x94, x(f(0x28)).fadd_m(f(4)).fmul_m(0.5).to_f32());
    wr::<f32>(this, 0x98, x(cz).fmul_m(0.5).to_f32());
    let (dx, dy, dz) = (x(f(0)).fsub_m(f(0x90)), x(f(4)).fsub_m(f(0x94)), x(f(8)).fsub_m(f(0x98)));
    wr::<GPtr<u8>>(this, 0xA4, entity);
    wr::<f32>(this, 0x9C, dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32());
    entity_register_reference(entity, this.byte_add(0xA4));
}

/// `CEscalators::AddOne(...)` (0x717C90): `AddThisOne` on the first escalator without `m_bExist`, with
/// the six argument dwords; nothing when all 32 exist.
#[hook(addr = 0x717C90, conv = cdecl, name = "CEscalators::AddOne", expect = "33 C9 B8 20 EA C6 00")]
pub fn add_one(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) {
    for i in 0..NUM_ESCALATORS {
        if rd::<u8>(escalator(i), 0x78) == 0 {
            add_this_one(escalator(i), a0, a1, a2, a3, a4, a5);
            return;
        }
    }
}

/// `CEscalators::Update` (0x718580): nothing during replay playback (`CReplay::Mode` 0xA43088 == 1);
/// else `CEscalator::Update` of every existing escalator.
#[hook(addr = 0x718580, conv = cdecl, name = "CEscalators::Update", expect = "80 3D 88 30 A4 00 01")]
pub fn update() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    for i in 0..NUM_ESCALATORS {
        if rd::<u8>(escalator(i), 0x78) != 0 {
            escalator_update(escalator(i));
        }
    }
}

/// `CStuntJumpManager::Init` (stub 0x49CA50 → 0x156DDB0): `new` pool of 256 "Stunt Jumps" (null when
/// `new` fails), enabled = 1. The SEH frame around the constructor is not reproduced.
#[hook(addr = 0x49CA50, conv = cdecl, name = "CStuntJumpManager::Init", expect = "E9 5B 13 0D 01")]
pub fn sj_init() {
    let p = operator_new(0x14);
    let pool = if p.is_null() { GPtr::NULL } else { stunt_jump_pool_ctor(p, 0x100, g(0x85A580)) };
    POOL_STUNT_JUMPS.set(pool);
    SJ_ENABLED.set(1);
}

/// `CStuntJumpManager::ShutdownForRestart` (stub 0x49CB10 → 0x156E3B0): pool `Clear`, state reset.
#[hook(addr = 0x49CB10, conv = cdecl, name = "CStuntJumpManager::ShutdownForRestart", expect = "E9 9B 18 0D 01")]
pub fn sj_shutdown_for_restart() {
    stunt_jump_pool_clear(POOL_STUNT_JUMPS.get());
    SJ_ACTIVE.set(0);
    SJ_HIT_REWARD.set(0);
    SJ_TIMER.set(0);
    SJ_STATE.set(0);
    SJ_NUM_JUMPS.set(0);
    SJ_NUM_COMPLETED.set(0);
}

/// `CStuntJumpManager::AddOne(const CBoundingBox& start, const CBoundingBox& end, const CVector& camera,
/// int32 reward)` (stub 0x49CB40 → 0x1561A90): a new pool slot (-1 without a pool or a free slot) gets
/// the boxes, camera and reward, done/found flags 0; `m_iNumJumps++`; returns the slot index.
#[hook(addr = 0x49CB40, conv = cdecl, name = "CStuntJumpManager::AddOne", expect = "E9 4B 4F 0C 01")]
pub fn sj_add_one(start: GPtr<u32>, end: GPtr<u32>, camera: GPtr<u32>, reward: i32) -> i32 {
    let pool = POOL_STUNT_JUMPS.get();
    if pool.is_null() {
        return -1;
    }
    let j = stunt_jump_pool_new(pool);
    if j.is_null() {
        return -1;
    }
    let d = j.cast::<u32>();
    for k in 0..6 {
        d.wrapping_index(k).write(start.wrapping_index(k).read());
    }
    for k in 0..6 {
        d.wrapping_index(6 + k).write(end.wrapping_index(k).read());
    }
    for k in 0..3 {
        d.wrapping_index(12 + k).write(camera.wrapping_index(k).read());
    }
    wr::<i32>(j, 0x3C, reward);
    wr::<u8>(j, 0x40, 0);
    wr::<u8>(j, 0x41, 0);
    SJ_NUM_JUMPS.set(SJ_NUM_JUMPS.get().wrapping_add(1));
    let base = rd::<GPtr<u8>>(POOL_STUNT_JUMPS.get(), 0);
    (j.addr() as i32).wrapping_sub(base.addr() as i32) / 0x44
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_SWITCH_OFF,
    HOOK_SHUTDOWN,
    HOOK_INIT,
    HOOK_ESCALATOR_ADD_THIS_ONE,
    HOOK_ADD_ONE,
    HOOK_UPDATE,
    HOOK_SJ_INIT,
    HOOK_SJ_SHUTDOWN_FOR_RESTART,
    HOOK_SJ_ADD_ONE,
];
