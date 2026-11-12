//! `CTaskComplexGotoDoorAndOpen::CreateFirstSubTask`, `CTaskComplexGoToCarDoorAndStandStill::ComputeRouteToDoor`
//! and the fight task's strafe-anim blend helper `FUN_0061c9b0`, executed by the original under SA:MP 0.3.7
//! (`catalog/samp-priority.csv`) (PORT-AI-8). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every
//! call to another catalogued function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// 0.0f
const K_ZERO: usize = 0x858B50;

orig! {
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(i: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CEventAreaCodes::CEventAreaCodes(CPed*)` (0x4B2190, `ret 4`)
    fn event_area_codes_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B2190 => unported;
    /// `CEventAreaCodes::~CEventAreaCodes` (`FUN_004b2210`)
    fn event_area_codes_dtor(e: GPtr<u8>) = thiscall 0x4B2210 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50, cdecl)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0, cdecl)
    fn clump_get_association(clump: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610, cdecl)
    fn anim_blend_animation(clump: u32, group: u32, anim: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0, cdecl)
    fn anim_add_animation(clump: u32, group: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0, cdecl)
    fn task_new_o(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(int32 moveState, const CVector&, float radius, bool, bool)`
    /// (0x667CD0, `ret 0x14`)
    fn go_to_point_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u8>, radius: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x667CD0 => unported;
    /// `CPointRoute::operator new(uint32)` (0x41B7C0, cdecl)
    fn point_route_new(size: u32) -> GPtr<u8> = cdecl 0x41B7C0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector&, CEntity&)` (0x5F3AC0, cdecl)
    fn geom_compute_entity_hit_side(p: GPtr<u8>, e: u32) -> i32 = cdecl 0x5F3AC0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanes(float z, CEntity&, CVector (*planes)[4], float* ds)`
    /// (0x5F3660, cdecl)
    fn geom_compute_entity_bbox_planes(z: u32, e: u32, planes: GPtr<u8>, ds: GPtr<u8>) = cdecl 0x5F3660 => unported;
    /// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(const CPed&, const CVector& start, CEntity&,
    /// const CVector& target, CPointRoute&, int32)` (0x5F3DD0, cdecl)
    fn geom_compute_route_round_entity_bbox(ped: GPtr<u8>, start: GPtr<u8>, e: u32, target: GPtr<u8>, route: GPtr<u8>, n: u32) -> u8 = cdecl 0x5F3DD0 => unported;
}

