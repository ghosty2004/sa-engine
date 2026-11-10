//! Field access, constants, globals and the original callees shared by the PHYS modules.
//!
//! `CPhysical` objects are reached through a byte pointer ([`Ent`]) and field offsets (the original
//! layout, gta-reversed `Entity/Physical.h`, offsets checked against the bytes; the derived classes'
//! fields some functions touch — `CVehicle` +0x45C, `CPed` +0x568, ... — are named where used). The
//! accessors read and write exactly the bytes the original's `mov`/`fld`/`fstp` do.

use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::vector::CVector;

/// A `CEntity*` / `CPhysical*` (any subclass).
pub type Ent = GPtr<u8>;

/// f32 at `p + off`
#[inline]
pub fn rf(p: Ent, off: isize) -> f32 {
    p.byte_add(off).cast::<f32>().read()
}
/// store f32 at `p + off`
#[inline]
pub fn wf(p: Ent, off: isize, v: f32) {
    p.byte_add(off).cast::<f32>().write(v)
}
/// dword at `p + off`
#[inline]
pub fn ru(p: Ent, off: isize) -> u32 {
    p.byte_add(off).cast::<u32>().read()
}
/// store dword
#[inline]
pub fn wu(p: Ent, off: isize, v: u32) {
    p.byte_add(off).cast::<u32>().write(v)
}
/// byte at `p + off`
#[inline]
pub fn rb(p: Ent, off: isize) -> u8 {
    p.byte_add(off).read()
}
/// store byte
#[inline]
pub fn wb(p: Ent, off: isize, v: u8) {
    p.byte_add(off).write(v)
}
/// word at `p + off`
#[inline]
pub fn rh(p: Ent, off: isize) -> u16 {
    p.byte_add(off).cast::<u16>().read()
}
/// store word
#[inline]
pub fn wh(p: Ent, off: isize, v: u16) {
    p.byte_add(off).cast::<u16>().write(v)
}
/// pointer at `p + off`
#[inline]
pub fn rp(p: Ent, off: isize) -> Ent {
    p.byte_add(off).cast::<GPtr<u8>>().read()
}
/// store pointer
#[inline]
pub fn wp(p: Ent, off: isize, v: Ent) {
    p.byte_add(off).cast::<GPtr<u8>>().write(v)
}
/// Three f32 words at `p + off` as raw bits (copied with integer moves in the original).
#[inline]
pub fn rv_bits(p: Ent, off: isize) -> [u32; 3] {
    [ru(p, off), ru(p, off + 4), ru(p, off + 8)]
}
/// Stores three raw words.
#[inline]
pub fn wv_bits(p: Ent, off: isize, v: [u32; 3]) {
    wu(p, off, v[0]);
    wu(p, off + 4, v[1]);
    wu(p, off + 8, v[2]);
}
/// Three f32 words at `p + off`.
#[inline]
pub fn rv(p: Ent, off: isize) -> [f32; 3] {
    [rf(p, off), rf(p, off + 4), rf(p, off + 8)]
}
/// Stores three f32 words (x, y, z order).
#[inline]
pub fn wv(p: Ent, off: isize, v: [f32; 3]) {
    wf(p, off, v[0]);
    wf(p, off + 4, v[1]);
    wf(p, off + 8, v[2]);
}

/// A `CVector` from three f32 words (bit copies).
#[inline]
pub fn cv(v: [f32; 3]) -> CVector {
    CVector::new(v[0], v[1], v[2])
}
/// The words of a `CVector`.
#[inline]
pub fn va(v: CVector) -> [f32; 3] {
    [v.x, v.y, v.z]
}

/// A pointer to a Rust local (phase A: process memory the original callees can use).
#[inline]
pub fn lp<T>(v: &mut T) -> GPtr<T> {
    GPtr::from_raw(v as *mut T)
}

/// `f32` from `.rdata` bits.
#[inline]
pub fn cf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

// ---- CEntity / CPhysical field offsets -------------------------------------------------------

