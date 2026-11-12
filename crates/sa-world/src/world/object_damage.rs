//! WRITER-WORLD-2 (batch `world6-1`, SA:MP-executed): `CObject::ObjectDamage`, the damage, break and
//! smash path of every object hit (explosions, bullets, cars). The entry is `nop; jmp 0x40191E`, a
//! protection chunk holding the body's first instruction (`mov eax, fs:[0]`) that jumps back to
//! 0x5A0D96. Ported from the 1.0 US bytes; gta-reversed as a reading aid. Where they differ the bytes
//! win: "set static" is a direct flag write, the effect types are tested in the bytes' order (3 always,
//! 1 only when not destroyed and above 30 damage, 2 only when destroyed), and the offset-effect
//! position is `stack residue + GetPosition()`: the original adds the position to a stack vector it
//! never wrote (the rotated offset it computes is unused). That residue is modelled as zero.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x};

orig! {
    /// `CPhysical::CanPhysicalBeDamaged(eWeaponType, bool*)` (0x5448B0)
    fn can_physical_be_damaged(e: GPtr<u8>, weapon: u32, out: u32) -> u8 = thiscall 0x5448B0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `BreakManager_c::Add(CObject*, RwV3d* velocity, float velocityRand, bool justFaces)` (0x59E9B0) on `g_breakMan` (0xBB4240)
    fn break_manager_add(man: GPtr<u8>, o: GPtr<u8>, vel: GPtr<u8>, rand: u32, just_faces: u32) -> u8 = thiscall 0x59E9B0 => unported;
    /// `CEntity::GetIsStatic()` (0x4633E0)
    fn get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// `CEntity::DeleteRwObject()` (vtable +0x20)
    fn v_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CObject::TryToExplode()` (0x59F2D0)
    fn try_to_explode(o: GPtr<u8>) -> u8 = thiscall 0x59F2D0 => unported;
    /// `CAudioEngine::ReportObjectDestruction(CEntity*)` (0x506ED0) on `AudioEngine` (0xB6BC90)
    fn report_object_destruction(ae: GPtr<u8>, e: GPtr<u8>) = thiscall 0x506ED0 => unported;
    /// `Fx_c::CreateMatFromVec(RwMatrix*, CVector* origin, CVector* direction)` (0x49E950) on `g_fx` (0xA9AE00)
    fn create_mat_from_vec(fx: GPtr<u8>, m: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>) = thiscall 0x49E950 => unported;
    /// `FxManager_c::CreateFxSystem(FxSystemBP_c*, RwMatrix*, RwMatrix*, bool)` (0x4A95C0) on `g_fxMan` (0xA9AE80)
    fn create_fx_system_mat(man: GPtr<u8>, bp: GPtr<u8>, m: GPtr<u8>, parent: u32, local: u32) -> GPtr<u8> = thiscall 0x4A95C0 => unported;
    /// `FxManager_c::CreateFxSystem(FxSystemBP_c*, CVector*, RwMatrix*, bool)` (0x4A96B0) on `g_fxMan`
    fn create_fx_system_pos(man: GPtr<u8>, bp: GPtr<u8>, pos: GPtr<u8>, parent: u32, local: u32) -> GPtr<u8> = thiscall 0x4A96B0 => unported;
    /// `FxSystem_c::PlayAndKill()` (0x4AA3D0)
    fn play_and_kill(s: GPtr<u8>) = thiscall 0x4AA3D0 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790)
    fn multiply3x3_mv(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vec_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => unported;
}

/// `MI_IMY_SHASH_WALL` (0x8CD748).
const MI_IMY_SHASH_WALL: usize = 0x8C_D748;
/// `150.0` (0x858A28), `30.0` (0x858CA4), `-500.0` (0x85AB78).
const F_150: f32 = 150.0;
const F_30: f32 = 30.0;
const F_M500: f32 = -500.0;

/// A Rust local passed by address to an original callee.
fn p<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// `GetPosition()`.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The smash sequence of effects 20 / 21 / 200 / 202: no collision, invisible (+0x1C bits 0 and 7
/// cleared), off the moving list unless static, static (+0x1C bit 2), explosion proof (+0x40 bit
/// 23), move and turn speed zeroed, `broken` sets +0x140 bit 10, then `DeleteRwObject`.
fn smash(e: GPtr<u8>, broken: bool) {
    wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFF_FF7E);
    if get_is_static(e) == 0 {
        remove_from_moving_list(e);
    }
    wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 4);
    wr::<u32>(e, 0x40, rd::<u32>(e, 0x40) | 0x80_0000);
    for k in 0..6 {
        wr::<u32>(e, 0x44 + 4 * k, 0);
    }
    if broken {
        wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) | 0x400);
    }
    v_delete_rw_object(e);
}

/// `CObject::ObjectDamage(float damage, CVector* fxOrigin, CVector* fxDirection, CEntity* damager,
/// eWeaponType weapon)` (0x5A0D90). Objects without collision are ignored. `WEAPON_UNIDENTIFIED` (55)
/// from a vehicle counts as `WEAPON_RUNOVERBYCAR` (50); `CanPhysicalBeDamaged(weapon, null)` must
/// hold. Health -= `damage * info->colDamageMultiplier`, floored at 0. Then, with a damage effect
/// (+0x144): invulnerable objects (+0x40 bit 22) only take damage from the player or the player's
/// vehicle, `MI_IMY_SHASH_WALL` only from a SWAT van (601, driven by a ped in a vehicle or the van
/// itself), the forklift (530) never. +0x148 = weapon. Above 150 effective damage or at 0 health the
/// effect applies: 1 change model (once; destroyed on the first change), 20 smash, 21 change then
/// smash, 200 / 202 break (`BreakManager_c::Add` with the info's break velocity, just faces above
/// `smashMultiplier * 150`) and smash; an object left without collision and invisible gets 0 health.
/// A destroyed object tries to explode and reports its destruction to the audio engine; exploded:
/// done. Finally the info's particle effect (+0x20 type: 3 always, 1 when not destroyed and damage
/// > 30, 2 when destroyed): at `fxOrigin` along `fxDirection` when the offset x < -500 (none without
/// an origin), else at the position (module docs), played and killed.
#[hook(addr = 0x5A0D90, conv = thiscall, name = "CObject::ObjectDamage", this = "CObject",
       expect = "90 E9 88 0B E6 FF",
       unverified = "world6-1", route = "L2", hazards = "uninit-locals,heap-writes",
       recs = "weapon-combat,driving",
       note = "entry nop; jmp 0x40191E (chunk = mov eax, fs:[0]; jmp 0x5A0D96), relocated whole; the offset-effect position adds the position to an uninitialised stack vector (zero here)")]