/// A dword-aligned stack object of `N` dwords, as a game pointer.
fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The f32 constant at `a`.
fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// An entity's position: its matrix +0x30, or the placement at +4 without a matrix.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskComplexGotoDoorAndOpen::CreateFirstSubTask(CPed*)` (0x66BD40, `ret 4`): a player's pad 0 gets bit 3 of
/// +0x10E and the task flag byte +0x34 bit 3; an area-codes event for the ped (a stack `CEventAreaCodes`) is added
/// to the global event group; the timer starts (+0x28 = now, +0x2C = 1000, +0x30 = 1). With a door entity (+0xC):
/// whether the door's position lies ahead of the ped along the door's forward axis (`(door - ped) . forward > 0`)
/// picks the side; the door position shifted by 0.75 * right becomes the approach point +0x10 (shifted by -0.5 *
/// forward toward the ped's side) and the target +0x1C (2.0 * forward to the far side); a door with flag 8 (+0x40)
/// sets +0x34 bit 2 and clears door flags 8 and 4; a playing anim 2 makes the ped blend anim 0 of its group (+0x4D4)
/// at 1000.0; the result is a new `CTaskSimpleGoToPoint(4, +0x10, 0.35)`. Without a door: null when +0x34 bit 0 is
/// set, else +0x34 bit 1 and `CTaskSimpleGoToPoint(4, +0x1C, 0.5)`. The event is destroyed before returning.
#[hook(addr = 0x66BD40, conv = thiscall, name = "CTaskComplexGotoDoorAndOpen::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "ai8-2", route = "L2", hazards = "heap-writes")]
pub fn goto_door_and_open_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if ped_is_player(ped) != 0 {
        let pad = pad_get_pad(0);
        wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) | 8);
        wr::<u8>(this, 0x34, rd::<u8>(this, 0x34) | 8);
    }
    let mut ev = [0u32; 8];
    event_area_codes_ctor(local(&mut ev), ped);
    event_group_add(get_event_global_group(), local(&mut ev), 0);
    wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 0x2C, 1000);
    wr::<u8>(this, 0x30, 1);
    let door = rd::<GPtr<u8>>(this, 0xC);
    let task = if door.is_null() {
        let fl = rd::<u8>(this, 0x34);
        if fl & 1 != 0 {
            event_area_codes_dtor(local(&mut ev));
            return GPtr::NULL;
        }
        wr::<u8>(this, 0x34, fl | 2);
        let t = task_new_o(0x30);
        if t.is_null() { GPtr::NULL } else { go_to_point_ctor(t, 4, this.byte_add(0x1C), 0x3F00_0000, 0, 0) }
    } else {
        let dp = pos_of(door);
        let (px, py, pz) = (rd::<f32>(dp, 0), rd::<f32>(dp, 4), rd::<f32>(dp, 8));
        let m = rd::<GPtr<u8>>(door, 0x14);
        let r = [rd::<f32>(m, 0), rd::<f32>(m, 4), rd::<f32>(m, 8)];
        let f = [rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18)];
        let q = pos_of(ped);
        let dz = x(pz).fsub_m(rd::<f32>(q, 8));
        let dy = x(py).fsub_m(rd::<f32>(q, 4));
        let dx = x(px).fsub_m(rd::<f32>(q, 0));
        let dot = x(f[1]).fmul(dy).fadd(x(f[2]).fmul(dz)).fadd(dx.fmul_m(f[0]));
        let front = !dot.fcom(x(k(K_ZERO))).test_any(0x41);
        let k1 = k(0x8D2EE0);
        let rz1 = x(r[2]).fmul_m(k1);
        let ry1 = x(r[1]).fmul_m(k1);
        let px1 = f32_of(x(r[0]).fmul_m(k1).fadd_m(px));
        let py1 = f32_of(x(py).fadd(ry1));
        let z = rz1.fadd_m(pz);
        let k2 = k(0x8D2EE4);
        let fz2 = x(f[2]).fmul_m(k2);
        let fy2 = f32_of(x(f[1]).fmul_m(k2));
        let fx2 = f32_of(x(f[0]).fmul_m(k2));
        let k3 = k(0x8D2EE8);
        let (a, b);
        if front {
            a = [f32_of(x(fx2).fadd_m(px1)), f32_of(x(py1).fadd_m(fy2)), f32_of(fz2.fadd(z))];
            let fz3 = x(f[2]).fmul_m(k3);
            let fy3 = f32_of(x(f[1]).fmul_m(k3));
            let fx3 = f32_of(x(f[0]).fmul_m(k3));
            b = [f32_of(x(fx3).fadd_m(px1)), f32_of(x(py1).fadd_m(fy3)), f32_of(z.fadd(fz3))];
        } else {
            a = [f32_of(x(px1).fsub_m(fx2)), f32_of(x(py1).fsub_m(fy2)), f32_of(z.fsub(fz2))];
            let fz3 = x(f[2]).fmul_m(k3);
            let fy3 = f32_of(x(f[1]).fmul_m(k3));
            let fx3 = f32_of(x(f[0]).fmul_m(k3));
            b = [f32_of(x(px1).fsub_m(fx3)), f32_of(x(py1).fsub_m(fy3)), f32_of(z.fsub(fz3))];
        }
        for (i, v) in a.into_iter().enumerate() {
            wr::<f32>(this, 0x10 + 4 * i as isize, v);
        }
        for (i, v) in b.into_iter().enumerate() {
            wr::<f32>(this, 0x1C + 4 * i as isize, v);
        }
        if rd::<u8>(door, 0x40) & 8 != 0 {
            wr::<u8>(this, 0x34, rd::<u8>(this, 0x34) | 4);
            wr::<u32>(door, 0x40, rd::<u32>(door, 0x40) & 0xFFFF_FFF7);
            let d2 = rd::<GPtr<u8>>(this, 0xC);
            wr::<u32>(d2, 0x40, rd::<u32>(d2, 0x40) & 0xFFFF_FFFB);
        }
        if !clump_get_association(rd::<u32>(ped, 0x18), 2).is_null() {
            anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 0, 0x447A_0000);
        }
        let t = task_new_o(0x30);
        if t.is_null() { GPtr::NULL } else { go_to_point_ctor(t, 4, this.byte_add(0x10), 0x3EB3_3333, 0, 0) }
    };
    event_area_codes_dtor(local(&mut ev));
    task
}

/// Appends `v` to a `CPointRoute` (count at +0, up to 8 points from +4) when it has room.
fn route_add(route: GPtr<u8>, v: [u32; 3]) {
    let n = rd::<i32>(route, 0);
    if n < 8 {
        let p = route.byte_add(n as isize * 12 + 4);
        wr::<u32>(p, 0, v[0]);
        wr::<u32>(p, 4, v[1]);
        wr::<u32>(p, 8, v[2]);
        wr::<i32>(route, 0, n + 1);
    }
}