/// `CPlaceable::m_placement.m_vPosn` (used when `m_matrix` is null)
pub const PLACEMENT_POS: isize = 0x4;
/// `CPlaceable::m_matrix` (`CMatrixLink*`)
pub const MATRIX: isize = 0x14;
/// `CEntity::m_pRwObject`
pub const RW_OBJECT: isize = 0x18;
/// `CEntity` flags dword (bit 0 `bUsesCollision`, 0x100 `m_bIsBIGBuilding`, ...)
pub const EFLAGS: isize = 0x1C;
/// `CEntity::m_nModelIndex` (int16)
pub const MODEL_INDEX: isize = 0x22;
/// `CEntity::m_nType` (low 3 bits) / `m_nStatus` (bits 3..7)
pub const TYPE: isize = 0x36;
/// `CPhysical::m_nLastCollisionTime`
pub const LAST_COLLISION_TIME: isize = 0x3C;
/// `CPhysical::m_nPhysicalFlags`
pub const PFLAGS: isize = 0x40;
pub const MOVE_SPEED: isize = 0x44;
pub const TURN_SPEED: isize = 0x50;
pub const FRICTION_MOVE: isize = 0x5C;
pub const FRICTION_TURN: isize = 0x68;
pub const FORCE: isize = 0x74;
pub const TORQUE: isize = 0x80;
pub const MASS: isize = 0x8C;
pub const TURN_MASS: isize = 0x90;
pub const VELOCITY_FREQ: isize = 0x94;
pub const AIR_RESISTANCE: isize = 0x98;
pub const ELASTICITY: isize = 0x9C;
pub const BUOYANCY: isize = 0xA0;
pub const CENTRE_OF_MASS: isize = 0xA4;
/// `CEntryInfoList` head (`CEntryInfoNode*`)
pub const COLLISION_LIST: isize = 0xB0;
/// `CPtrNodeDoubleLink*` in `CWorld::ms_listMovingEntityPtrs`
pub const MOVING_LIST: isize = 0xB4;
pub const FAKE_PHYSICS: isize = 0xB8;
pub const NUM_COLLIDED: isize = 0xB9;
pub const CONTACT_SURFACE: isize = 0xBA;
/// `m_apCollidedEntities[6]`
pub const COLLIDED: isize = 0xBC;
pub const MOVING_SPEED: isize = 0xD4;
pub const DAMAGE_INTENSITY: isize = 0xD8;
pub const DAMAGE_ENTITY: isize = 0xDC;
pub const LAST_IMPACT_VELOCITY: isize = 0xE0;
pub const LAST_IMPACT_POS: isize = 0xEC;
pub const PIECE_TYPE: isize = 0xF8;
pub const ATTACHED_TO: isize = 0xFC;
pub const ATTACH_OFFSET: isize = 0x100;
pub const ATTACH_ROTATION: isize = 0x10C;
pub const ATTACH_QUAT: isize = 0x118;
pub const IGNORED_COLLISION: isize = 0x128;
pub const SURFACE_BRIGHTNESS: isize = 0x12C;
pub const DYNAMIC_LIGHTING: isize = 0x130;
pub const SHADOW_DATA: isize = 0x134;

/// Entity type values (`m_nType`, low 3 bits of +0x36).
pub mod ty {
    pub const BUILDING: u8 = 1;
    pub const VEHICLE: u8 = 2;
    pub const PED: u8 = 3;
    pub const OBJECT: u8 = 4;
}

/// `m_nType` of `e`.
#[inline]
pub fn etype(e: Ent) -> u8 {
    rb(e, TYPE) & 7
}
/// `m_matrix` of `e`.
#[inline]
pub fn matrix(e: Ent) -> Ent {
    rp(e, MATRIX)
}
/// `CPhysical::m_nPhysicalFlags`
#[inline]
pub fn pflags(e: Ent) -> u32 {
    ru(e, PFLAGS)
}
/// `GetPosition()`: `&m_matrix->pos` (+0x30) or `&m_placement.pos` (+4) when there is no matrix.
#[inline]
pub fn pos_ptr(e: Ent) -> Ent {
    let m = matrix(e);
    if m.is_null() { e.byte_add(PLACEMENT_POS) } else { m.byte_add(0x30) }
}

// ---- `.rdata` constants (f32 bits; the address in the name) ---------------------------------

