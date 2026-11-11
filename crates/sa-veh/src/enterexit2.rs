//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CCarEnterExit`. Ported from the 1.0 US bytes;
//! gta-reversed and docs/re/specs/VEH as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::rd;

orig! {
    /// `CVehicle` virtual slot 0xBC (`GetRideAnimData()`)
    pub fn get_ride_anim_data(v: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xBC => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    pub fn blend_animation(clump: GPtr<u8>, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CPed::StopNonPartialAnims()` (0x5DED10)
    pub fn stop_non_partial_anims(ped: GPtr<u8>) = thiscall 0x5DED10 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(const CVehicle*, const CPed*)` (0x64F110)
    pub fn compute_target_door_to_exit(v: GPtr<u8>, ped: GPtr<u8>) -> i32 = cdecl 0x64F110 => unported;
    /// `FUN_004aeac0`: an event constructor (vtable 0x85AFE8, 0x20 bytes: `+0x14`/`+0x18` entity
    /// references, `+0x1C` the door), thiscall with three arguments
    pub fn event_ctor_4aeac0(ev: GPtr<u8>, a: GPtr<u8>, veh: GPtr<u8>, door: i32) -> GPtr<u8> = thiscall 0x4AEAC0 => unported;
    /// `FUN_004aeb90`: its destructor
    pub fn event_dtor_4aeb90(ev: GPtr<u8>) = thiscall 0x4AEB90 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `Invert(const CMatrix& src, CMatrix& dst)` (0x59B920)
    pub fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `CVector::FromMultiply(const CMatrix&, const CVector&)` (0x59C670)
    pub fn vector_from_multiply(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x59C670 => unported;
    /// `CCollision::TestLineSphere(const CColLine&, const CColSphere&)` (0x417470)
    pub fn test_line_sphere(line: GPtr<u8>, sphere: GPtr<u8>) -> u8 = cdecl 0x417470 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    pub fn event_group_add(group: GPtr<u8>, ev: GPtr<u8>, unk: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// 1000.0f, the blend delta of every AddInCarAnim call.
const BLEND_1000: u32 = 0x447A_0000;

/// `CCarEnterExit::AddInCarAnim(const CVehicle*, CPed*, bool asDriver)` (0x64F720): the sitting
/// animation blended on the ped's clump at delta 1000, then (tail jump) `CPed::StopNonPartialAnims`.
/// With ride anim data (vtable +0xBC, called twice): its group, anim 0xC2. Driver: bikes (`+0x590 ==
/// 5`) anim 0x3C when handling model flag 0x400 else 0x51; others 0x3D for `+0x429 & 8` (low car) else
/// 0x3C. Passenger: 0x41 for a low car, else 0x40 (group 0).
#[hook(addr = 0x64F720, conv = cdecl, name = "CCarEnterExit::AddInCarAnim", expect = "8A 44 24 0C 84 C0")]
pub fn add_in_car_anim(veh: GPtr<u8>, ped: GPtr<u8>, as_driver: u32) {
    let (group, id) = if !get_ride_anim_data(veh).is_null() {
        let d = get_ride_anim_data(veh);
        (rd::<u32>(d, 0), 0xC2)
    } else if as_driver as u8 != 0 {
        if rd::<u32>(veh, 0x590) == 5 {
            let h = rd::<GPtr<u8>>(veh, 0x384);
            (
                0,
                if rd::<u32>(h, 0xCC) & 0x400 != 0 {
                    0x3C
                } else {
                    0x51
                },
            )
        } else {
            (0, if rd::<u8>(veh, 0x429) & 8 != 0 { 0x3D } else { 0x3C })
        }
    } else {
        (0, if rd::<u8>(veh, 0x429) & 8 != 0 { 0x41 } else { 0x40 })
    };
    blend_animation(rd::<GPtr<u8>>(ped, 0x18), group, id, BLEND_1000);
    stop_non_partial_anims(ped);
}

/// `CCarEnterExit::MakeUndraggedPassengerPedsLeaveCar(const CVehicle*, const CPed* dragged, const CPed*
/// dragger)` (0x64F540): for every passenger slot below the passenger count (`+0x488`, re-read each
/// step) holding a ped other than `dragged` without flag `+0x470 & 0x800000`: a stack event built by
/// `FUN_004aeac0(dragger, vehicle, ComputeTargetDoorToExit(vehicle, ped))` is added to the ped's
/// intelligence event group (`+0x47C` `+0x68`, `Add(event, false)`) and destroyed.
#[hook(addr = 0x64F540, conv = cdecl, name = "CCarEnterExit::MakeUndraggedPassengerPedsLeaveCar", expect = "64 A1 00 00 00 00")]
pub fn make_undragged_passenger_peds_leave_car(veh: GPtr<u8>, dragged: GPtr<u8>, dragger: GPtr<u8>) {
    let mut i = 0u32;
    while i < u32::from(rd::<u8>(veh, 0x488)) {
        let p = rd::<GPtr<u8>>(veh, 0x464 + 4 * i as isize);
        if !p.is_null() && p != dragged && rd::<u32>(p, 0x470) & 0x80_0000 == 0 {
            let door = compute_target_door_to_exit(veh, p);
            let mut ev = [0u32; 8];
            let evp = GPtr::from_raw(&raw mut ev).cast::<u8>();
            event_ctor_4aeac0(evp, dragger, veh, door);
            let intel = rd::<GPtr<u8>>(p, 0x47C);
            event_group_add(intel.byte_add(0x68), evp, 0);
            event_dtor_4aeb90(evp);
        }
        i += 1;
    }
}

/// `CCarEnterExit::IsPathToDoorBlockedByVehicleCollisionModel(const CPed*, CVehicle*, const CVector&
/// target)` (0x651210): false for model 0x241. Otherwise the ped position and the target are taken into
/// the vehicle's space (`Invert` of its matrix into a stack CMatrix, `FromMultiply`); the line runs
/// from the ped point to (target.x, target.y, ped z); every collision sphere, with its centre z replaced
/// by the ped z, is tested with `TestLineSphere` until one hits. Returns whether one hit. Only AL is
/// defined. The sphere's surface bytes (`+0x10`) are stale stack in the original, zero here.
#[hook(addr = 0x651210, conv = cdecl, name = "CCarEnterExit::IsPathToDoorBlockedByVehicleCollisionModel", expect = "64 A1 00 00 00 00")]
pub fn is_path_to_door_blocked_by_vehicle_collision_model(
    ped: GPtr<u8>,
    veh: GPtr<u8>,
    target: GPtr<u8>,
) -> u8 {
    if rd::<u16>(veh, 0x22) == 0x241 {
        return 0;
    }
    let cd = rd::<GPtr<u8>>(get_col_model(veh), 0x2C);
    let mut m = [0u32; 0x12];
    let mp = GPtr::from_raw(&raw mut m).cast::<u8>();
    matrix_invert(rd::<GPtr<u8>>(veh, 0x14), mp);
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let ppos = if pm.is_null() {
        ped.byte_add(4)
    } else {
        pm.byte_add(0x30)
    };
    let mut a = [0u32; 3];
    let mut b = [0u32; 3];
    vector_from_multiply(GPtr::from_raw(&raw mut a).cast(), mp, ppos);
    vector_from_multiply(GPtr::from_raw(&raw mut b).cast(), mp, target);
    let mut line = [a[0], a[1], a[2], 0, b[0], b[1], a[2], 0];
    let lp = GPtr::from_raw(&raw mut line).cast::<u8>();
    let mut i = 0i32;
    while i < i32::from(rd::<i16>(cd, 0)) {
        let sp = rd::<GPtr<u8>>(cd, 8).byte_add(0x14 * i as isize);
        let mut sphere = [rd::<u32>(sp, 0), rd::<u32>(sp, 4), a[2], 0, 0];
        sphere[3] = crate::common::x(rd::<f32>(sp, 0xC)).to_f32().to_bits();
        if test_line_sphere(lp, GPtr::from_raw(&raw mut sphere).cast()) != 0 {
            break;
        }
        i += 1;
    }
    let hit = i != i32::from(rd::<i16>(cd, 0));
    matrix_dtor(mp);
    u8::from(hit)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_PATH_TO_DOOR_BLOCKED_BY_VEHICLE_COLLISION_MODEL,
    HOOK_ADD_IN_CAR_ANIM,
    HOOK_MAKE_UNDRAGGED_PASSENGER_PEDS_LEAVE_CAR,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