/// `CTaskComplexGoToCarDoorAndStandStill::ComputeRouteToDoor(const CPed&)` (0x645910, `ret 4`): creates the point
/// route +0x4C on first use (`CPointRoute::operator new(0x64)`, count 0) and empties it; the vehicle's (+0xC)
/// bounding-box planes at the ped's height give the ped's hit side and the door target's (+0x38) hit side; the ped
/// and the target are each projected onto their side's plane (`p - (n . p + d) n`), the route round the box between
/// the two projections is computed into a stack `CPointRoute`, and the route becomes start projection, that route's
/// points, target projection (each only while fewer than 8 points).
#[hook(addr = 0x645910, conv = thiscall, name = "CTaskComplexGoToCarDoorAndStandStill::ComputeRouteToDoor", expect = "81 EC CC 00 00 00 57",
       unverified = "ai8-2", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn go_to_car_door_compute_route_to_door(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u32>(this, 0x4C) == 0 {
        let r = point_route_new(0x64);
        if !r.is_null() {
            wr::<u32>(r, 0, 0);
        }
        wr::<GPtr<u8>>(this, 0x4C, r);
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 0x4C), 0, 0);
    let veh = rd::<u32>(this, 0xC);
    let side1 = geom_compute_entity_hit_side(pos_of(ped), veh);
    let target = this.byte_add(0x38);
    let side2 = geom_compute_entity_hit_side(target, rd::<u32>(this, 0xC));
    let mut ds = [0u32; 4];
    let mut planes = [0u32; 12];
    geom_compute_entity_bbox_planes(rd::<u32>(pos_of(ped), 8), rd::<u32>(this, 0xC), local(&mut planes), local(&mut ds));
    let n = |s: i32, i: usize| f32::from_bits(planes[s as usize * 3 + i]);
    let d = |s: i32| f32::from_bits(ds[s as usize]);
    let p = pos_of(ped);
    let dd = x(n(side1, 2))
        .fmul_m(rd::<f32>(p, 8))
        .fadd(x(n(side1, 1)).fmul_m(rd::<f32>(p, 4)))
        .fadd(x(n(side1, 0)).fmul_m(rd::<f32>(p, 0)))
        .fadd_m(d(side1))
        .fchs();
    let v1x = f32_of(dd.fmul_m(n(side1, 0)));
    let v1y = f32_of(dd.fmul_m(n(side1, 1)));
    let v1z = dd.fmul_m(n(side1, 2));
    let p = pos_of(ped);
    let mut start = [
        f32_of(x(v1x).fadd_m(rd::<f32>(p, 0))).to_bits(),
        f32_of(x(v1y).fadd_m(rd::<f32>(p, 4))).to_bits(),
        f32_of(v1z.fadd_m(rd::<f32>(p, 8))).to_bits(),
    ];
    let mut local_route = [0u32; 0x19];
    let t = |i: isize| rd::<f32>(target, 4 * i);
    let d2 = x(t(2))
        .fmul_m(n(side2, 2))
        .fadd(x(t(1)).fmul_m(n(side2, 1)))
        .fadd(x(t(0)).fmul_m(n(side2, 0)))
        .fadd_m(d(side2))
        .fchs();
    let v2x = d2.fmul_m(n(side2, 0));
    let v2y = f32_of(d2.fmul_m(n(side2, 1)));
    let v2z = f32_of(d2.fmul_m(n(side2, 2)));
    let mut end = [
        f32_of(v2x.fadd_m(t(0))).to_bits(),
        f32_of(x(v2y).fadd_m(t(1))).to_bits(),
        f32_of(x(v2z).fadd_m(t(2))).to_bits(),
    ];
    geom_compute_route_round_entity_bbox(ped, local(&mut start), rd::<u32>(this, 0xC), local(&mut end), local(&mut local_route), 0);
    route_add(rd::<GPtr<u8>>(this, 0x4C), start);
    let cnt = local_route[0] as i32;
    for i in 0..cnt.max(0) as usize {
        let q = &local_route[1 + 3 * i..4 + 3 * i];
        route_add(rd::<GPtr<u8>>(this, 0x4C), [q[0], q[1], q[2]]);
    }
    route_add(rd::<GPtr<u8>>(this, 0x4C), end);
}