/// `.rdata` f32 constants used by the PHYS code. The name is the address.
pub mod k {
    /// 1.0
    pub const K_858624: u32 = 0x3F80_0000;
    /// 200.0
    pub const K_858A48: u32 = 0x4348_0000;
    /// 0.1
    pub const K_858B1C: u32 = 0x3DCC_CCCD;
    /// 60.0
    pub const K_858B34: u32 = 0x4270_0000;
    /// 0.02
    pub const K_858B38: u32 = 0x3CA3_D70A;
    /// 3.0
    pub const K_858B3C: u32 = 0x4040_0000;
    /// 0.005
    pub const K_858B4C: u32 = 0x3BA3_D70A;
    /// 0.0
    pub const K_858B50: u32 = 0x0000_0000;
    /// 0.5
    pub const K_858B8C: u32 = 0x3F00_0000;
    /// 4.0
    pub const K_858B90: u32 = 0x4080_0000;
    /// 20.0
    pub const K_858BA4: u32 = 0x41A0_0000;
    /// 120.0
    pub const K_858BB0: u32 = 0x42F0_0000;
    /// 1e-5
    pub const K_858C14: u32 = 0x3727_C5AC;
    /// -1.0
    pub const K_858C1C: u32 = 0xBF80_0000;
    /// 0.9
    pub const K_858C20: u32 = 0x3F66_6666;
    /// 0.3
    pub const K_858C24: u32 = 0x3E99_999A;
    /// 0.05
    pub const K_858C28: u32 = 0x3D4C_CCCD;
    /// 1000.0
    pub const K_858C4C: u32 = 0x447A_0000;
    /// 0.01
    pub const K_858C58: u32 = 0x3C23_D70A;
    /// 1/32767 (3.0518509e-5)
    pub const K_858C7C: u32 = 0x3800_0100;
    /// 0.25
    pub const K_858C84: u32 = 0x3E80_0000;
    /// 0.8
    pub const K_858C98: u32 = 0x3F4C_CCCD;
    /// 2.0
    pub const K_858CA0: u32 = 0x4000_0000;
    /// -0.9
    pub const K_858CAC: u32 = 0xBF66_6666;
    /// 0.7
    pub const K_858CB0: u32 = 0x3F33_3333;
    /// pi
    pub const K_858CB8: u32 = 0x4049_0FDB;
    /// 2 pi
    pub const K_858CBC: u32 = 0x40C9_0FDB;
    /// -pi
    pub const K_858CC0: u32 = 0xC049_0FDB;
    /// 0.2
    pub const K_858CC4: u32 = 0x3E4C_CCCD;
    /// 0.6
    pub const K_858CC8: u32 = 0x3F19_999A;
    /// 0.0025
    pub const K_858CD8: u32 = 0x3B23_D70B;
    /// 0.001
    pub const K_858CDC: u32 = 0x3A83_126F;
    /// -0.3
    pub const K_858EE4: u32 = 0xBE99_999A;
    /// 0.4
    pub const K_858EE8: u32 = 0x3ECC_CCCD;
    /// 1.2
    pub const K_858F08: u32 = 0x3F99_999A;
    /// 1.1
    pub const K_858F14: u32 = 0x3F8C_CCCD;
    /// 0.75
    pub const K_858F34: u32 = 0x3F40_0000;
    /// -0.5
    pub const K_858F40: u32 = 0xBF00_0000;
    /// 1.25
    pub const K_8595F0: u32 = 0x3FA0_0000;
    /// 1.3
    pub const K_859918: u32 = 0x3FA6_6666;
    /// -1000.0
    pub const K_859948: u32 = 0xC47A_0000;
    /// 0.0005
    pub const K_859978: u32 = 0x3A03_126F;
    /// 1.4
    pub const K_859994: u32 = 0x3FB3_3333;
    /// 0.0002
    pub const K_85A7D0: u32 = 0x3951_B717;
    /// 180.0
    pub const K_85A994: u32 = 0x4334_0000;
    /// 0.66
    pub const K_85BA50: u32 = 0x3F28_F5C3;
    /// 9.0
    pub const K_85EED4: u32 = 0x4110_0000;
    /// 0.99
    pub const K_862CD0: u32 = 0x3F7D_70A4;
    /// 0.85
    pub const K_862E1C: u32 = 0x3F59_999A;
    /// 2000.0
    pub const K_8631F0: u32 = 0x44FA_0000;
    /// 12.5
    pub const K_86321C: u32 = 0x4148_0000;
    /// 1.15
    pub const K_863264: u32 = 0x3F93_3333;
    /// 0.008
    pub const K_863984: u32 = 0x3C03_126F;
    /// 1.0
    pub const K_863B58: u32 = 0x3F80_0000;
    /// 0.001
    pub const K_863B5C: u32 = 0x3A83_126F;
    /// 10.0
    pub const K_863B60: u32 = 0x4120_0000;
    /// 0.06
    pub const K_863B64: u32 = 0x3D75_C28F;
    /// 0.98
    pub const K_863B68: u32 = 0x3F7A_E148;
    /// 0.03
    pub const K_863B6C: u32 = 0x3CF5_C28F;
    /// 1.3
    pub const K_863B70: u32 = 0x3FA6_6666;
    /// -0.98
    pub const K_863B74: u32 = 0xBF7A_E148;
    /// 1.4
    pub const K_863B78: u32 = 0x3FB3_3333;
    /// -0.95
    pub const K_863B7C: u32 = 0xBF73_3333;
    /// 1.7
    pub const K_863B80: u32 = 0x3FD9_999A;
    /// -0.95
    pub const K_863B84: u32 = 0xBF73_3333;
    /// 1.5
    pub const K_863B88: u32 = 0x3FC0_0000;
    /// -0.95
    pub const K_863B8C: u32 = 0xBF73_3333;
    /// 500.0
    pub const K_863B90: u32 = 0x43FA_0000;
    /// 0.3
    pub const K_863B94: u32 = 0x3E99_999A;
    /// 0.1
    pub const K_863B98: u32 = 0x3DCC_CCCD;
    /// -0.008
    pub const K_863BFC: u32 = 0xBC03_126F;
    /// 0.016
    pub const K_863C00: u32 = 0x3C83_126F;
    /// -0.53
    pub const K_863C04: u32 = 0xBF07_AE14;
    /// 99998.0
    pub const K_863C08: u32 = 0x47C3_4F00;
    /// 1/765 (0.0013071896)
    pub const K_863C0C: u32 = 0x3AAB_5601;
    /// 0.333333
    pub const K_863C10: u32 = 0x3EAA_AA9F;
    /// 0.8667
    pub const K_863C14: u32 = 0x3F5D_E00D;
    /// 0.87
    pub const K_863C18: u32 = 0x3F5E_B852;
    /// 0.707
    pub const K_863C1C: u32 = 0x3F34_FDF4;
    /// 9999.0
    pub const K_863C20: u32 = 0x461C_3C00;
    /// 1.05
    pub const K_87124C: u32 = 0x3F86_6666;
    /// -1.1
    pub const K_871654: u32 = 0xBF8C_CCCD;
    /// -0.6
    pub const K_871658: u32 = 0xBF19_999A;
    /// 0.82
    pub const K_87165C: u32 = 0x3F51_EB85;
    /// 0.83
    pub const K_871660: u32 = 0x3F54_7AE1;
    /// 0.5263158
    pub const K_871664: u32 = 0x3F06_BCA2;
}