pub fn object_damage(this: GPtr<u8>, damage: f32, origin: GPtr<u8>, dir: GPtr<u8>, damager: GPtr<u8>, weapon: u32) {
    if rd::<u8>(this, 0x1C) & 1 == 0 {
        return;
    }
    let mut weapon = weapon;
    if weapon == 0x37 && !damager.is_null() && rd::<u8>(damager, 0x36) & 7 == 2 {
        weapon = 0x32;
    }
    if can_physical_be_damaged(this, weapon, 0) == 0 {
        return;
    }
    let info = rd::<GPtr<u8>>(this, 0x160);
    let h = x(damage).fmul_m(rd::<f32>(info, 0x18)).fsubr_m(rd::<f32>(this, 0x154));
    wr::<f32>(this, 0x154, h.to_f32());
    if !h.fcom(x(0.0)).test_parity(0x05) {
        wr::<u32>(this, 0x154, 0);
    }
    if rd::<u8>(this, 0x144) == 0 {
        return;
    }
    if rd::<u32>(this, 0x40) & 0x40_0000 != 0
        && damager != find_player_ped(-1)
        && damager != find_player_vehicle(-1, 0)
    {
        return;
    }
    if i32::from(rd::<i16>(this, 0x22)) == i32::from(g::<u16>(MI_IMY_SHASH_WALL).read()) {
        if damager.is_null() {
            return;
        }
        let van = match rd::<u8>(damager, 0x36) & 7 {
            3 => {
                if rd::<u32>(damager, 0x46C) & 0x100 == 0 {
                    return;
                }
                rd::<GPtr<u8>>(damager, 0x58C)
            }
            2 => damager,
            _ => return,
        };
        if van.is_null() || rd::<u16>(van, 0x22) != 0x259 {
            return;
        }
    }
    if !damager.is_null() && rd::<u16>(damager, 0x22) == 0x212 {
        return;
    }
    wr::<u8>(this, 0x148, weapon as u8);
    let mut destroyed = false;
    let info = rd::<GPtr<u8>>(this, 0x160);
    let big = !x(damage).fmul_m(rd::<f32>(info, 0x18)).fcom(x(F_150)).test_any(0x41);
    if big || !x(rd::<f32>(this, 0x154)).fcom(x(0.0)).test_parity(0x44) {
        match rd::<u8>(this, 0x144) {
            1 => {
                if rd::<u32>(this, 0x1C) & 0x200 == 0 {
                    destroyed = true;
                    v_delete_rw_object(this);
                }
                wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x200);
            }
            20 => {
                smash(this, false);
                destroyed = true;
            }
            21 => {
                if rd::<u32>(this, 0x1C) & 0x200 != 0 {
                    smash(this, false);
                    destroyed = true;
                } else {
                    v_delete_rw_object(this);
                    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x200);
                }
            }
            200 | 202 => {
                let mut vel = [rd::<u32>(info, 0x38), rd::<u32>(info, 0x3C), rd::<u32>(info, 0x40)];
                let dm = x(damage).fmul_m(rd::<f32>(info, 0x18));
                let sm = x(rd::<f32>(info, 0x34)).fmul_m(F_150);
                let just_faces = u32::from(!sm.fcom(dm).test_parity(0x05));
                break_manager_add(g(0xBB_4240), this, p(&mut vel), rd::<u32>(info, 0x44), just_faces);
                smash(this, true);
                destroyed = true;
            }
            _ => {}
        }
        if rd::<u8>(this, 0x1C) & 0x81 == 0 {
            wr::<u32>(this, 0x154, 0);
        }
    }
    if destroyed {
        let exploded = try_to_explode(this) != 0;
        report_object_destruction(g(0xB6_BC90), this);
        if exploded {
            return;
        }
    }
    let info = rd::<GPtr<u8>>(this, 0x160);
    let fx = rd::<u8>(info, 0x20);
    if fx == 0 {
        return;
    }
    let play = if fx == 3 {
        true
    } else if !destroyed {
        fx == 1 && !x(damage).fcom(x(F_30)).test_any(0x41)
    } else {
        fx == 2
    };
    if !play {
        return;
    }
    let sys = if !x(rd::<f32>(info, 0x24)).fcom(x(F_M500)).test_parity(0x05) {
        if origin.is_null() {
            return;
        }
        let mut mat = [0u32; 16];
        create_mat_from_vec(g(0xA9_AE00), p(&mut mat), origin, dir);
        create_fx_system_mat(g(0xA9_AE80), rd::<GPtr<u8>>(info, 0x30), p(&mut mat), 0, 0)
    } else {
        let mut m = [0u32; 0x12];
        matrix_copy_ctor(p(&mut m), rd::<GPtr<u8>>(this, 0x14));
        let info = rd::<GPtr<u8>>(this, 0x160);
        let mut off = [rd::<u32>(info, 0x24), rd::<u32>(info, 0x28), rd::<u32>(info, 0x2C)];
        let mut out = [0u32; 3];
        multiply3x3_mv(p(&mut out), p(&mut m), p(&mut off));
        // the rotated offset (stored back into `off` by the original) is not used
        let mut v = [0f32; 3]; // uninitialised in the original
        vec_add_assign(p(&mut v), ent_pos(this));
        let s = create_fx_system_pos(g(0xA9_AE80), rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x160), 0x30), p(&mut v), 0, 0);
        matrix_dtor(p(&mut m));
        s
    };
    if !sys.is_null() {
        play_and_kill(sys);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_OBJECT_DAMAGE];