/// `FUN_0061c9b0(CPed*)` (0x61C9B0, `ret 4`; a `CTaskSimpleFight` helper, called from its `ProcessPed` and
/// `MakeAbortable`): the fight strafe anims 0x29..0x2C. Unless a strafe was requested or is active (+0x26 / +0x24)
/// and the player's move vector (player data +0xC, +0x10) is at least 0.1 long, every one of them blends out at
/// -8.0 and +0x24 / +0x27 / +0x26 are cleared. Otherwise the move vector divided by `|x| + |y|` sets the blend
/// amounts directly (x > 0: 0x2C = x, 0x2A = 0; x < 0: 0x2A = -x, 0x2C = 0; y < 0: 0x29 = -y, 0x2B = 0; y > 0:
/// 0x2B = y, 0x29 = 0; missing anims are added with `AddAnimation`), +0x24 = 1, +0x27 = the old +0x26, +0x26 = 0.
#[hook(addr = 0x61C9B0, conv = thiscall, name = "FUN_0061c9b0", expect = "83 EC 08 53 55 56",
       unverified = "ai8-2", route = "L2", hazards = "heap-writes", recs = "weapon-combat")]
pub fn fight_blend_strafe_anims(this: GPtr<u8>, ped: GPtr<u8>) {
    let clump = || rd::<u32>(ped, 0x18);
    let mut a29 = clump_get_association(clump(), 0x29);
    let mut a2a = clump_get_association(clump(), 0x2A);
    let mut a2b = clump_get_association(clump(), 0x2B);
    let mut a2c = clump_get_association(clump(), 0x2C);
    let mut moving = false;
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if rd::<u8>(this, 0x26) != 0 || rd::<u8>(this, 0x24) != 0 {
        let (mx, my) = (x(rd::<f32>(pd, 0xC)), x(rd::<f32>(pd, 0x10)));
        let len = mx.fmul(mx).fadd(my.fmul(my)).fsqrt();
        moving = len.fcom(x(k(0x858B1C))).test_parity(0x05);
    }
    if !moving {
        for a in [a29, a2a, a2b, a2c] {
            if !a.is_null() {
                wr::<u32>(a, 0x1C, 0xC100_0000);
            }
        }
        wr::<u8>(this, 0x24, 0);
        wr::<u8>(this, 0x27, 0);
        wr::<u8>(this, 0x26, 0);
        return;
    }
    let s = x(rd::<f32>(pd, 0x10)).fabs().fadd(x(rd::<f32>(pd, 0xC)).fabs());
    let inv = s.fdivr_m(k(0x858624));
    let nx = f32_of(inv.fmul_m(rd::<f32>(pd, 0xC)));
    let ny = f32_of(inv.fmul_m(rd::<f32>(pd, 0x10)));
    let zero = x(k(K_ZERO));
    if !x(nx).fcom(zero).test_any(0x41) {
        if !a2a.is_null() {
            wr::<u32>(a2a, 0x18, 0);
        }
        if a2c.is_null() {
            a2c = anim_add_animation(clump(), 0, 0x2C);
        }
        wr::<f32>(a2c, 0x18, nx);
    } else if !x(nx).fcom(zero).test_parity(0x05) {
        if !a2c.is_null() {
            wr::<u32>(a2c, 0x18, 0);
        }
        if a2a.is_null() {
            a2a = anim_add_animation(clump(), 0, 0x2A);
        }
        wr::<f32>(a2a, 0x18, f32_of(x(nx).fchs()));
    }
    if !x(ny).fcom(zero).test_parity(0x05) {
        if !a2b.is_null() {
            wr::<u32>(a2b, 0x18, 0);
        }
        if a29.is_null() {
            a29 = anim_add_animation(clump(), 0, 0x29);
        }
        wr::<f32>(a29, 0x18, f32_of(x(ny).fchs()));
    } else if !x(ny).fcom(zero).test_any(0x41) {
        if !a29.is_null() {
            wr::<u32>(a29, 0x18, 0);
        }
        if a2b.is_null() {
            a2b = anim_add_animation(clump(), 0, 0x2B);
        }
        wr::<f32>(a2b, 0x18, ny);
    }
    let old = rd::<u8>(this, 0x26);
    wr::<u8>(this, 0x24, 1);
    wr::<u8>(this, 0x27, old);
    wr::<u8>(this, 0x26, 0);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GOTO_DOOR_AND_OPEN_CREATE_FIRST_SUB_TASK,
    HOOK_GO_TO_CAR_DOOR_COMPUTE_ROUTE_TO_DOOR,
    HOOK_FIGHT_BLEND_STRAFE_ANIMS,
];