// ---- `.data` globals ---------------------------------------------------------------------------

sa_core::globals! {
    list GLOBALS;
    /// `CWorld::ms_listMovingEntityPtrs` (head `CPtrNodeDoubleLink*`)
    pub static MOVING_LIST_HEAD @ 0xB9ACC8: GPtr<u8> = GPtr::NULL;
    /// `CPhysical::HIGHSPEED_ELASTICITY_MULT_COPCAR`
    pub static HIGHSPEED_ELASTICITY_MULT_COPCAR @ 0x8CD784: f32 = 2.0;
    /// `CPhysical::PHYSICAL_SHIFT_SPEED_DAMP`
    pub static PHYSICAL_SHIFT_SPEED_DAMP @ 0x8CD788: f32 = 0.707;
    /// `CPhysical::SOFTCOL_DEPTH_MIN`
    pub static SOFTCOL_DEPTH_MIN @ 0x8CD78C: f32 = 0.5;
    /// `CPhysical::SOFTCOL_DEPTH_MULT`
    pub static SOFTCOL_DEPTH_MULT @ 0x8CD790: f32 = 2.0;
    /// `CPhysical::SOFTCOL_SPEED_MULT`
    pub static SOFTCOL_SPEED_MULT @ 0x8CD794: f32 = 0.95;
    /// `CPhysical::SOFTCOL_SPEED_MULT2`
    pub static SOFTCOL_SPEED_MULT2 @ 0x8CD798: f32 = 1.05;
    /// `CPhysical::SOFTCOL_CARLINE_SPEED_MULT`
    pub static SOFTCOL_CARLINE_SPEED_MULT @ 0x8CD79C: f32 = 0.4;
    /// `CPhysical::DAMPING_LIMIT_IN_FRAME`
    pub static DAMPING_LIMIT_IN_FRAME @ 0x8CD7A0: f32 = 0.25;
    /// `CPhysical::DAMPING_LIMIT_OF_SPRING_FORCE`
    pub static DAMPING_LIMIT_OF_SPRING_FORCE @ 0x8CD7A4: f32 = 0.999;
    /// 0x8CD7A8 (150.0)
    pub static G_8CD7A8 @ 0x8CD7A8: f32 = 150.0;
    /// 0x8CD7AC (1.0)
    pub static G_8CD7AC @ 0x8CD7AC: f32 = 1.0;
    /// 0x8CD7B0 (0.02)
    pub static G_8CD7B0 @ 0x8CD7B0: f32 = 0.02;
    /// 0x8CD7B4 (0.01)
    pub static G_8CD7B4 @ 0x8CD7B4: f32 = 0.01;
    /// `CPhysical::TEST_ADD_AMBIENT_LIGHT_FRAC`
    pub static TEST_ADD_AMBIENT_LIGHT_FRAC @ 0x8CD7B8: f32 = 0.5;
    /// 0x8CD7C4 (0.05)
    pub static G_8CD7C4 @ 0x8CD7C4: f32 = 0.05;
    /// 0x8CD7C8 (0.707)
    pub static G_8CD7C8 @ 0x8CD7C8: f32 = 0.707;
    /// 0x8CD7CC (0.2)
    pub static G_8CD7CC @ 0x8CD7CC: f32 = 0.2;
    /// 0x8CD7D0 (1.5; ProcessShiftSectorList's vehicle test)
    pub static G_8CD7D0 @ 0x8CD7D0: f32 = 1.5;
    /// 0x8CD7D4 (1.5; ProcessShiftSectorList's shift multiplier)
    pub static G_8CD7D4 @ 0x8CD7D4: f32 = 1.5;
    /// 0x8CD7D8 (0.75)
    pub static G_8CD7D8 @ 0x8CD7D8: f32 = 0.75;
    /// 0x8CD7DC (1e-5)
    pub static G_8CD7DC @ 0x8CD7DC: f32 = 0.00001;
    /// 0x8CD7E0 (0.5)
    pub static G_8CD7E0 @ 0x8CD7E0: f32 = 0.5;
    /// 0x8CD7E4 (-0.2)
    pub static G_8CD7E4 @ 0x8CD7E4: f32 = -0.2;
    /// 0x8CD7E8 (1.8849558)
    pub static G_8CD7E8 @ 0x8CD7E8: f32 = 1.884_955_8;
    /// 0x8CD7EC (0.5)
    pub static G_8CD7EC @ 0x8CD7EC: f32 = 0.5;
    /// 0x8CD7F0 (0.01)
    pub static G_8CD7F0 @ 0x8CD7F0: f32 = 0.01;
    /// 0x8CD7F4 (0.05)
    pub static G_8CD7F4 @ 0x8CD7F4: f32 = 0.05;
    /// 0x8CD7F8 (0.2)
    pub static G_8CD7F8 @ 0x8CD7F8: f32 = 0.2;
    /// 0x8CD7FC (4.0)
    pub static G_8CD7FC @ 0x8CD7FC: f32 = 4.0;
}

// ---- original callees shared by several modules ----------------------------------------------

orig! {
    /// `Multiply3x3(CVector* out, const CMatrix& m, const CVector& v)` (0x59C790, cdecl, returns out)
    pub fn multiply3x3(out: GPtr<CVector>, m: Ent, v: GPtr<CVector>) -> GPtr<CVector>
        = cdecl 0x59C790 => multiply3x3_port;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    pub fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector>
        = cdecl 0x59C730 => crate::collision::data::cross_product_port;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, thiscall)
    pub fn clean_up_old_reference(e: Ent, r: GPtr<Ent>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, thiscall)
    pub fn register_reference(e: Ent, r: GPtr<Ent>) = thiscall 0x571B70 => unported;
    /// `CEntity::GetColModel()` (0x535300, thiscall)
    pub fn get_col_model(e: Ent) -> Ent = thiscall 0x535300 => unported;
    /// CRT `rand()` (0x821B1E)
    pub fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
}

/// Phases B/C side of `Multiply3x3` 0x59C790 (sa-math's port of the same sum order).
pub fn multiply3x3_port(out: GPtr<CVector>, m: Ent, v: GPtr<CVector>) -> GPtr<CVector> {
    let r = crate::collision::prim::vh::mul3x3(m.cast(), rv(v.cast(), 0));
    wv(out.cast(), 0, r);
    out
}

/// `_CIpow` (0x822130): `st1 ^ st0` as the CRT computes it (an x87 stack intrinsic, called through
/// [`sa_core::orig::x87_binary`] with the register values, exact as doubles); unported in phases B/C.
pub fn ci_pow(base: sa_math::x87::X87, exp: sa_math::x87::X87) -> sa_math::x87::X87 {
    let r = sa_core::orig::x87_binary(0x822130, base.to_f64(), exp.to_f64());
    sa_math::x87::X87::from_f64_exact(r)
}
