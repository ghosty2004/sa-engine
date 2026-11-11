//! PORT-AUDIO-3 hand ports, helper B: ped / ped speech / pedless speech / collision / weapon / global weapon / weather / water-cannon / twin-loop entity bodies (see `docs/arch/audio-port.md`, PORT-AUDIO-3).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids (the exe wins).
//!
//! Conventions: every stack argument word is passed through as `u32` (byte / int16 arguments keep the
//! caller's upper bytes where the original pushes the whole slot); x87 values through [`X87`] in the
//! original's operation order; floats are rounded where the original stores them (`fstp dword`).

#[allow(unused_imports)]
use sa_core::abi::HookReg;
#[allow(unused_imports)]
use sa_core::{hook, orig};

#[allow(unused_imports)]
use crate::common::{GPtr, X87, g, rd, wr, x};
use sa_world::world::common::{f32_of, xi};

sa_core::globals! {
    list GLOBALS;
}

/// `AEAudioHardware`, `AESoundManager`, `g_surfaceInfos`
const HW: usize = 0xB5F8B8;
const SM: usize = 0xB62CB0;
const SURFACE_INFOS: usize = 0xB79538;
/// Pointer whose byte +0x74 (int8) is the audio volume offset the collision sounds add (`[0xBD00F8]+0x74`).
const VOLUME_OFFSET_PTR: usize = 0xBD00F8;
/// `CAECollisionAudioEntity` surface table: 16-byte rows at 0x8AD908 (+8 a level, +0xC a volume factor).
const SURFACE_ROWS: usize = 0x8AD908;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// An f32 constant of the exe (`.rdata`).
fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `v * v` (`fld st(k); fmul st(k+1)`: both register operands).
fn sq(v: X87) -> X87 {
    v.fmul(v)
}

/// The surface row's int32 at `+off` (0x8AD908 + 16 * surface).
fn surface_row_i32(surface: u8, off: usize) -> i32 {
    g::<i32>(SURFACE_ROWS + ((surface as usize) << 4) + off).read()
}

/// The volume offset byte (`movsx [[0xBD00F8] + 0x74]`).
fn volume_offset() -> i32 {
    let p = g::<u32>(VOLUME_OFFSET_PTR).read();
    rd::<i8>(g(p as usize), 0x74) as i32
}

orig! {
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bankId, int16 slot)` (0x4D88C0)
    fn hw_is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAESound::Initialise(...)` (0x4EFE50, 14 argument dwords)
    fn sound_initialise(sound: GPtr<u8>, slot: u32, id: u32, entity: u32, px: u32, py: u32, pz: u32, volume: u32, max_distance: u32, speed: u32, time_scale: u32, ignored_cycles: u32, flags: u32, speed_variance: u32, play_pos: u32) = thiscall 0x4EFE50 => crate::sound::initialise;
    /// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10)
    fn sm_request_new_sound(sm: GPtr<u8>, sound: GPtr<u8>) -> u32 = thiscall 0x4EFB10 => unported;
    /// `CAESound::~CAESound()` (0x4EF660)
    fn sound_destructor(sound: GPtr<u8>) = thiscall 0x4EF660 => unported;
    /// `CAESound::SetPosition(CVector)` (0x4EF880, the vector by value)
    fn sound_set_position(sound: GPtr<u8>, px: u32, py: u32, pz: u32) = thiscall 0x4EF880 => unported;
    /// `SurfaceInfos_c::GetFrictionEffect(uint32 surface)` (0x55E650)
    fn surface_get_friction_effect(infos: GPtr<u8>, surface: u32) -> u32 = thiscall 0x55E650 => unported;
    /// `SurfaceInfos_c::IsAudioGrass / IsAudioWater / IsAudioMetal(uint32 surface)` (AL)
    fn surface_is_audio_grass(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EA50 => unported;
    fn surface_is_audio_water(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EAD0 => unported;
    fn surface_is_audio_metal(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EAF0 => unported;
    /// `CEntity::GetIsTypePhysical()` (0x4DA030, AL)
    fn entity_get_is_type_physical(e: GPtr<u8>) -> u8 = thiscall 0x4DA030 => unported;
    /// `CAECollisionAudioEntity::ChooseCollisionSoundID(uint8 surface)` (0x4DAA50, EAX)
    fn collision_choose_collision_sound_id(this: GPtr<u8>, surface: u32) -> u32 = thiscall 0x4DAA50 => crate::entities::collision_choose_collision_sound_id;
    /// `CAECollisionAudioEntity::AddCollisionSoundToList(CEntity*, CEntity*, uint8, uint8, CAESound*, int32)` (0x4DAAC0)
    fn collision_add_collision_sound_to_list(this: GPtr<u8>, e1: u32, e2: u32, s1: u32, s2: u32, sound: u32, status: u32) = thiscall 0x4DAAC0 => crate::entities::collision_add_collision_sound_to_list;
    /// `CAECollisionAudioEntity::GetCollisionSoundStatus(CEntity*, CEntity*, uint8, uint8, int32* index)` (0x4DA830, AL)
    fn collision_get_collision_sound_status(this: GPtr<u8>, e1: u32, e2: u32, s1: u32, s2: u32, out: GPtr<u8>) -> u8 = thiscall 0x4DA830 => crate::port2_b::collision_get_collision_sound_status;
    /// `CAECollisionAudioEntity::PlayOneShotCollisionSound` (0x4DB150, `ret 0x18`)
    fn collision_play_one_shot_call(this: GPtr<u8>, e1: u32, e2: u32, s1: u32, s2: u32, impulse: u32, posn: u32) = thiscall 0x4DB150 => collision_play_one_shot;
    /// `CAECollisionAudioEntity::PlayLoopingCollisionSound` (0x4DB450, `ret 0x1C`)
    fn collision_play_looping_call(this: GPtr<u8>, e1: u32, e2: u32, s1: u32, s2: u32, impulse: u32, posn: u32, flag: u32) = thiscall 0x4DB450 => collision_play_looping;
    /// `CAECollisionAudioEntity::UpdateLoopingCollisionSound` (0x4DA540, `ret 0x20`)
    fn collision_update_looping_call(this: GPtr<u8>, sound: u32, e1: u32, e2: u32, s1: u32, s2: u32, impulse: u32, posn: u32, flag: u32) = thiscall 0x4DA540 => collision_update_looping;
}

// ---- CAECollisionAudioEntity: looping / one-shot collision sounds ---------------------------------

/// Which speed a looping collision sound uses (identical branch structure in 0x4DA540 and 0x4DB450):
/// both surfaces 0x3F (vehicle on vehicle) -> the relative speed of the two entities; exactly one 0x3F
/// and the other not 0x3E -> the first entity's own speed (also when it is the second surface that is
/// 0x3F); otherwise none.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoopSpeed {
    None,
    Relative,
    Own,
}

fn loop_speed_kind(s1: u8, s2: u8) -> LoopSpeed {
    if s1 == 0x3F {
        if s2 == 0x3F {
            LoopSpeed::Relative
        } else if s2 == 0x3E {
            LoopSpeed::None
        } else {
            LoopSpeed::Own
        }
    } else if s2 != 0x3F || s1 == 0x3E {
        LoopSpeed::None
    } else {
        LoopSpeed::Own
    }
}

/// The original's choice between two speed vectors and the larger's magnitude. `s` is the x87 stack
/// after the first `fstp dword` (st0..st4), `m` the stored dword (reloaded as a memory operand):
/// `A = (s2² + s3²) + s4²`, `B = (m² + s0²) + s1²`; `fcompp; test ah, 5; jp` -> `B >= A` (or
/// unordered) picks `t`, else `nt`; the result is `sqrt((p0² + p1²) + p2²)` with p0 the stack top.
fn pick_magnitude(s: [X87; 5], m: f32, nt: [X87; 3], t: [X87; 3]) -> X87 {
    let a = sq(s[2]).fadd(sq(s[3])).fadd(sq(s[4]));
    let b = x(m).fmul_m(m).fadd(sq(s[0])).fadd(sq(s[1]));
    let p = if b.fcom(a).test_parity(0x05) { t } else { nt };
    sq(p[0]).fadd(sq(p[1])).fadd(sq(p[2])).fsqrt()
}

/// `fld [a+off]; fsub [b+off]`
fn dsub(a: GPtr<u8>, b: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(a, off)).fsub_m(rd::<f32>(b, off))
}

/// `fld [e+off]`
fn fl(e: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(e, off))
}

/// The "own speed" magnitude of entity `e` (both looping functions, same bytes): loads +0x4C, +0x48,
/// +0x44, +0x58, +0x54 and the dword +0x50 through a stack slot; move speed (+0x44) or turn speed (+0x50).
fn own_speed(e: GPtr<u8>) -> X87 {
    let s = [fl(e, 0x54), fl(e, 0x58), fl(e, 0x44), fl(e, 0x48), fl(e, 0x4C)];
    let nt = [fl(e, 0x44), fl(e, 0x48), fl(e, 0x4C)];
    let t = [fl(e, 0x50), fl(e, 0x54), fl(e, 0x58)];
    pick_magnitude(s, rd::<f32>(e, 0x50), nt, t)
}

/// `fcom [c]; fnstsw; test ah, 5; jnp keep` else `fstp st0; fld [c]`: `v` unless `v >= c` or unordered.
fn clamp_below(v: X87, k: f32) -> X87 {
    if v.fcom(x(k)).test_parity(0x05) { x(k) } else { v }
}

/// The volume / speed tail shared by the two looping functions, from the clamped speed term `v`:
/// `w = min(v * [0x85F09C], [0x858624])`, `vol = lg2 * log2(w) * [0x858BA4] + offset` (FYL2X, 64-bit
/// intermediate) and the speed term `max'(v, [0x858F34])` (`v` kept only when greater; the caller
/// multiplies it by [0x858C98]). `flag` multiplies `v` by [0x85F0A0] first.
fn loop_volume_and_speed(v: X87, flag: u32) -> (f32, X87) {
    let v = if flag as u8 != 0 { v.fmul_m(c(0x85F0A0)) } else { v };
    let w = clamp_below(x(c(0x85F09C)).fmul(v), c(0x858624));
    let vol = f32_of(X87::lg2().fyl2x(w).fmul_m(c(0x858BA4)).fadd(xi(volume_offset())));
    let v = if v.fcom(x(c(0x858F34))).test_any(0x41) {
        x(c(0x858F34))
    } else {
        v
    };
    (vol, v)
}

/// Moves the sound's float at `off` towards `target` by at most `step` (0x4DA74F..0x4DA7FB): above ->
/// `min(cur + step, target)`, below -> `max(cur - step, target)` (the target itself when the stepped
/// value is not strictly inside), equal or unordered -> unchanged.
fn approach(sound: GPtr<u8>, off: isize, target: f32, step: f32) {
    let cur = fl(sound, off);
    let t = x(target);
    if !t.fcom(cur).test_any(0x41) {
        let s = cur.fadd_m(step);
        let v = if !s.fcom(t).test_parity(0x05) { s } else { t };
        wr::<f32>(sound, off, f32_of(v));
    } else if !t.fcom(cur).test_parity(0x05) {
        let s = cur.fsub_m(step);
        let v = if !s.fcom(t).test_any(0x41) { s } else { t };
        wr::<f32>(sound, off, f32_of(v));
    }
}

/// `CAECollisionAudioEntity::UpdateLoopingCollisionSound(CAESound*, CEntity*, CEntity*, uint8 s1,
/// uint8 s2, float impulse, CVector& posn, bool flag)` (0x4DA540, `ret 0x20`; `this` unused): the speed
/// term (relative speed of e1 - e2 when both surfaces are 0x3F, e1's own speed when exactly one is 0x3F
/// and the other not 0x3E, else [0x858B50]) times `T[s1].factor * T[s2].factor` (int32 +0xC of the
/// surface rows, `fild; fimul`), * [0x858FC4] * [0x858B3C], clamped below [0x858624]; then the shared
/// volume / speed tail; the sound's volume (+0x14) approaches it by at most [0x858624], its speed
/// (+0x1C) by at most [0x858B1C]; `SetPosition(*posn)`. `impulse` is not read.
#[hook(addr = 0x4DA540, conv = thiscall, name = "CAECollisionAudioEntity::UpdateLoopingCollisionSound", expect = "8A 54 24 10 83 EC 0C")]
#[allow(clippy::too_many_arguments)]
pub fn collision_update_looping(
    _this: GPtr<u8>,
    sound: u32,
    e1: u32,
    e2: u32,
    s1: u32,
    s2: u32,
    _impulse: u32,
    posn: u32,
    flag: u32,
) {
    let (a, b) = (s1 as u8, s2 as u8);
    let (p, q): (GPtr<u8>, GPtr<u8>) = (g(e1 as usize), g(e2 as usize));
    let v = match loop_speed_kind(a, b) {
        LoopSpeed::Relative => {
            // loads +44, +48, +4C, +50, +54, +58 (e1 - e2); the +58 difference goes through a dword
            let s = [
                dsub(p, q, 0x54),
                dsub(p, q, 0x50),
                dsub(p, q, 0x4C),
                dsub(p, q, 0x48),
                dsub(p, q, 0x44),
            ];
            let m = f32_of(dsub(p, q, 0x58));
            let nt = [dsub(p, q, 0x4C), dsub(p, q, 0x48), dsub(p, q, 0x44)];
            let t = [dsub(p, q, 0x58), dsub(p, q, 0x54), dsub(p, q, 0x50)];
            pick_magnitude(s, m, nt, t)
        }
        LoopSpeed::Own => own_speed(p),
        LoopSpeed::None => x(c(0x858B50)),
    };
    let k = xi(surface_row_i32(a, 0xC)).fmul(xi(surface_row_i32(b, 0xC)));
    let v = clamp_below(v.fmul(k).fmul_m(c(0x858FC4)).fmul_m(c(0x858B3C)), c(0x858624));
    let (vol, v) = loop_volume_and_speed(v, flag);
    let spd = f32_of(v.fmul_m(c(0x858C98)));
    let snd: GPtr<u8> = g(sound as usize);
    approach(snd, 0x14, vol, c(0x858624));
    approach(snd, 0x1C, spd, c(0x858B1C));
    let pv: GPtr<u8> = g(posn as usize);
    sound_set_position(snd, rd::<u32>(pv, 0), rd::<u32>(pv, 4), rd::<u32>(pv, 8));
}

/// A `CAESound` stack local (0x74 bytes, dword aligned) as the collision `Play*` functions build it:
/// the inlined constructor writes only +8 (physical = null); `Initialise` writes every field
/// `CAESound::operator=` (inside `RequestNewSound`) copies. The four it never writes (+0x55, +0x5A,
/// +0x6A, +0x72) are not copied by `operator=` and not read by the destructor, so their residue never
/// leaves the local.
struct StackSound([u32; 0x74 / 4]);

impl StackSound {
    fn new() -> StackSound {
        StackSound([0; 0x74 / 4])
    }
    fn ptr(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast()
    }
}

/// `CAECollisionAudioEntity::PlayLoopingCollisionSound(CEntity* e1, CEntity* e2, uint8 s1, uint8 s2,
/// float impulse, CVector& posn, bool flag)` (0x4DB450, `ret 0x1C`): speed term as
/// `UpdateLoopingCollisionSound` (relative speed loaded in the order +4C, +48, +44, +58, +54, +50) but
/// zero speed (`fcom 0; test ah, 0x44; jnp`) or no speed -> volume -100.0 and the speed floor
/// [0x858F34]; otherwise `sqrt(v * T1 * T2 * [0x858FC4]) * [0x858B3C]` clamped below [0x858C24], then
/// the shared tail. The sound id is 0 (grass on either surface), 3 (water), 2 (metal) or 1, tested in
/// that order (s1 then s2 each). `Initialise(slot 2, id, this, *posn, vol, 2.0, spd, 1.0, 0, flags 4,
/// 0, 0)`, `RequestNewSound`, and with a sound `AddCollisionSoundToList(e1, e2, s1, s2, sound, 2)`.
#[hook(addr = 0x4DB450, conv = thiscall, name = "CAECollisionAudioEntity::PlayLoopingCollisionSound", expect = "6A FF 68 78 BE 83 00")]
#[allow(clippy::too_many_arguments)]
pub fn collision_play_looping(
    this: GPtr<u8>,
    e1: u32,
    e2: u32,
    s1: u32,
    s2: u32,
    _impulse: u32,
    posn: u32,
    flag: u32,
) {
    let mut local = StackSound::new();
    let (a, b) = (s1 as u8, s2 as u8);
    let (p, q): (GPtr<u8>, GPtr<u8>) = (g(e1 as usize), g(e2 as usize));
    let speed = match loop_speed_kind(a, b) {
        LoopSpeed::Relative => {
            let s = [
                dsub(p, q, 0x54),
                dsub(p, q, 0x58),
                dsub(p, q, 0x44),
                dsub(p, q, 0x48),
                dsub(p, q, 0x4C),
            ];
            let m = f32_of(dsub(p, q, 0x50));
            let nt = [dsub(p, q, 0x44), dsub(p, q, 0x48), dsub(p, q, 0x4C)];
            let t = [dsub(p, q, 0x50), dsub(p, q, 0x54), dsub(p, q, 0x58)];
            Some(pick_magnitude(s, m, nt, t))
        }
        LoopSpeed::Own => Some(own_speed(p)),
        LoopSpeed::None => None,
    };
    let mut vol = f32::from_bits(0xC2C8_0000); // -100.0
    let floor = x(c(0x858F34));
    let v = match speed {
        Some(sp) if sp.fcom(x(c(0x858B50))).test_parity(0x44) => {
            let k = xi(surface_row_i32(a, 0xC)).fmul(xi(surface_row_i32(b, 0xC)));
            let v = clamp_below(
                sp.fmul(k).fmul_m(c(0x858FC4)).fsqrt().fmul_m(c(0x858B3C)),
                c(0x858C24),
            );
            let (vl, v) = loop_volume_and_speed(v, flag);
            vol = vl;
            v
        }
        _ => floor,
    };
    let spd = f32_of(v.fmul_m(c(0x858C98)));
    let si = g::<u8>(SURFACE_INFOS);
    let id = if surface_is_audio_grass(si, a as u32) != 0 || surface_is_audio_grass(si, b as u32) != 0 {
        0
    } else if surface_is_audio_water(si, a as u32) != 0 || surface_is_audio_water(si, b as u32) != 0 {
        3
    } else if surface_is_audio_metal(si, a as u32) != 0 || surface_is_audio_metal(si, b as u32) != 0 {
        2
    } else {
        1
    };
    let pv: GPtr<u8> = g(posn as usize);
    let sp = local.ptr();
    sound_initialise(
        sp,
        2,
        id,
        this.addr() as u32,
        rd::<u32>(pv, 0),
        rd::<u32>(pv, 4),
        rd::<u32>(pv, 8),
        vol.to_bits(),
        0x4000_0000,
        spd.to_bits(),
        0x3F80_0000,
        0,
        4,
        0,
        0,
    );
    let sound = sm_request_new_sound(g(SM), sp);
    if sound != 0 {
        collision_add_collision_sound_to_list(this, e1, e2, s1, s2, sound, 2);
    }
    sound_destructor(sp);
}

/// `CAECollisionAudioEntity::PlayOneShotCollisionSound(CEntity* e1, CEntity* e2, uint8 s1, uint8 s2,
/// float impulse, CVector& posn)` (0x4DB150, `ret 0x18`): two passes, (e1, e2, s1, s2) then (e2, e1,
/// s2, s1) as (a, b, sa, sb). A pass is skipped for `sa >= 0xC3`; `sa == 0xC0` needs `sb == 0xC0` and
/// bank 0x128 in slot 0x1D, and multiplies the impulse argument (kept for the second pass) by
/// [0x858C80]; `sb == 0xC0` (sa not) ends both passes, as does a missing bank 0xA0 / 0xE2 (slot 0x1D)
/// for `sa == 0xC1` / `0xC2`; those three use slot 0x1D, the rest slot 2. The id is
/// `ChooseCollisionSoundID(sa)` (-1 skips). `v = T[sb].factor * impulse * [0x858C58]` (`* [0x85862C]`
/// for sa 0xBC on sb 0x3E) `* [0x858B58]`; `n = floor(v)` (> 100: 100, else `floor((float)v)`), skipped
/// unless `0 < n`; `vol = offset - [0x858B3C]`, plus `lg2 log2((100 - (100-n)) / (100 - T[sa].level)) *
/// [0x858BA4]` when `100 - n > T[sa].level` (play position = that level's low word, else `100 - n`);
/// played only when `vol > [0x859014]` (or unordered): `Initialise(slot, id, this, *posn, vol, 2.0,
/// 1.0, 1.0, 0, 0x424, 0.02, pos)`, `RequestNewSound`; the surface's last id (+0x7C + 2 * sa) = id;
/// with a sound `AddCollisionSoundToList(a, b, s1, s2, sound, 1)` (the arguments' s1 / s2, unswapped).
#[hook(addr = 0x4DB150, conv = thiscall, name = "CAECollisionAudioEntity::PlayOneShotCollisionSound", expect = "6A FF 68 58 BE 83 00")]
pub fn collision_play_one_shot(this: GPtr<u8>, e1: u32, e2: u32, s1: u32, s2: u32, impulse: u32, posn: u32) {
    let mut local = StackSound::new();
    let mut impulse = f32::from_bits(impulse);
    for pass in 0..2 {
        let (ea, eb, sa, sb) = if pass == 0 {
            (e1, e2, s1 as u8, s2 as u8)
        } else {
            (e2, e1, s2 as u8, s1 as u8)
        };
        if sa >= 0xC3 {
            continue;
        }
        let mut slot = 2u32;
        if sa == 0xC0 {
            if sb != sa {
                break;
            }
            slot = 0x1D;
            if hw_is_sound_bank_loaded(g(HW), 0x128, 0x1D) == 0 {
                break;
            }
            impulse = f32_of(x(impulse).fmul_m(c(0x858C80)));
        } else {
            if sb == 0xC0 {
                break;
            }
            let bank = match sa {
                0xC1 => Some(0xA0),
                0xC2 => Some(0xE2),
                _ => None,
            };
            if let Some(bank) = bank {
                slot = 0x1D;
                if hw_is_sound_bank_loaded(g(HW), bank, 0x1D) == 0 {
                    break;
                }
            }
        }
        // `mov [esp+28h], bl; push dword [esp+28h]`: only the low byte is the surface (the callee
        // reads `movzx byte [esp+4]`)
        let id = collision_choose_collision_sound_id(this, sa as u32);
        if id as u16 == 0xFFFF {
            continue;
        }
        let mut v = xi(surface_row_i32(sb, 0xC)).fmul_m(impulse).fmul_m(c(0x858C58));
        if sa == 0xBC && sb == 0x3E {
            v = v.fmul_m(c(0x85862C));
        }
        let v = v.fmul_m(c(0x858B58));
        let vf = f32_of(v);
        let first = x87_floor_ftol(v.to_f64()) as u16 as i16;
        let n: i32 = if first > 100 {
            100
        } else {
            x87_floor_ftol(f64::from(vf)) as u16 as i16 as i32
        };
        let mut pos = 100i32.wrapping_sub(n) as u32;
        if pos as u16 as i16 >= 100 {
            continue;
        }
        let level = surface_row_i32(sa, 8);
        let d = pos as u16 as i16 as i32;
        let mut vol = xi(volume_offset()).fsub_m(c(0x858B3C));
        if d > level {
            let ratio = xi(100i32.wrapping_sub(d)).fdiv(xi(100i32.wrapping_sub(level)));
            pos = level as u16 as u32;
            vol = vol.fadd(X87::lg2().fyl2x(ratio).fmul_m(c(0x858BA4)));
        }
        if !vol.fcom(x(c(0x859014))).test_parity(0x41) {
            continue;
        }
        let pv: GPtr<u8> = g(posn as usize);
        let sp = local.ptr();
        sound_initialise(
            sp,
            slot,
            id,
            this.addr() as u32,
            rd::<u32>(pv, 0),
            rd::<u32>(pv, 4),
            rd::<u32>(pv, 8),
            f32_of(vol).to_bits(),
            0x4000_0000,
            0x3F80_0000,
            0x3F80_0000,
            0,
            0x424,
            0x3CA3_D70A,
            pos,
        );
        let sound = sm_request_new_sound(g(SM), sp);
        wr::<u16>(this, 0x7C + 2 * sa as isize, id as u16);
        if sound != 0 {
            collision_add_collision_sound_to_list(this, ea, eb, s1, s2, sound, 1);
        }
    }
    sound_destructor(local.ptr());
}

/// `fstp qword [esp]; call floor; call _ftol` (CRT 0x8219F0, exact on a double; 0x821B40), EAX.
fn x87_floor_ftol(v: f64) -> u32 {
    X87::from_f64_exact(sa_stream::modelinfo::vehicle::floor_f64(v)).ftol() as u32
}

/// The surface override of `ReportCollision` for the entity at pass `i` (`e`, the other one `other`):
/// vehicles 0x3F (bikes, +0x594 == 10: 0xBC), 0x43 when scraping a building (`matrix up . normal >
/// [0x858CC8]`) unless the friction effect of `fric_surface` is 1 (pass 0: the second surface byte as
/// the original holds it in BL; pass 1: the first argument byte as rewritten by pass 0); fixed models
/// 0x158 / 0x16B / 0x156, 0x16C, 0x157 and the model words at 0x8CD774 / 0x8CD77C / 0x8CD6F4 /
/// 0x8CD780; physicals with bit 7 of +0x40 0xC0; otherwise `cur` unchanged.
fn report_surface(e: GPtr<u8>, other: u32, normal: u32, cur: u8, fric_surface: u8) -> u8 {
    if rd::<u8>(e, 0x36) & 7 == 2 {
        let bike = rd::<u32>(e, 0x594) == 0xA;
        if other != 0 && rd::<u8>(g(other as usize), 0x36) & 7 == 1 && normal != 0 {
            let m: GPtr<u8> = g(rd::<u32>(e, 0x14) as usize);
            let n: GPtr<u8> = g(normal as usize);
            let d = fl(m, 0x28)
                .fmul_m(rd::<f32>(n, 8))
                .fadd(fl(m, 0x24).fmul_m(rd::<f32>(n, 4)))
                .fadd(fl(m, 0x20).fmul_m(rd::<f32>(n, 0)));
            if !d.fcom(x(c(0x858CC8))).test_any(0x41) {
                if bike {
                    return 0xBC;
                }
                if surface_get_friction_effect(g(SURFACE_INFOS), fric_surface as u32) != 1 {
                    return 0x43;
                }
            }
        }
        return if bike { 0xBC } else { 0x3F };
    }
    let model = rd::<u16>(e, 0x22);
    match model {
        0x158 => return 0x2D,
        0x16B => return 0xBE,
        0x156 | 0x16C | 0x157 => return 0xBF,
        _ => {}
    }
    let m = model as i16 as i32;
    if m == g::<u16>(0x8CD774).read() as i32 {
        0xC1
    } else if m == g::<u16>(0x8CD77C).read() as i32 {
        0xC2
    } else if m == g::<u16>(0x8CD6F4).read() as i32 {
        0xAF
    } else if m == g::<u16>(0x8CD780).read() as i32 {
        0x2B
    } else if entity_get_is_type_physical(e) != 0 && rd::<u8>(e, 0x40) & 0x80 != 0 {
        0xC0
    } else {
        cur
    }
}

/// `CAECollisionAudioEntity::ReportCollision(CEntity* e1, CEntity* e2, uint8 s1, uint8 s2, CVector&
/// posn, CVector* normal, float impulse, float speed, bool oneShot, bool unknown)` (0x4DBA10, `ret
/// 0x28`): nothing unless bank 0x27 is in slot 2, fewer than 300 collisions are active (+0x204) and
/// `speed` is not `<= [0x858B50]` (NaN continues). Pass 0 overrides s1 for e1 (friction test on s2),
/// pass 1 s2 for e2 (friction test on the new s1); the surface arguments go on as their argument dwords
/// with the low byte replaced. `oneShot` -> `PlayOneShotCollisionSound(e1, e2, s1, s2, impulse, posn)`.
/// Otherwise `GetCollisionSoundStatus(e1, e2, s1, s2, &index)` (the index in the `oneShot` slot):
/// 2 -> entry end time = now + 100 and, with a sound, `UpdateLoopingCollisionSound(sound, entry e1,
/// e2, s1, s2, impulse, posn, unknown ? unknown : 0)`; with `unknown` other statuses ->
/// `PlayLoopingCollisionSound(.., impulse, posn, unknown)`; without it status 1 -> `PlayLooping(.., 0)`,
/// the rest `PlayOneShotCollisionSound`.
#[hook(addr = 0x4DBA10, conv = thiscall, name = "CAECollisionAudioEntity::ReportCollision", expect = "51 56 8B F1 6A 02")]
#[allow(clippy::too_many_arguments)]
pub fn collision_report_collision(
    this: GPtr<u8>,
    e1: u32,
    e2: u32,
    s1: u32,
    s2: u32,
    posn: u32,
    normal: u32,
    impulse: u32,
    speed: u32,
    one_shot: u32,
    unknown: u32,
) {
    if hw_is_sound_bank_loaded(g(HW), 0x27, 2) == 0 || rd::<u32>(this, 0x204) == 300 {
        return;
    }
    if !x(f32::from_bits(speed)).fcom(x(c(0x858B50))).test_parity(0x41) {
        return;
    }
    let a = report_surface(g(e1 as usize), e2, normal, s1 as u8, s2 as u8);
    let b = report_surface(g(e2 as usize), e1, normal, s2 as u8, a);
    let s1 = (s1 & !0xFF) | a as u32;
    let s2 = (s2 & !0xFF) | b as u32;
    if one_shot as u8 != 0 {
        collision_play_one_shot_call(this, e1, e2, s1, s2, impulse, posn);
        return;
    }
    let mut index = [one_shot];
    let status =
        collision_get_collision_sound_status(this, e1, e2, s1, s2, GPtr::from_raw(index.as_mut_ptr()).cast());
    let flag = if unknown as u8 != 0 { unknown } else { 0 };
    if status == 2 {
        let e = this.byte_add(0x208 + index[0].wrapping_mul(0x14) as i32 as isize);
        wr::<u32>(e, 0xC, g::<u32>(TIME_MS).read().wrapping_add(100));
        let snd = rd::<u32>(e, 8);
        if snd != 0 {
            collision_update_looping_call(
                this,
                snd,
                rd::<u32>(e, 0),
                rd::<u32>(e, 4),
                rd::<u8>(e, 0x11) as u32,
                rd::<u8>(e, 0x12) as u32,
                impulse,
                posn,
                flag,
            );
        }
        return;
    }
    if unknown as u8 != 0 || status == 1 {
        collision_play_looping_call(this, e1, e2, s1, s2, impulse, posn, flag);
    } else {
        collision_play_one_shot_call(this, e1, e2, s1, s2, impulse, posn);
    }
}

// ---- CAEPedAudioEntity: Service / shirt flap ------------------------------------------------------

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0, AL)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CVehicle` virtual slot 0x9C (AL; asked of automobiles before the wind-rush volume)
    fn vcall_vehicle_9c(veh: GPtr<u8>) -> u8 = thiscall vslot 0x9C => unported;
    /// `CPedIntelligence::GetUsingParachute()` (0x6011B0, AL)
    fn intelligence_get_using_parachute(intel: GPtr<u8>) -> u8 = thiscall 0x6011B0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, const char* name)` (0x4D6870, cdecl)
    fn anim_get_association(clump: u32, name: u32) -> u32 = cdecl 0x4D6870 => unported;
    /// `CAEPedAudioEntity::PlayShirtFlap(float volume, float speed)` (0x4E2A90)
    fn ped_play_shirt_flap_call(this: GPtr<u8>, volume: u32, speed: u32) = thiscall 0x4E2A90 => ped_play_shirt_flap;
    /// `CAETwinLoopSoundEntity::StopSoundAndForget()` (0x4F2960)
    fn twin_stop_sound_and_forget(twin: GPtr<u8>) = thiscall 0x4F2960 => crate::entities::twin_loop_stop_sound_and_forget;
    /// `CAETwinLoopSoundEntity::Initialise(int16 slot, int16 sfx1, int16 sfx2, CAEAudioEntity*, int16, int16, int16, int16)` (0x4F28A0)
    fn twin_initialise(twin: GPtr<u8>, slot: u32, sfx1: u32, sfx2: u32, ent: u32, min_time: u32, max_time: u32, a: u32, b: u32) = thiscall 0x4F28A0 => unported;
    /// `CAETwinLoopSoundEntity::UpdateTwinLoopSound(CVector pos, float volume, float speed)` (0x4F29A0)
    fn twin_update(twin: GPtr<u8>, px: u32, py: u32, pz: u32, volume: u32, speed: u32) = thiscall 0x4F29A0 => unported;
    /// `CAETwinLoopSoundEntity::PlayTwinLoopSound(CVector pos, float volume, float speed, float maxDistance, float timeScale, eSoundEnvironment)` (0x4F2CB0)
    fn twin_play(twin: GPtr<u8>, px: u32, py: u32, pz: u32, volume: u32, speed: u32, max_distance: u32, time_scale: u32, flags: u32) = thiscall 0x4F2CB0 => unported;
}

/// `|v|` of the vector at `e + off` as the bytes compute it: `fld z; fld y; fld x`, then
/// `sqrt((x² + y²) + z²)`.
fn magnitude_at(e: GPtr<u8>, off: isize) -> X87 {
    let (px, py, pz) = (fl(e, off), fl(e, off + 4), fl(e, off + 8));
    sq(px).fadd(sq(py)).fadd(sq(pz)).fsqrt()
}

/// The clamp of a stored dword to [0, 1] (0x4E2FFC / 0x4E311E / 0x4E3185 / 0x4E31CE): `[0x858624] < v`
/// -> 1.0 (the literal 0x3F800000), `0.0 > v` -> 0, otherwise (NaN included) unchanged. The third
/// compare of the longer variants repeats the first one and cannot change the result.
fn clamp01(v: f32) -> f32 {
    if x(c(0x858624)).fcom(x(v)) == sa_math::x87::Cmp::Less {
        f32::from_bits(0x3F80_0000)
    } else if !x(c(0x858B50)).fcom(x(v)).test_any(0x41) {
        0.0
    } else {
        v
    }
}

/// An entity's position: `matrix ? &matrix->pos : &placement.pos`.
fn entity_position(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<u32>(e, 0x14);
    if m != 0 {
        g::<u8>(m as usize).byte_add(0x30)
    } else {
        e.byte_add(4)
    }
}

/// `CAEPedAudioEntity::PlayShirtFlap(float volume, float speed)` (0x4E2A90, `ret 8`): the twin loop at
/// +0xA8 not playing (word +0x130 == 0) -> `Initialise(5, 0x13, 0x14, this, 200, 1000, -1, -1)` and
/// `PlayTwinLoopSound(ped position, volume, speed, 2.0, 1.0, 0)`; otherwise
/// `UpdateTwinLoopSound(ped position, volume, speed)`. The original's `CAESound` stack local is never
/// initialised beyond the constructor's +8 = null and only destroyed.
#[hook(addr = 0x4E2A90, conv = thiscall, name = "CAEPedAudioEntity::PlayShirtFlap", expect = "6A FF 68 B8 BF 83 00")]
pub fn ped_play_shirt_flap(this: GPtr<u8>, volume: u32, speed: u32) {
    let mut local = StackSound::new();
    let twin = this.byte_add(0xA8);
    let ped: GPtr<u8> = g(rd::<u32>(this, 0x94) as usize);
    if rd::<u16>(this, 0x130) == 0 {
        twin_initialise(
            twin,
            5,
            0x13,
            0x14,
            this.addr() as u32,
            200,
            1000,
            0xFFFF_FFFF,
            0xFFFF_FFFF,
        );
        let p = entity_position(ped);
        twin_play(
            twin,
            rd::<u32>(p, 0),
            rd::<u32>(p, 4),
            rd::<u32>(p, 8),
            volume,
            speed,
            0x4000_0000,
            0x3F80_0000,
            0,
        );
    } else {
        let p = entity_position(ped);
        twin_update(
            twin,
            rd::<u32>(p, 0),
            rd::<u32>(p, 4),
            rd::<u32>(p, 8),
            volume,
            speed,
        );
    }
    sound_destructor(local.ptr());
}

/// `AudioLog10(v) * [0x858BA4] + base` and `[k] * v + [add]`, both stored (one wind-rush source).
fn wind_volume_speed(v: f32, base: f32, k: usize, add: usize) -> (f32, f32) {
    let vol = f32_of(crate::utility::audio_log10(v).fmul_m(c(0x858BA4)).fadd_m(base));
    (vol, f32_of(x(c(k)).fmul_m(v).fadd_m(c(add))))
}

/// `CAEPedAudioEntity::Service()` (0x4E2EE0): nothing for a non-player ped. Wind rush / shirt flap:
/// volume -100 and speed 1.0 by default; a player driving (state 0x32) a bike, or an automobile whose
/// vslot 0x9C says yes, with a non-zero speed: `v = clamp01(|move speed|)`, volume
/// `AudioLog10(v) * [0x858BA4] + ([veh+0x1B8] == 2 ? [0x8AE654] : [0x8AE650])`, speed `[0xB613BC] * v +
/// [0x8AE658]`. On foot (not attached to a vehicle, flag 0x100 of +0x46C clear): `s = |ped move speed| /
/// [0x8AE6A4]`; without a parachute `s < [0x8AE69C]` -> 0, else `clamp01((s - [0x8AE69C]) /
/// [0xB613CC])`, base [0x8AE660]; with a parachute base [0x8AE664], the blend amounts of the two
/// parachute animations (names 0x85F1E4, 0x85F1D8) clamped as `a1`, `a2`, and `clamp01(s)`; volume and
/// speed as above with [0xB613C0] / [0x8AE668]. Then `speed *= [0x8AE644] * a1 + 1.0`, `T = [0x8AE648] *
/// a1 + vol + [0x8AE64C] * a2`; +0x158 moves towards T by at most [0x8AE670] up / [0x8AE674] down (T
/// itself when within the step; unordered: kept) and is set to T outright when both T and it are below
/// [0x8AE6A0]. +0x158 above [0x859014] (or NaN) -> `PlayShirtFlap(+0x158, speed)`, else the twin loop
/// stops (`StopSoundAndForget`, a tail call in the original).
#[hook(addr = 0x4E2EE0, conv = thiscall, name = "CAEPedAudioEntity::Service", expect = "83 EC 18 53 56")]
pub fn ped_service(this: GPtr<u8>) {
    let ped: GPtr<u8> = g(rd::<u32>(this, 0x94) as usize);
    let mut vol = f32::from_bits(0xC2C8_0000);
    let mut speed = 1.0f32;
    let mut a1 = 0.0f32;
    let mut a2 = 0.0f32;
    if ped_is_player(ped) == 0 {
        return;
    }
    let attached = rd::<u32>(ped, 0xFC);
    let on_foot = !((attached != 0 && rd::<u8>(g(attached as usize), 0x36) & 7 == 2)
        || rd::<u32>(ped, 0x46C) & 0x100 != 0);
    if rd::<u32>(ped, 0x530) == 0x32 && rd::<u32>(ped, 0x58C) != 0 {
        let vp: GPtr<u8> = g(rd::<u32>(ped, 0x58C) as usize);
        let sub = rd::<u32>(vp, 0x590);
        if sub == 9 || (sub == 0 && vcall_vehicle_9c(vp) != 0) {
            let vp: GPtr<u8> = g(rd::<u32>(ped, 0x58C) as usize);
            let mut m = magnitude_at(vp, 0x44);
            let zero = x(c(0x858B50));
            if !m.fcom(zero).test_parity(0x05) {
                m = m.fchs();
            }
            if !m.fcom(zero).test_any(0x41) {
                let base = if rd::<u8>(vp, 0x1B8) == 2 {
                    c(0x8AE654)
                } else {
                    c(0x8AE650)
                };
                (vol, speed) = wind_volume_speed(clamp01(f32_of(m)), base, 0xB613BC, 0x8AE658);
            }
        }
    }
    let intel = rd::<u32>(ped, 0x47C);
    let parachute = intel != 0 && intelligence_get_using_parachute(g(intel as usize)) != 0;
    if on_foot {
        let s = f32_of(magnitude_at(ped, 0x44).fdiv_m(c(0x8AE6A4)));
        let (s, base) = if !parachute {
            if !x(s).fcom(x(c(0x8AE69C))).test_parity(0x05) {
                (0.0, c(0x8AE660))
            } else {
                (
                    clamp01(f32_of(x(s).fsub_m(c(0x8AE69C)).fdiv_m(c(0xB613CC)))),
                    c(0x8AE660),
                )
            }
        } else {
            let assoc = anim_get_association(rd::<u32>(ped, 0x18), 0x85F1E4);
            if assoc != 0 {
                a1 = clamp01(rd::<f32>(g(assoc as usize), 0x18));
            }
            let assoc = anim_get_association(rd::<u32>(ped, 0x18), 0x85F1D8);
            if assoc != 0 {
                a2 = clamp01(rd::<f32>(g(assoc as usize), 0x18));
            }
            (clamp01(s), c(0x8AE664))
        };
        (vol, speed) = wind_volume_speed(s, base, 0xB613C0, 0x8AE668);
    }
    let speed = f32_of(x(c(0x8AE644)).fmul_m(a1).fadd_m(c(0x858624)).fmul_m(speed));
    let t = x(c(0x8AE648))
        .fmul_m(a1)
        .fadd_m(vol)
        .fadd(x(c(0x8AE64C)).fmul_m(a2));
    let cur = rd::<f32>(this, 0x158);
    if !t.fcom(x(cur)).test_any(0x41) {
        let n = x(c(0x8AE670)).fadd_m(cur);
        let v = if !n.fcom(t).test_parity(0x05) { n } else { t };
        wr::<f32>(this, 0x158, f32_of(v));
    } else if !t.fcom(x(cur)).test_parity(0x41) {
        let n = x(cur).fsub_m(c(0x8AE674));
        let v = if !n.fcom(t).test_any(0x41) { n } else { t };
        wr::<f32>(this, 0x158, f32_of(v));
    }
    let lim = c(0x8AE6A0);
    if !t.fcom(x(lim)).test_parity(0x05) && !x(rd::<f32>(this, 0x158)).fcom(x(lim)).test_parity(0x05) {
        wr::<f32>(this, 0x158, f32_of(t));
    }
    let v = rd::<f32>(this, 0x158);
    if !x(v).fcom(x(c(0x859014))).test_parity(0x41) {
        twin_stop_sound_and_forget(this.byte_add(0xA8));
        return;
    }
    ped_play_shirt_flap_call(this, v.to_bits(), speed.to_bits());
}

// ---- CAEWeatherAudioEntity::Service ---------------------------------------------------------------

orig! {
    /// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntity(int16 event, CAEAudioEntity*)` (0x4EF570, AX)
    fn sm_are_sounds_of_this_event_playing(sm: GPtr<u8>, event: u32, entity: u32) -> u32 = thiscall 0x4EF570 => unported;
    /// `CAEAudioHardware::LoadSoundBank(uint16 bankId, int16 slot)` (0x4D88A0)
    fn hw_load_sound_bank(hw: GPtr<u8>, bank: u32, slot: u32) = thiscall 0x4D88A0 => unported;
    /// `CCullZones::PlayerNoRain()` / `CamNoRain()` (AL)
    fn cull_player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    fn cull_cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CAETwinLoopSoundEntity::DoSoundsSwitchThisFrame()` (0x4F2CA0, AL)
    fn twin_do_sounds_switch_this_frame(twin: GPtr<u8>) -> u8 = thiscall 0x4F2CA0 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange(float, float)` (0x4D9C50, ST0)
    fn ae_random_float_st0(min: u32, max: u32) -> f64 = cdecl 0x4D9C50 => unported;
    /// `CAEAudioUtility::ResolveProbability(float)` (0x4D9C80, AL)
    fn ae_resolve_probability(p: f32) -> u8 = cdecl 0x4D9C80 => crate::utility::resolve_probability;
}

/// `CAEWeatherAudioEntity::m_sfRainVolume` (0x8CC30C) and `m_snLastRainDropSoundID` (0x8CC310, int16)
const RAIN_VOLUME: usize = 0x8CC30C;
const LAST_RAIN_DROP_ID: usize = 0x8CC310;
/// The two rain twin loops (`m_sfRainTwinLoopSoundEntity` and its neighbour, 0xA8 bytes each).
const RAIN_TWIN_1: usize = 0xB6BB18;
const RAIN_TWIN_2: usize = 0xB6BBC0;
/// `CWeather::Rain`, `CGame::currArea`
const WEATHER_RAIN: usize = 0xC81324;
const CURR_AREA: usize = 0xB72914;

/// One wind sound: `Initialise(slot 0, id, this, pos, -50.0, 1.0, 1.0, 1.0, 0, flags, 0, play_pos)`,
/// event = `event` (+0xC, written after `Initialise`), `RequestNewSound`.
fn weather_wind_sound(
    this: GPtr<u8>,
    local: &mut StackSound,
    id: u32,
    pos: [u32; 3],
    flags: u32,
    play_pos: u32,
    event: u32,
) {
    let sp = local.ptr();
    sound_initialise(
        sp,
        0,
        id,
        this.addr() as u32,
        pos[0],
        pos[1],
        pos[2],
        0xC248_0000,
        0x3F80_0000,
        0x3F80_0000,
        0x3F80_0000,
        0,
        flags,
        0,
        play_pos,
    );
    wr::<u32>(sp, 0xC, event);
    sm_request_new_sound(g(SM), sp);
}

/// A rain drop: id = last id + 1 (int16 compare, > 11 -> 2; the original increments the whole dword),
/// `volume = rand(-6, 6) + (rain volume - [0x858B48])`, one of two positions by `ResolveProbability(0.5)`,
/// `Initialise(slot 6, id, this, pos, volume, 1.0, 1.0, 1.0, 0, 0, 0, 0)`, flags word (+0x56) = 3,
/// `RequestNewSound`; the last id word is updated.
fn weather_rain_drop(this: GPtr<u8>, local: &mut StackSound, a: [u32; 3], b: [u32; 3]) {
    let mut id = g::<u32>(LAST_RAIN_DROP_ID).read().wrapping_add(1);
    if id as u16 as i16 > 0xB {
        id = 2;
    }
    let r = X87::from_f64_exact(ae_random_float_st0(0xC0C0_0000, 0x40C0_0000));
    let vol = f32_of(r.fadd(x(g::<f32>(RAIN_VOLUME).read()).fsub_m(c(0x858B48))));
    let pos = if ae_resolve_probability(0.5) != 0 { a } else { b };
    let sp = local.ptr();
    sound_initialise(
        sp,
        6,
        id,
        this.addr() as u32,
        pos[0],
        pos[1],
        pos[2],
        vol.to_bits(),
        0x3F80_0000,
        0x3F80_0000,
        0x3F80_0000,
        0,
        0,
        0,
        0,
    );
    wr::<u16>(sp, 0x56, 3);
    sm_request_new_sound(g(SM), sp);
    g::<u16>(LAST_RAIN_DROP_ID).write(id as u16);
}

/// One rain twin loop: not playing (+0x88 == 0) -> bank 0x69 in slot 6 or `LoadSoundBank(0x69, 6)` and
/// `false` (the caller ends); else `Initialise(6, sfx1, sfx2, this, 0x41, 0x15E, a, a)` and
/// `PlayTwinLoopSound(pos, rain volume, 1.0, 1.0, 1.0, 0x23)`. Playing -> `UpdateTwinLoopSound(pos,
/// rain volume, 1.0)` and a rain drop when the loop switches this frame.
#[allow(clippy::too_many_arguments)]
fn weather_rain_twin(
    this: GPtr<u8>,
    local: &mut StackSound,
    twin: usize,
    sfx: (u32, u32),
    a: u32,
    pos: [u32; 3],
    drop_alt: [u32; 3],
) -> bool {
    let tw = g::<u8>(twin);
    let vol = g::<u32>(RAIN_VOLUME).read();
    if rd::<u16>(tw, 0x88) == 0 {
        if hw_is_sound_bank_loaded(g(HW), 0x69, 6) == 0 {
            hw_load_sound_bank(g(HW), 0x69, 6);
            return false;
        }
        twin_initialise(tw, 6, sfx.0, sfx.1, this.addr() as u32, 0x41, 0x15E, a, a);
        twin_play(
            tw,
            pos[0],
            pos[1],
            pos[2],
            vol,
            0x3F80_0000,
            0x3F80_0000,
            0x3F80_0000,
            0x23,
        );
    } else {
        twin_update(tw, pos[0], pos[1], pos[2], vol, 0x3F80_0000);
        if twin_do_sounds_switch_this_frame(tw) != 0 {
            weather_rain_drop(this, local, pos, drop_alt);
        }
    }
    true
}

/// `CAEWeatherAudioEntity::Service()` (0x5052F0): outside interiors (`CGame::currArea` 0) the wind
/// sounds: event 3 not playing for this entity -> bank 0x3B in slot 0: two sounds (id 0 at (-0.905,
/// 0.423, 0) flags 0x1005; id 0 at (0.905, 0.423, 0) flags 0x1025, play position 0x32), else
/// `LoadSoundBank(0x3B, 0)`; event 4: id 0x1D at the first position; event 5: id 0x1E at the second
/// (positions are the bytes' stack constants 0x3F67EF9E / 0x3ED89375). Rain: `T = offset([+0x8F]) +
/// lg2 log2(max'(Rain - [0x858CC4], 0) * [0x8595F0]) * [0x858BA4]` when it rains (`Rain > 0` or NaN),
/// neither cull zone forbids it and in no interior (a rain volume not above [0x859B60] restarts at
/// -50), else `T = [0x859014]`; the rain volume moves towards T by at most [0x858B8C] (T itself when
/// within the step). `T == [0x859014]` and the rain volume not above [0x859B60] -> volume -100 and both
/// rain loops stop; otherwise the two rain twin loops (sfx 1/0 then 0/1, the second with 0x32 / 0x32)
/// play or update, each switch adding a rain drop.
#[hook(addr = 0x5052F0, conv = thiscall, name = "CAEWeatherAudioEntity::Service", expect = "6A FF 68 28 C5 83 00")]
pub fn weather_service(this: GPtr<u8>) {
    let mut local = StackSound::new();
    // the bytes' stack constants: (-0.905, 0.423, 0), (-0.905, -0.423, 0), (0.905, 0.423, 0), (0.905, -0.423, 0)
    let left_front = [0xBF67_EF9E, 0x3ED8_9375, 0];
    let left_back = [0xBF67_EF9E, 0xBED8_9375, 0];
    let right_front = [0x3F67_EF9E, 0x3ED8_9375, 0];
    let right_back = [0x3F67_EF9E, 0xBED8_9375, 0];
    let ent = this.addr() as u32;
    if g::<u32>(CURR_AREA).read() == 0 {
        if sm_are_sounds_of_this_event_playing(g(SM), 3, ent) as u16 == 0 {
            if hw_is_sound_bank_loaded(g(HW), 0x3B, 0) != 0 {
                weather_wind_sound(this, &mut local, 0, left_front, 0x1005, 0, 3);
                weather_wind_sound(this, &mut local, 0, right_front, 0x1025, 0x32, 3);
            } else {
                hw_load_sound_bank(g(HW), 0x3B, 0);
            }
        }
        if sm_are_sounds_of_this_event_playing(g(SM), 4, ent) as u16 == 0 {
            if hw_is_sound_bank_loaded(g(HW), 0x3B, 0) != 0 {
                weather_wind_sound(this, &mut local, 0x1D, left_front, 0x1005, 0, 4);
            } else {
                hw_load_sound_bank(g(HW), 0x3B, 0);
            }
        }
        if sm_are_sounds_of_this_event_playing(g(SM), 5, ent) as u16 == 0 {
            if hw_is_sound_bank_loaded(g(HW), 0x3B, 0) != 0 {
                weather_wind_sound(this, &mut local, 0x1E, right_front, 0x1025, 0x32, 5);
            } else {
                hw_load_sound_bank(g(HW), 0x3B, 0);
            }
        }
    }
    let raining = x(g::<f32>(WEATHER_RAIN).read())
        .fcom(x(c(0x858B50)))
        .test_parity(0x41)
        && cull_player_no_rain() == 0
        && cull_cam_no_rain() == 0
        && g::<u32>(CURR_AREA).read() == 0;
    let t = if raining {
        if x(g::<f32>(RAIN_VOLUME).read())
            .fcom(x(c(0x859B60)))
            .test_any(0x41)
        {
            g::<f32>(RAIN_VOLUME).write(f32::from_bits(0xC248_0000));
        }
        let mut v = x(g::<f32>(WEATHER_RAIN).read()).fsub_m(c(0x858CC4));
        if v.fcom(x(c(0x858B50))).test_any(0x41) {
            v = x(c(0x858B50));
        }
        let v = v.fmul_m(c(0x8595F0));
        let off = rd::<i8>(g(g::<u32>(VOLUME_OFFSET_PTR).read() as usize), 0x8F) as i32;
        xi(off).fadd(X87::lg2().fyl2x(v).fmul_m(c(0x858BA4)))
    } else {
        x(c(0x859014))
    };
    let cur = g::<f32>(RAIN_VOLUME).read();
    let step = c(0x858B8C);
    if !t.fcom(x(cur)).test_parity(0x05) {
        // T < current: step down
        let n = x(cur).fsub_m(step);
        let v = if !n.fcom(t).test_any(0x41) { n } else { t };
        g::<f32>(RAIN_VOLUME).write(f32_of(v));
    } else if !t.fcom(x(cur)).test_any(0x41) {
        // T > current: step up
        let n = x(cur).fadd_m(step);
        let v = if !n.fcom(t).test_parity(0x05) { n } else { t };
        g::<f32>(RAIN_VOLUME).write(f32_of(v));
    }
    if !t.fcom(x(c(0x859014))).test_parity(0x44)
        && !x(g::<f32>(RAIN_VOLUME).read())
            .fcom(x(c(0x859B60)))
            .test_parity(0x41)
    {
        g::<u32>(RAIN_VOLUME).write(0xC2C8_0000);
        if g::<u16>(RAIN_TWIN_1 + 0x88).read() != 0 {
            twin_stop_sound_and_forget(g(RAIN_TWIN_1));
        }
        if g::<u16>(RAIN_TWIN_2 + 0x88).read() != 0 {
            twin_stop_sound_and_forget(g(RAIN_TWIN_2));
        }
    } else if weather_rain_twin(this, &mut local, RAIN_TWIN_1, (1, 0), 0, left_front, left_back) {
        weather_rain_twin(
            this,
            &mut local,
            RAIN_TWIN_2,
            (0, 1),
            0x32,
            right_front,
            right_back,
        );
    }
    sound_destructor(local.ptr());
}

// ---- unnamed AUDIO rows ---------------------------------------------------------------------------

/// `FUN_004DA510(uint8 surface)` (0x4DA510, cdecl, ST0): the collision volume of a surface,
/// `lg2 * log2(T[surface].factor * [0x858C58]) * [0x858BA4]` (int32 +0xC of the 16-byte surface rows at
/// 0x8AD908; FYL2X's 64-bit result rounded by the PC=24 multiply, so the value is narrow). Called by
/// `CAEPedAudioEntity::HandlePedHit` (0x4E1FE5, 0x4E2077).
#[hook(addr = 0x4DA510, conv = cdecl, name = "FUN_004da510", expect = "0F B6 44 24 04 C1 E0 04")]
pub fn collision_surface_volume(surface: u32) -> X87 {
    let v = xi(surface_row_i32(surface as u8, 0xC)).fmul_m(c(0x858C58));
    X87::lg2().fyl2x(v).fmul_m(c(0x858BA4))
}

orig! {
    /// KERNEL32 `FreeLibrary`, as `call [0x858110]` at 0x502BFF
    fn free_library(module: u32) -> i32 = stdcall site 0x502BFF => unported;
}

/// `FUN_00502BF0()` (0x502BF0, cdecl; called by `CAEUserRadioTrackManager::Shutdown` 0x4F2FC6): when the
/// module flag 0xB6BAD8 is set, `FreeLibrary([0xB6BAD4])` and the flag cleared (the user-track
/// decoder library loaded on demand). No result (the caller tail-jumps on without reading EAX).
/// Not hooked: its catalogue row has the placeholder name "(missed by ghidra: cat7-call)" (sa-hook table rule); reached through `orig!` only.
pub fn user_radio_free_decoder_library() {
    if g::<u32>(0xB6BAD8).read() != 0 {
        free_library(g::<u32>(0xB6BAD4).read());
        g::<u32>(0xB6BAD8).write(0);
    }
}

// ---- CAEPedSpeechAudioEntity: ped conversations ---------------------------------------------------

orig! {
    /// `CAEPedSpeechAudioEntity` virtual slot 0x10 (`GetPedType`, AX)
    fn vcall_speech_get_ped_type(speech: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CPed::CanPedHoldConversation()` (0x5EFFA0, AL) / `CPed::GetPedTalking()` (0x5EFF50, AL)
    fn ped_can_ped_hold_conversation(ped: GPtr<u8>) -> u8 = thiscall 0x5EFFA0 => unported;
    fn ped_get_ped_talking(ped: GPtr<u8>) -> u8 = thiscall 0x5EFF50 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60, cdecl, returns out)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u32 = cdecl 0x40FE60 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr_st0(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CAEPedSpeechAudioEntity::ReservePedConversationSpeechSlots()` (0x4E37F0, AL)
    fn speech_reserve_ped_conversation_speech_slots() -> u8 = cdecl 0x4E37F0 => crate::port2_b::speech_reserve_ped_conversation_speech_slots;
    /// `CAEPedSpeechAudioEntity::SetUpConversation()` (0x4E3A00)
    fn speech_set_up_conversation_call() = cdecl 0x4E3A00 => speech_set_up_conversation;
    /// `CAEAudioUtility::GetRandomNumberInRange(int, int)` (0x4D9C10)
    fn ae_random_int(min: i32, max: i32) -> i32 = cdecl 0x4D9C10 => crate::utility::get_random_number_in_range_int;
}

/// `CAEPedSpeechAudioEntity` conversation statics: `s_bPedConversationHappening` 0xB613FD,
/// `s_bPlayerConversationHappening` 0xB613FC, `s_pConversationPed1` 0xB61410 / `Ped2` 0xB6140C, the
/// conversation type word 0xB613E8 and the 8 context words 0xB613EC..0xB613FA.
const CONV_PED_HAPPENING: usize = 0xB613FD;
const CONV_PLAYER_HAPPENING: usize = 0xB613FC;
const CONV_PED1: usize = 0xB61410;
const CONV_PED2: usize = 0xB6140C;
const CONV_TYPE: usize = 0xB613E8;
const CONV_CTX: usize = 0xB613EC;

/// `TheCamera`'s position (`m_matrix ? &m_matrix->pos : &placement.pos`, 0xB6F03C / 0xB6F02C).
fn camera_position() -> GPtr<u8> {
    let m = g::<u32>(0xB6F03C).read();
    if m != 0 {
        g::<u8>(m as usize).byte_add(0x30)
    } else {
        g(0xB6F02C)
    }
}

/// `MagnitudeSqr(ped position - camera position) > [0x859600]` (through the original `VectorSub` and
/// `MagnitudeSqr`; NaN counts as near).
fn too_far_from_camera(ped: GPtr<u8>) -> bool {
    let mut d = [0u32; 3];
    let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    vector_sub(dp, entity_position(ped), camera_position());
    let m = X87::from_f64_exact(vector_magnitude_sqr_st0(dp));
    !m.fcom(x(c(0x859600))).test_any(0x41)
}

/// The conversation context of a ped by its type (+0x598 0x16 or 5 -> `female`, else `male`).
fn conv_ctx_for(ped: u32, female: u16, male: u16) -> u16 {
    let t = rd::<u32>(g(ped as usize), 0x598);
    if t == 0x16 || t == 5 { female } else { male }
}

/// `rand(0, 2)` mapped as 0 -> 0xA2, 1 -> 0xA1, else 0xA3 (`0xA1 + 2 * (ax != 1)`).
fn conv_ctx_reply() -> u16 {
    let r = ae_random_int(0, 2) as u16;
    if r == 0 { 0xA2 } else { 0xA1 + 2 * (r != 1) as u16 }
}

/// `CAEPedSpeechAudioEntity::SetUpConversation()` (0x4E3A00, cdecl): clears the 8 context words, sets
/// the greetings (word 0: ped2's, word 1: ped1's; 0xA5 for ped types 0x16 / 5, else 0xA6), then by
/// `rand(1, 10)`: 1..4 -> type 3, `rand(0, 1)` ? (0xAB, 0xA2) : (0xAA, 0xA1), 0xA9, the reply word, and
/// the farewells (ped2, ped1: 0xA7 / 0xA8); 5..8 -> type 2, an unused `rand(0, 1)`, 0xA9, the reply,
/// the farewells; 9..10 -> type 1, `0xAA + (rand(0, 1) != 0)`, 0xA4.
#[hook(addr = 0x4E3A00, conv = cdecl, name = "CAEPedSpeechAudioEntity::SetUpConversation", expect = "B8 EE 13 B6 00")]
pub fn speech_set_up_conversation() {
    let w = |i: usize, v: u16| g::<u16>(CONV_CTX + 2 * i).write(v);
    for i in 0..8 {
        w(i, 0);
    }
    let (p1, p2) = (g::<u32>(CONV_PED1).read(), g::<u32>(CONV_PED2).read());
    w(0, conv_ctx_for(p2, 0xA5, 0xA6));
    w(1, conv_ctx_for(p1, 0xA5, 0xA6));
    let r = ae_random_int(1, 10);
    match r.wrapping_sub(1) as u32 {
        0..=3 => {
            g::<u16>(CONV_TYPE).write(3);
            if ae_random_int(0, 1) as u16 != 0 {
                w(2, 0xAB);
                w(3, 0xA2);
            } else {
                w(2, 0xAA);
                w(3, 0xA1);
            }
            w(4, 0xA9);
            w(5, conv_ctx_reply());
            w(6, conv_ctx_for(g::<u32>(CONV_PED2).read(), 0xA7, 0xA8));
            w(7, conv_ctx_for(g::<u32>(CONV_PED1).read(), 0xA7, 0xA8));
        }
        4..=7 => {
            g::<u16>(CONV_TYPE).write(2);
            ae_random_int(0, 1);
            w(2, 0xA9);
            w(3, conv_ctx_reply());
            w(4, conv_ctx_for(g::<u32>(CONV_PED2).read(), 0xA7, 0xA8));
            w(5, conv_ctx_for(g::<u32>(CONV_PED1).read(), 0xA7, 0xA8));
        }
        8..=9 => {
            g::<u16>(CONV_TYPE).write(1);
            let r = ae_random_int(0, 1) as u16;
            w(3, 0xA4);
            w(2, 0xAA + (r != 0) as u16);
        }
        _ => {}
    }
}

/// `CAEPedSpeechAudioEntity::RequestPedConversation(CPed* ped1, CPed* ped2)` (0x4E50E0, cdecl, AL):
/// refused while a ped conversation is being set up (0xB613E6), either ped has +0x32E / +0x32D set, a
/// ped or player conversation is happening, both speech entities (+0x294) report the same ped type
/// (vslot 0x10, int16), either ped cannot hold a conversation or is talking, either is farther than
/// sqrt([0x859600]) from the camera, or the speech slots cannot be reserved. Otherwise the conversation
/// starts (0xB613FD = 1, peds stored) and `SetUpConversation` runs; true.
#[hook(addr = 0x4E50E0, conv = cdecl, name = "CAEPedSpeechAudioEntity::RequestPedConversation", expect = "A0 E6 13 B6 00")]
pub fn speech_request_ped_conversation(ped1: u32, ped2: u32) -> u8 {
    if g::<u8>(0xB613E6).read() != 0 {
        return 0;
    }
    let (a, b): (GPtr<u8>, GPtr<u8>) = (g(ped1 as usize), g(ped2 as usize));
    if rd::<u8>(a, 0x32E) != 0
        || rd::<u8>(a, 0x32D) != 0
        || rd::<u8>(b, 0x32E) != 0
        || rd::<u8>(b, 0x32D) != 0
        || g::<u8>(CONV_PED_HAPPENING).read() != 0
        || g::<u8>(CONV_PLAYER_HAPPENING).read() != 0
    {
        return 0;
    }
    if vcall_speech_get_ped_type(a.byte_add(0x294)) as u16
        == vcall_speech_get_ped_type(b.byte_add(0x294)) as u16
    {
        return 0;
    }
    if ped_can_ped_hold_conversation(a) == 0
        || ped_can_ped_hold_conversation(b) == 0
        || ped_get_ped_talking(a) != 0
        || ped_get_ped_talking(b) != 0
    {
        return 0;
    }
    if too_far_from_camera(a) || too_far_from_camera(b) || speech_reserve_ped_conversation_speech_slots() == 0
    {
        return 0;
    }
    g::<u8>(CONV_PED_HAPPENING).write(1);
    g::<u32>(CONV_PED1).write(ped1);
    g::<u32>(CONV_PED2).write(ped2);
    speech_set_up_conversation_call();
    1
}

// ---- CAEPedSpeechAudioEntity: StopCurrentSpeech / Initialise --------------------------------------

orig! {
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, EAX)
    fn task_manager_get_task_secondary(tm: GPtr<u8>, slot: u32) -> u32 = thiscall 0x681810 => unported;
    /// `CTaskComplexFacial::StopAll()` (0x691250)
    fn task_facial_stop_all(task: GPtr<u8>) = thiscall 0x691250 => unported;
    /// `CAESound::StopSoundAndForget()` (0x4EF850)
    fn sound_stop_sound_and_forget(sound: GPtr<u8>) = thiscall 0x4EF850 => unported;
    /// `FUN_004e3350` (0x4E3350): the ped-speech slot initialiser
    fn speech_slot_init(slot: GPtr<u8>) -> u32 = thiscall 0x4E3350 => crate::port2_a::ped_speech_slot_init;
    /// `CAEPedSpeechAudioEntity::GetVoiceAndTypeForSpecialPed(uint32 modelHash)` (0x4E4170, AL)
    fn speech_get_voice_and_type_for_special_ped(this: GPtr<u8>, hash: u32) -> u8 = thiscall 0x4E4170 => crate::port2_b::speech_get_voice_and_type_for_special_ped;
    /// `CPedModelInfo::IncrementVoice()` (0x4C7300)
    fn ped_model_info_increment_voice(mi: GPtr<u8>) = thiscall 0x4C7300 => unported;
}

/// `CAEPedSpeechAudioEntity::s_PedSpeechSlots` (0xB61C38, 6 * 0x1C) entry `i` (int16 index).
fn speech_slot_at(i: u16) -> GPtr<u8> {
    g::<u8>(0xB61C38).byte_add(i as i16 as isize * 0x1C)
}

/// `CAEPedSpeechAudioEntity::StopCurrentSpeech()` (0x4E3FB0; its first instruction after the prologue
/// sits in a relocated `.text` chunk at 0x403773): nothing unless initialised (+0x90) and talking (+0x98).
/// The ped's secondary facial task (slot 3) stops all; the speech slot (+0xA8): status 5 -> the sound
/// (+0xA0) stops and is forgotten, +0xA0 = 0; slot byte +0x19 set -> the slot is cleared inline (same
/// fields as `FUN_004e3350`, byte order +0x19, +0x1A, +0x18) and gets +0x19 = 1, status 4; else +0x1A
/// set -> `FUN_004e3350`, +0x1A = 1, status 4; else `FUN_004e3350`. Then +0x98 = 0, the player
/// speaking flag 0xB613E5 cleared for ped type 2 (+0x92), slot = -1.
#[hook(addr = 0x4E3FB0, conv = thiscall, name = "CAEPedSpeechAudioEntity::StopCurrentSpeech", expect = "53 56 8B F1 90")]
pub fn speech_stop_current_speech(this: GPtr<u8>) {
    if rd::<u8>(this, 0x90) == 0 || rd::<u8>(this, 0x98) == 0 {
        return;
    }
    let ped: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    let intel = rd::<u32>(ped, 0x47C);
    if intel != 0 && task_manager_get_task_secondary(g::<u8>(intel as usize).byte_add(4), 3) != 0 {
        let intel = rd::<u32>(g(rd::<u32>(this, 4) as usize), 0x47C);
        let t = task_manager_get_task_secondary(g::<u8>(intel as usize).byte_add(4), 3);
        task_facial_stop_all(g(t as usize));
    }
    let slot = speech_slot_at(rd::<u16>(this, 0xA8));
    if rd::<u16>(slot, 0) == 5 {
        let snd = rd::<u32>(this, 0xA0);
        if snd != 0 {
            sound_stop_sound_and_forget(g(snd as usize));
        }
        wr::<u32>(this, 0xA0, 0);
    }
    let slot = speech_slot_at(rd::<u16>(this, 0xA8));
    if rd::<u8>(slot, 0x19) != 0 {
        wr::<u16>(slot, 0, 0);
        wr::<u32>(slot, 4, 0);
        wr::<u32>(slot, 8, 0);
        wr::<u16>(slot, 0xC, 0xFFFF);
        wr::<u16>(slot, 0xE, 0xFFFF);
        wr::<u32>(slot, 0x10, 0);
        wr::<u16>(slot, 0x14, 0xFFFF);
        wr::<u16>(slot, 0x16, 0xFFFF);
        wr::<u8>(slot, 0x19, 0);
        wr::<u8>(slot, 0x1A, 0);
        wr::<u8>(slot, 0x18, 0);
        let slot = speech_slot_at(rd::<u16>(this, 0xA8));
        wr::<u8>(slot, 0x19, 1);
        wr::<u16>(speech_slot_at(rd::<u16>(this, 0xA8)), 0, 4);
    } else if rd::<u8>(slot, 0x1A) != 0 {
        speech_slot_init(slot);
        wr::<u8>(speech_slot_at(rd::<u16>(this, 0xA8)), 0x1A, 1);
        wr::<u16>(speech_slot_at(rd::<u16>(this, 0xA8)), 0, 4);
    } else {
        speech_slot_init(slot);
    }
    wr::<u8>(this, 0x98, 0);
    if rd::<u16>(this, 0x92) == 2 {
        g::<u8>(0xB613E5).write(0);
    }
    wr::<u16>(this, 0xA8, 0xFFFF);
}

/// `CAEPedSpeechAudioEntity::Initialise(CEntity*)` (0x4E68D0, `ret 4`; one store sits in a relocated
/// `.text` chunk at 0x4038E0): entity +4, sound +0xA0 = 0, +0xAC = -100.0, +0xB0 = -1, +0x96 = 0, the 19
/// dwords +0xB4.. zeroed, bytes +0x98..+0x9C = 0. A model (+0x22, int16) >= 0 whose model info word +0x3C
/// (the audio ped type, stored at +0x92) is 0..5: type 5 needs `GetVoiceAndTypeForSpecialPed(model info
/// +4)`; the others take the voice +0x42 (stored at +0x94; negative: nothing more) and
/// `CPedModelInfo::IncrementVoice`. With the type (reread) still 0..5: +0x90 = 1 and, for types other
/// than 5, +0x96 = 1 for ped types (+0x28) 5 / 0x16, else 0.
#[hook(addr = 0x4E68D0, conv = thiscall, name = "CAEPedSpeechAudioEntity::Initialise", expect = "8B 44 24 04 53 33 DB")]
pub fn speech_initialise(this: GPtr<u8>, entity: u32) {
    wr::<u32>(this, 4, entity);
    wr::<u32>(this, 0xA0, 0);
    wr::<u32>(this, 0xAC, 0xC2C8_0000);
    wr::<u16>(this, 0xB0, 0xFFFF);
    wr::<u16>(this, 0x96, 0);
    for i in 0..0x13 {
        wr::<u32>(this, 0xB4 + 4 * i, 0);
    }
    for off in 0x98..=0x9C {
        wr::<u8>(this, off, 0);
    }
    let model = rd::<u16>(g(entity as usize), 0x22) as i16 as i32;
    if model < 0 {
        return;
    }
    let mi_at = |m: i32| -> GPtr<u8> { g(g::<u32>(0xA9B0C8 + 4 * m as usize).read() as usize) };
    let mi = mi_at(model);
    let ty = rd::<u16>(mi, 0x3C) as i16;
    wr::<u16>(this, 0x92, ty as u16);
    if !(0..6).contains(&ty) {
        return;
    }
    if ty == 5 {
        if speech_get_voice_and_type_for_special_ped(this, rd::<u32>(mi, 4)) == 0 {
            return;
        }
    } else {
        let voice = rd::<u16>(mi, 0x42) as i16;
        wr::<u16>(this, 0x94, voice as u16);
        if voice < 0 {
            return;
        }
        ped_model_info_increment_voice(mi);
    }
    let mi = mi_at(model);
    let ty = rd::<u16>(mi, 0x3C) as i16;
    if !(0..6).contains(&ty) {
        return;
    }
    if ty != 5 {
        let pt = rd::<u32>(mi, 0x28);
        wr::<u16>(this, 0x96, (pt == 5 || pt == 0x16) as u16);
    }
    wr::<u8>(this, 0x90, 1);
}

// ---- CAEGlobalWeaponAudioEntity::Constructor ------------------------------------------------------

orig! {
    /// CRT `operator new(uint32)` (0x82119A, cdecl)
    fn crt_operator_new(size: u32) -> u32 = cdecl 0x82119A => unported;
    /// `CPhysical::CPhysical()` (0x542260, returns this)
    fn physical_constructor(this: GPtr<u8>) -> u32 = thiscall 0x542260 => unported;
}

/// `CAEGlobalWeaponAudioEntity::CAEGlobalWeaponAudioEntity()` (0x5075B0): the `CAEAudioEntity` /
/// `CAEWeaponAudioEntity` fields zeroed in the bytes' order (+0x10, +0x84, +0x88, +0x8C, +0x9C, byte
/// +0x7E, +0x90, bytes +0x7C / +0x7D, +0x94, +0x98), vtable 0x862E5C, byte +0xA4 = 0, then the
/// ambient gun-fire physical: `operator new(0x138)` constructed as a `CPhysical` (null when the
/// allocation fails) at +0xA0. Returns `this`.
#[hook(addr = 0x5075B0, conv = thiscall, name = "CAEGlobalWeaponAudioEntity::Constructor", expect = "6A FF 68 23 C6 83 00")]
pub fn global_weapon_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x10, 0);
    for off in [0x84, 0x88, 0x8C, 0x9C] {
        wr::<u32>(this, off, 0);
    }
    wr::<u8>(this, 0x7E, 0);
    wr::<u32>(this, 0x90, 0);
    wr::<u8>(this, 0x7C, 0);
    wr::<u8>(this, 0x7D, 0);
    wr::<u32>(this, 0x94, 0);
    wr::<u32>(this, 0x98, 0);
    wr::<u32>(this, 0, 0x862E5C);
    wr::<u8>(this, 0xA4, 0);
    let mem = crt_operator_new(0x138);
    let phys = if mem != 0 {
        physical_constructor(g(mem as usize))
    } else {
        0
    };
    wr::<u32>(this, 0xA0, phys);
    this
}

// ---- CAEWaterCannonAudioEntity::UpdateGenericWaterCannonSound -------------------------------------

orig! {
    /// `CAESound::StopSound()` (0x4EF1C0)
    fn sound_stop_sound(sound: GPtr<u8>) = thiscall 0x4EF1C0 => crate::sound::stop_sound;
}

/// `CAEWaterCannonAudioEntity::UpdateGenericWaterCannonSound(bool play, int16 index, int16 slot, int16
/// sfx, float speed, float volume, CVector pos, float maxDistance)` (0x502F80, `ret 0x28`; one load
/// sits in a relocated `.text` chunk at 0x407E77). Not playing: the sound at +0x90 + 4 * index is
/// stopped (the pointer kept). Playing without a sound: the member sound (+8) is initialised (`slot`,
/// `sfx`, this, pos, volume, 1.0, 1.0, 1.0, 0, 0, 0, 0), then volume +0x1C, max distance +0x20 and
/// speed +0x24 are stored, flags word +0x5E = 4, and `RequestNewSound`'s result is kept; with a sound
/// its volume (+0x14) and speed (+0x1C) are set and `SetPosition(pos)`.
#[hook(addr = 0x502F80, conv = thiscall, name = "CAEWaterCannonAudioEntity::UpdateGenericWaterCannonSound", expect = "8A 44 24 04 53 56 84 C0")]
#[allow(clippy::too_many_arguments)]
pub fn water_cannon_update_generic_sound(
    this: GPtr<u8>,
    play: u32,
    index: u32,
    slot: u32,
    sfx: u32,
    speed: u32,
    volume: u32,
    px: u32,
    py: u32,
    pz: u32,
    max_distance: u32,
) {
    let cell = this.byte_add(0x90 + 4 * (index as u16 as i16 as isize));
    let snd = rd::<u32>(cell, 0);
    if play as u8 == 0 {
        if snd != 0 {
            sound_stop_sound(g(snd as usize));
        }
        return;
    }
    if snd == 0 {
        let s = this.byte_add(8);
        sound_initialise(
            s,
            slot,
            sfx,
            this.addr() as u32,
            px,
            py,
            pz,
            volume,
            0x3F80_0000,
            0x3F80_0000,
            0x3F80_0000,
            0,
            0,
            0,
            0,
        );
        wr::<u32>(this, 0x1C, volume);
        wr::<u32>(this, 0x20, max_distance);
        wr::<u32>(this, 0x24, speed);
        wr::<u16>(this, 0x5E, 4);
        let r = sm_request_new_sound(g(SM), s);
        wr::<u32>(cell, 0, r);
    } else {
        let sp: GPtr<u8> = g(snd as usize);
        wr::<u32>(sp, 0x14, volume);
        wr::<u32>(g(rd::<u32>(cell, 0) as usize), 0x1C, speed);
        sound_set_position(g(rd::<u32>(cell, 0) as usize), px, py, pz);
    }
}

// ---- CAETwinLoopSoundEntity::PlayTwinLoopSound ----------------------------------------------------

/// `CTimer::m_snTimeInMillisecondsNonClipped`
const TIME_MS_NON_CLIPPED: usize = 0xB7CB7C;

/// A twin-loop play position: the word at `off` unless -1, then `GetRandomNumberInRange(0, 99)`.
fn twin_play_pos(this: GPtr<u8>, off: isize) -> u16 {
    let w = rd::<u16>(this, off);
    if w == 0xFFFF {
        ae_random_int(0, 99) as u16
    } else {
        w
    }
}

/// `CAETwinLoopSoundEntity::PlayTwinLoopSound(CVector pos, float volume, float speed, float maxDistance,
/// float timeScale, eSoundEnvironment flags)` (0x4F2CB0, `ret 0x20`). The entry is `push -1; jmp
/// 0x40A47D`: that `.text` chunk only pushes the SEH handler, computed as `ror([0x1560920],
/// [0x45BA91])`, and jumps back; the port has no SEH frame. Both sounds (+0xA0, +0xA4) are stopped and
/// forgotten (pointers kept until overwritten); sound 1: `Initialise(slot +0x7C, sfx +0x7E, this, pos,
/// volume, maxDistance, speed, timeScale, 0, flags | 0x24, 0, 0)`, play position = word +0x9A (-1: a
/// random 0..99) -> +0xA0; sound 2: sfx +0x80, volume -100.0, play position +0x9C -> +0xA4. Next switch
/// time +0x94 = `rand(+0x8E, +0x90)` + non-clipped time; playing flag +0x98 = 1.
#[hook(addr = 0x4F2CB0, conv = thiscall, name = "CAETwinLoopSoundEntity::PlayTwinLoopSound", expect = "6A FF E9 C6 77 F1 FF")]
#[allow(clippy::too_many_arguments)]
pub fn twin_play_twin_loop_sound(
    this: GPtr<u8>,
    px: u32,
    py: u32,
    pz: u32,
    volume: u32,
    speed: u32,
    max_distance: u32,
    time_scale: u32,
    flags: u32,
) {
    let mut local = StackSound::new();
    for off in [0xA0, 0xA4] {
        let s = rd::<u32>(this, off);
        if s != 0 {
            sound_stop_sound_and_forget(g(s as usize));
        }
    }
    let flags = flags | 0x24;
    let ent = this.addr() as u32;
    let pos1 = twin_play_pos(this, 0x9A);
    let sp = local.ptr();
    sound_initialise(
        sp,
        rd::<u16>(this, 0x7C) as u32,
        rd::<u16>(this, 0x7E) as u32,
        ent,
        px,
        py,
        pz,
        volume,
        max_distance,
        speed,
        time_scale,
        0,
        flags,
        0,
        0,
    );
    wr::<u16>(sp, 0x5C, pos1);
    let s1 = sm_request_new_sound(g(SM), sp);
    wr::<u32>(this, 0xA0, s1);
    let pos2 = twin_play_pos(this, 0x9C);
    sound_initialise(
        sp,
        rd::<u16>(this, 0x7C) as u32,
        rd::<u16>(this, 0x80) as u32,
        ent,
        px,
        py,
        pz,
        0xC2C8_0000,
        max_distance,
        speed,
        time_scale,
        0,
        flags,
        0,
        0,
    );
    wr::<u16>(sp, 0x5C, pos2);
    let s2 = sm_request_new_sound(g(SM), sp);
    wr::<u32>(this, 0xA4, s2);
    let now = g::<u32>(TIME_MS_NON_CLIPPED).read();
    let r = ae_random_int(rd::<u16>(this, 0x8E) as i32, rd::<u16>(this, 0x90) as i32);
    wr::<u32>(this, 0x94, (r as u32).wrapping_add(now));
    wr::<u8>(this, 0x98, 1);
    sound_destructor(local.ptr());
}

// ---- CAEPedSpeechAudioEntity::IsCJDressedInForGangSpeech ------------------------------------------

orig! {
    /// `FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> u32 = cdecl 0x56E210 => unported;
    /// `CTheZones::GetLevelFromPosition(CVector)` (0x441B70, cdecl, the vector by value)
    fn zones_get_level_from_position(x: u32, y: u32, z: u32) -> u32 = cdecl 0x441B70 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30, cdecl)
    fn key_gen_get_uppercase_key(name: u32) -> u32 = cdecl 0x53CF30 => unported;
}

/// `CAEPedSpeechAudioEntity::IsCJDressedInForGangSpeech()` (0x4E4270, cdecl, AL): false without player
/// 0 or outside level 1 (`GetLevelFromPosition` of the player's position); otherwise true when one of
/// the clothes-description keys (`[[player+0x480]+4]` +0x28, +0x30, +0x68, +0x64) equals the uppercase
/// key of a gang item name, tested in the bytes' order (+0x28: 0x85F304, 0x85F2F8; +0x30: 0x85F2EC,
/// 0x85F2E0; +0x68: 0x85F2D4 .. 0x85F294 (7); +0x64: 0x85F288, 0x85F27C, 0x85F270, 0x85F264).
#[hook(addr = 0x4E4270, conv = cdecl, name = "CAEPedSpeechAudioEntity::IsCJDressedInForGangSpeech", expect = "53 55 56 57 6A 00")]
pub fn speech_is_cj_dressed_in_for_gang_speech() -> u8 {
    let p = find_player_ped(0);
    if p == 0 {
        return 0;
    }
    let pp: GPtr<u8> = g(p as usize);
    let clothes: GPtr<u8> = g(rd::<u32>(g(rd::<u32>(pp, 0x480) as usize), 4) as usize);
    let keys = [
        rd::<u32>(clothes, 0x28),
        rd::<u32>(clothes, 0x30),
        rd::<u32>(clothes, 0x68),
        rd::<u32>(clothes, 0x64),
    ];
    let pos = entity_position(pp);
    if zones_get_level_from_position(rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)) != 1 {
        return 0;
    }
    const NAMES: [(usize, u32); 15] = [
        (0, 0x85F304),
        (0, 0x85F2F8),
        (1, 0x85F2EC),
        (1, 0x85F2E0),
        (2, 0x85F2D4),
        (2, 0x85F2CC),
        (2, 0x85F2C4),
        (2, 0x85F2B8),
        (2, 0x85F2AC),
        (2, 0x85F2A0),
        (2, 0x85F294),
        (3, 0x85F288),
        (3, 0x85F27C),
        (3, 0x85F270),
        (3, 0x85F264),
    ];
    for (k, name) in NAMES {
        if keys[k] == key_gen_get_uppercase_key(name) {
            return 1;
        }
    }
    0
}

// ---- CAECollisionAudioEntity::PlayBulletHitCollisionSound -----------------------------------------

orig! {
    /// `SurfaceInfos_c::IsAudioWood / IsAudioConcrete / IsAudioGravel / IsAudioTile(uint32)` (AL)
    fn surface_is_audio_wood(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EAB0 => unported;
    fn surface_is_audio_concrete(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EA30 => unported;
    fn surface_is_audio_gravel(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EA90 => unported;
    fn surface_is_audio_tile(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EB30 => unported;
}

/// `GetRandomNumberInRange(min, max)` redrawn while it equals the last bullet-hit id (+0x202, int16).
fn bullet_hit_id(this: GPtr<u8>, min: i32, max: i32) -> u32 {
    loop {
        let r = ae_random_int(min, max) as u32;
        if r as u16 != rd::<u16>(this, 0x202) {
            return r;
        }
    }
}

/// `CAECollisionAudioEntity::PlayBulletHitCollisionSound(eSurfaceType, CVector& posn, float angle)`
/// (0x4DB7C0, `ret 0xC`; two loads sit in a relocated `.text` chunk at 0x4040C0): nothing for surfaces
/// >= 0xC3. Volume = the volume offset byte `[[0xBD00F8]+0x75]`, max distance 1.5; id (never the last one,
/// +0x202): 0x3E -> 7..9; water -> 16..18 (max distance 2.0, volume + [0x858B44]); wood -> 19..21;
/// metal -> `ResolveProbability(([0x85991C] - angle) * [0x85F0AC])` ? 10..12 : 4..6; concrete, gravel or
/// tile -> 13..15; else 1..3. `Initialise(slot 3, id, this, *posn, volume, maxDistance, 1.0, 1.0, 0, 0,
/// 0.02, 0)`, `RequestNewSound`, +0x202 = id.
#[hook(addr = 0x4DB7C0, conv = thiscall, name = "CAECollisionAudioEntity::PlayBulletHitCollisionSound", expect = "6A FF 68 98 BE 83 00")]
pub fn collision_play_bullet_hit(this: GPtr<u8>, surface: u32, posn: u32, angle: u32) {
    let mut local = StackSound::new();
    let s = surface as u8;
    if s < 0xC3 {
        let mut vol = volume_offset_at(0x75) as f32;
        let mut max_distance = 0x3FC0_0000u32;
        let si = g::<u8>(SURFACE_INFOS);
        let su = s as u32;
        let id = if s == 0x3E {
            bullet_hit_id(this, 7, 9)
        } else if surface_is_audio_water(si, su) != 0 {
            let id = bullet_hit_id(this, 0x10, 0x12);
            max_distance = 0x4000_0000;
            vol = f32_of(x(vol).fadd_m(c(0x858B44)));
            id
        } else if surface_is_audio_wood(si, su) != 0 {
            bullet_hit_id(this, 0x13, 0x15)
        } else if surface_is_audio_metal(si, su) != 0 {
            let p = f32_of(x(c(0x85991C)).fsub_m(f32::from_bits(angle)).fmul_m(c(0x85F0AC)));
            if ae_resolve_probability(p) != 0 {
                bullet_hit_id(this, 0xA, 0xC)
            } else {
                bullet_hit_id(this, 4, 6)
            }
        } else if surface_is_audio_concrete(si, su) != 0
            || surface_is_audio_gravel(si, su) != 0
            || surface_is_audio_tile(si, su) != 0
        {
            bullet_hit_id(this, 0xD, 0xF)
        } else {
            bullet_hit_id(this, 1, 3)
        };
        if id as u16 as i16 >= 0 {
            let pv: GPtr<u8> = g(posn as usize);
            let sp = local.ptr();
            sound_initialise(
                sp,
                3,
                id,
                this.addr() as u32,
                rd::<u32>(pv, 0),
                rd::<u32>(pv, 4),
                rd::<u32>(pv, 8),
                vol.to_bits(),
                max_distance,
                0x3F80_0000,
                0x3F80_0000,
                0,
                0,
                0x3CA3_D70A,
                0,
            );
            sm_request_new_sound(g(SM), sp);
            wr::<u16>(this, 0x202, id as u16);
        }
    }
    sound_destructor(local.ptr());
}

/// The volume offset byte at `[[0xBD00F8] + off]` (int8).
fn volume_offset_at(off: isize) -> i32 {
    rd::<i8>(g(g::<u32>(VOLUME_OFFSET_PTR).read() as usize), off) as i32
}

// ---- CAECollisionAudioEntity::ReportWaterSplash(CPhysical*, float, bool) --------------------------

orig! {
    /// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntityAndPhysical(int16, CAEAudioEntity*, CPhysical*)` (0x4EF5D0, AX)
    fn sm_are_sounds_playing_for_entity_and_physical(sm: GPtr<u8>, event: u32, entity: u32, physical: u32) -> u32 = thiscall 0x4EF5D0 => unported;
    /// `CAudioEngine::IsLoadingTuneActive()` (0x506D90, AL)
    fn engine_is_loading_tune_active(engine: GPtr<u8>) -> u8 = thiscall 0x506D90 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070, EAX)
    fn intelligence_get_task_swim(intel: GPtr<u8>) -> u32 = thiscall 0x601070 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude_st0(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CAESound::RegisterWithPhysicalEntity(CEntity*)` (0x4EF820)
    fn sound_register_with_physical_entity(sound: GPtr<u8>, physical: u32) = thiscall 0x4EF820 => unported;
}

/// The splash volume term (0x4DAFB2..0x4DB04D): `|move speed|` (inline, (x² + y²) + z²) below
/// [0x858CC8] -> `CVector::Magnitude(move speed)`, else [0x858CC8]; `lg2 log2(that * [0x85F0A8]) *
/// [0x858BA4]`; kept when above [0x85F0A4] (the bytes compute it a second time, identically), else
/// [0x85F0A4].
fn splash_volume_term(phys: GPtr<u8>) -> X87 {
    let term = || {
        let inline = magnitude_at(phys, 0x44);
        let m = if !inline.fcom(x(c(0x858CC8))).test_parity(0x05) {
            X87::from_f64_exact(vector_magnitude_st0(phys.byte_add(0x44)))
        } else {
            x(c(0x858CC8))
        };
        X87::lg2().fyl2x(m.fmul_m(c(0x85F0A8))).fmul_m(c(0x858BA4))
    };
    if !term().fcom(x(c(0x85F0A4))).test_any(0x41) {
        term()
    } else {
        x(c(0x85F0A4))
    }
}

/// `CAECollisionAudioEntity::ReportWaterSplash(CPhysical*, float volume, bool forcePlay)` (0x4DAE40,
/// `ret 0xC`). Without `forcePlay` (or when the physical rises faster than [0x858EF4] and the volume
/// is not exactly [0x859014] -- then without `forcePlay` nothing at all) any splash event 1..3 playing
/// for this entity and physical blocks it. Bank 0x27 in slot 2, else (no loading tune) it is loaded and
/// nothing plays. Base volume = the offset byte `[[0xBD00F8]+0x76]`: peds (type 3) need `forcePlay` and
/// no swim task, base - [0x858B44]; vehicles: speed 0.94, event 3, time + 0xF8; others base -
/// [0x858CCC]; peds and others: speed 1.26, event 2, time + 0xA6. Volume = base + `volume` when that is
/// above [0x859014], else base + the splash term. Two sounds through the member sound (+8): id 0x43
/// (that speed, event 1) and id 0x42 (speed 0.0, the event above, client value = the time as an
/// unsigned float), both slot 2, max distance 2.5, flags 0x84, registered with the physical.
#[hook(addr = 0x4DAE40, conv = thiscall, name = "CAECollisionAudioEntity::ReportWaterSplash-for-physical", expect = "83 EC 08 53 55")]
pub fn collision_report_water_splash_physical(this: GPtr<u8>, physical: u32, volume: u32, force: u32) {
    let phys: GPtr<u8> = g(physical as usize);
    let force = force as u8 != 0;
    let vol_arg = f32::from_bits(volume);
    let rising = !x(rd::<f32>(phys, 0x4C)).fcom(x(c(0x858EF4))).test_any(0x41);
    let check_playing = if rising && !x(vol_arg).fcom(x(c(0x859014))).test_parity(0x44) {
        if !force {
            return;
        }
        false
    } else {
        !force
    };
    let ent = this.addr() as u32;
    if check_playing {
        for ev in 1..=3 {
            if sm_are_sounds_playing_for_entity_and_physical(g(SM), ev, ent, physical) as u16 != 0 {
                return;
            }
        }
    }
    if hw_is_sound_bank_loaded(g(HW), 0x27, 2) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(HW), 0x27, 2);
        }
        return;
    }
    let mut base = volume_offset_at(0x76) as f32;
    let mut time = g::<u32>(TIME_MS).read();
    let ty = rd::<u8>(phys, 0x36) & 7;
    let (speed, event) = if ty == 2 {
        time = time.wrapping_add(0xF8);
        (0x3F70_A3D7u32, 3u32)
    } else {
        if ty == 3 {
            if !force || intelligence_get_task_swim(g(rd::<u32>(phys, 0x47C) as usize)) != 0 {
                return;
            }
            base = f32_of(x(base).fsub_m(c(0x858B44)));
        } else {
            base = f32_of(x(base).fsub_m(c(0x858CCC)));
        }
        time = time.wrapping_add(0xA6);
        (0x3FA1_47AE, 2)
    };
    let v = if !x(vol_arg).fcom(x(c(0x859014))).test_any(0x41) {
        x(base).fadd_m(vol_arg)
    } else {
        splash_volume_term(phys).fadd_m(base)
    };
    let vol = f32_of(v).to_bits();
    let s = this.byte_add(8);
    let p = entity_position(phys);
    sound_initialise(
        s,
        2,
        0x43,
        ent,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        vol,
        0x4020_0000,
        speed,
        0x3F80_0000,
        0,
        0x84,
        0,
        0,
    );
    sound_register_with_physical_entity(s, physical);
    wr::<u32>(this, 0x14, 1);
    sm_request_new_sound(g(SM), s);
    let p = entity_position(phys);
    sound_initialise(
        s,
        2,
        0x42,
        ent,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        vol,
        0x4020_0000,
        0,
        0x3F80_0000,
        0,
        0x84,
        0,
        0,
    );
    sound_register_with_physical_entity(s, physical);
    wr::<u32>(this, 0x14, event);
    let mut t = xi(time as i32);
    if (time as i32) < 0 {
        t = t.fadd_m(c(0x858C54));
    }
    wr::<f32>(this, 0x18, f32_of(t));
    sm_request_new_sound(g(SM), s);
}

// ---- CAEPedAudioEntity::HandlePedJacked -----------------------------------------------------------

/// `(float)(uint32)v` as the bytes convert it: `fild` of the int32, `+ 4294967296.0` ([0x858C54]) when
/// negative, stored as f32.
fn u32_to_f32(v: u32) -> f32 {
    let t = xi(v as i32);
    f32_of(if (v as i32) < 0 { t.fadd_m(c(0x858C54)) } else { t })
}

/// One ped-audio sound through the member sound (+8): `Initialise(slot, id, this, ped position,
/// volume, 2.0, 0.0, 1.0, 0, 0x84, 0, 0)`, registered with the ped, event +0x14 = `event`, client value
/// +0x18 = `(float)(uint32)time`, `RequestNewSound`.
fn ped_event_sound(this: GPtr<u8>, slot: u32, id: u32, volume: f32, event: u32, time: u32) {
    let ped = rd::<u32>(this, 0x94);
    let p = entity_position(g(ped as usize));
    let s = this.byte_add(8);
    sound_initialise(
        s,
        slot,
        id,
        this.addr() as u32,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        volume.to_bits(),
        0x4000_0000,
        0,
        0x3F80_0000,
        0,
        0x84,
        0,
        0,
    );
    sound_register_with_physical_entity(s, rd::<u32>(this, 0x94));
    wr::<u32>(this, 0x14, event);
    wr::<f32>(this, 0x18, u32_to_f32(time));
    sm_request_new_sound(g(SM), s);
}

/// `CAEPedAudioEntity::HandlePedJacked(eAudioEvents event)` (0x4E2350, `ret 4`): bank 0x8F in slot 5
/// (else, without a loading tune, it is loaded and nothing plays). Volume = the offset byte
/// `[[0xBD00F8] + event]`. Events 0x45..0x49 play up to two slot-5 sounds whose client value is now +
/// (0x1F4, 0x3A5, 0x384, 0x21, 0x10A): a fixed id first (0x3A, 0x32, -, 0x3A, -), then a random one
/// (`rand(0x28, 0x2B)` / `rand(0x24, 0x27)` / 0x28..0x2B / 0x28..0x2B / 0x24..0x27, drawn first); the
/// bytes then fall through (no `ret` after 0x4E25BB) into the block every other event takes: bank 0 in
/// slot 0x29 -> one sound (slot 0x29, id 0, client value = now, or now + (0x855, 0xA49, 0xAF0, 0x16E,
/// 0x341) after a case: the argument slot the cases overwrite), else that bank is loaded.
#[hook(addr = 0x4E2350, conv = thiscall, name = "CAEPedAudioEntity::HandlePedJacked", expect = "83 EC 0C 53 55")]
pub fn ped_handle_ped_jacked(this: GPtr<u8>, event: u32) {
    if hw_is_sound_bank_loaded(g(HW), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(HW), 0x8F, 5);
        }
        return;
    }
    let vol = volume_offset_at(event as i32 as isize) as f32;
    let now = g::<u32>(TIME_MS).read();
    // (fixed id, random range, client time offset, offset of the trailing slot-0x29 sound)
    let case = match event.wrapping_sub(0x45) {
        0 => Some((Some(0x3A), (0x28, 0x2B), 0x1F4, 0x855)),
        1 => Some((Some(0x32), (0x24, 0x27), 0x3A5, 0xA49)),
        2 => Some((None, (0x28, 0x2B), 0x384, 0xAF0)),
        3 => Some((Some(0x3A), (0x28, 0x2B), 0x21, 0x16E)),
        4 => Some((None, (0x24, 0x27), 0x10A, 0x341)),
        _ => None,
    };
    let mut last_time = now;
    if let Some((fixed, (lo, hi), off, off2)) = case {
        let id = ae_random_int(lo, hi) as u32;
        let t = now.wrapping_add(off);
        last_time = now.wrapping_add(off2);
        if let Some(f) = fixed {
            ped_event_sound(this, 5, f, vol, event, t);
        }
        if id as u16 as i16 >= 0 {
            ped_event_sound(this, 5, id, vol, event, t);
        }
    }
    if hw_is_sound_bank_loaded(g(HW), 0, 0x29) != 0 {
        ped_event_sound(this, 0x29, 0, vol, event, last_time);
    } else {
        hw_load_sound_bank(g(HW), 0, 0x29);
    }
}

// ---- CAEWeaponAudioEntity::ReportStealthKill ------------------------------------------------------

/// Bank `bank` in `slot`, else (without a loading tune) it is loaded; false then.
fn bank_ready_or_load(bank: u32, slot: u32) -> bool {
    if hw_is_sound_bank_loaded(g(HW), bank, slot) != 0 {
        return true;
    }
    if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
        hw_load_sound_bank(g(HW), bank, slot);
    }
    false
}

/// One weapon-entity sound through the member sound (+8): `Initialise(slot, id, this, physical
/// position, volume, 1.0, 0.0, 1.0, 0, 0x84, 0, 0)`, registered with the physical, event +0x14,
/// client value +0x18 = `(float)(uint32)` now, `RequestNewSound`.
fn weapon_physical_sound(this: GPtr<u8>, physical: u32, slot: u32, id: u32, volume: f32, event: u32) {
    let p = entity_position(g(physical as usize));
    let s = this.byte_add(8);
    sound_initialise(
        s,
        slot,
        id,
        this.addr() as u32,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        volume.to_bits(),
        0x3F80_0000,
        0,
        0x3F80_0000,
        0,
        0x84,
        0,
        0,
    );
    sound_register_with_physical_entity(s, physical);
    wr::<u32>(this, 0x14, event);
    wr::<f32>(this, 0x18, u32_to_f32(g::<u32>(TIME_MS).read()));
    sm_request_new_sound(g(SM), s);
}

/// `CAEWeaponAudioEntity::ReportStealthKill(eWeaponType, CPhysical*, eAudioEvents)` (0x503B20, `ret
/// 0xC`): knife (type 4) only. Bank 0x8F in slot 5 (or loaded): id 0x51 in slot 5 at volume `offset[event]
/// - [0x858B44]` (event 0xD); then bank 0x27 in slot 2 (or loaded): id 0x2F in slot 2 at volume
/// `offset[event]` (event 0xE).
#[hook(addr = 0x503B20, conv = thiscall, name = "CAEWeaponAudioEntity::ReportStealthKill", expect = "83 7C 24 04 04 53")]
pub fn weapon_report_stealth_kill(this: GPtr<u8>, ty: u32, physical: u32, event: u32) {
    if ty != 4 || !bank_ready_or_load(0x8F, 5) {
        return;
    }
    let vol = volume_offset_at(event as i32 as isize) as f32;
    weapon_physical_sound(this, physical, 5, 0x51, f32_of(x(vol).fsub_m(c(0x858B44))), 0xD);
    if !bank_ready_or_load(0x27, 2) {
        return;
    }
    weapon_physical_sound(this, physical, 2, 0x2F, vol, 0xE);
}

// ---- CAEWeaponAudioEntity::ReportChainsawEvent ----------------------------------------------------

orig! {
    /// `CAESoundManager::AreSoundsPlayingInBankSlot(int16)` (0x4EF520, AX)
    fn sm_are_sounds_playing_in_bank_slot(sm: GPtr<u8>, slot: u32) -> u32 = thiscall 0x4EF520 => unported;
    /// `CAESoundManager::CancelSoundsInBankSlot(int16, bool)` (0x4EFC60)
    fn sm_cancel_sounds_in_bank_slot(sm: GPtr<u8>, slot: u32, b: u32) = thiscall 0x4EFC60 => unported;
}

/// `CAEWeaponAudioEntity::ReportChainsawEvent(CPhysical*, eAudioEvents)` (0x503910, `ret 8`): bank 0x24
/// in slot 0x28, else sounds playing in that slot are cancelled and the bank is loaded (a tail call
/// that rewrites the argument slots). Volume = `offset[event]`. State byte +0x80, time +0x94 = now on
/// every handled transition: 0x99 (states 4 / 0) -> unless event 9 plays for this entity, id 1 (slot
/// 0x28, max distance 0.66, speed 1.0, time scale 0.0, flags 0x84, event 9), state 0; 0x9A (states 3 /
/// 0 / 1) -> unless event 10 plays, id 0 (max distance 1.25, event 10), state 1; 0x9B (states 1 / 2)
/// -> state 2.
#[hook(addr = 0x503910, conv = thiscall, name = "CAEWeaponAudioEntity::ReportChainsawEvent", expect = "53 56 57 6A 28")]
pub fn weapon_report_chainsaw_event(this: GPtr<u8>, physical: u32, event: u32) {
    if hw_is_sound_bank_loaded(g(HW), 0x24, 0x28) == 0 {
        if sm_are_sounds_playing_in_bank_slot(g(SM), 0x28) as u16 != 0 {
            sm_cancel_sounds_in_bank_slot(g(SM), 0x28, 0);
        }
        hw_load_sound_bank(g(HW), 0x24, 0x28);
        return;
    }
    let vol = volume_offset_at(event as i32 as isize) as f32;
    let state = rd::<u8>(this, 0x80);
    let ent = this.addr() as u32;
    let play = |id: u32, max_distance: u32, ev: u32| {
        if sm_are_sounds_of_this_event_playing(g(SM), ev, ent) as u16 == 0 {
            let p = entity_position(g(physical as usize));
            let s = this.byte_add(8);
            sound_initialise(
                s,
                0x28,
                id,
                ent,
                rd::<u32>(p, 0),
                rd::<u32>(p, 4),
                rd::<u32>(p, 8),
                vol.to_bits(),
                max_distance,
                0x3F80_0000,
                0,
                0,
                0x84,
                0,
                0,
            );
            sound_register_with_physical_entity(s, physical);
            wr::<u32>(this, 0x14, ev);
            sm_request_new_sound(g(SM), s);
        }
    };
    let next = match event.wrapping_sub(0x99) {
        0 if state == 4 || state == 0 => {
            play(1, 0x3F28_F5C3, 9);
            0
        }
        1 if state == 3 || state == 0 || state == 1 => {
            play(0, 0x3FA0_0000, 10);
            1
        }
        2 if state == 1 || state == 2 => 2,
        _ => return,
    };
    wr::<u8>(this, 0x80, next);
    wr::<u32>(this, 0x94, g::<u32>(TIME_MS).read());
}

// ---- destructors ----------------------------------------------------------------------------------

orig! {
    /// `CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity()` (0x4F2B80)
    fn twin_destructor(twin: GPtr<u8>) = thiscall 0x4F2B80 => crate::entities::twin_loop_destructor;
}

/// `CAEAudioEntity`'s vtable (0x859988), stored last by every audio-entity destructor.
const AUDIO_ENTITY_VTABLE: u32 = 0x859988;

/// `CAEPedAudioEntity::~CAEPedAudioEntity()` (0x5DE920; `push -1; jmp 0x4084CA`: the chunk only pushes
/// the SEH handler, `[0x15608D0] + [0x7034B0]`): vtable 0x86C2A8, the twin loop (+0xA8) destroyed,
/// vtable 0x859988, the entity sound (+8) destroyed.
/// Not hooked (protection): 0x402195 `adc ebx, [0x5DE923]` reads the entry bytes as data (sa-hook INF-20 / keycells tests); reached through `orig!` only.
pub fn ped_audio_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C2A8);
    twin_destructor(this.byte_add(0xA8));
    wr::<u32>(this, 0, AUDIO_ENTITY_VTABLE);
    sound_destructor(this.byte_add(8));
}

/// `FUN_00506B20` = `CAEFrontendAudioEntity::~CAEFrontendAudioEntity()` (0x506B20, vtable 0x862E54 per
/// `catalog/data-tables.csv`; called by `CAudioEngine::~CAudioEngine`; `push -1; jmp 0x401533`: the
/// chunk only pushes the SEH handler, `[0x1560648] ^ [0x6541B2]`): vtable 0x862E54, the twin loops
/// +0x144 and +0x9C destroyed, vtable 0x859988, the entity sound (+8) destroyed.
/// Not hooked (protection): 0x4021F5 `adc eax, [0x506B22]` reads the entry bytes as data (sa-hook INF-20 / keycells tests); reached through `orig!` only.
pub fn frontend_audio_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x862E54);
    twin_destructor(this.byte_add(0x144));
    twin_destructor(this.byte_add(0x9C));
    wr::<u32>(this, 0, AUDIO_ENTITY_VTABLE);
    sound_destructor(this.byte_add(8));
}

/// `FUN_00506BA0` = `CAEScriptAudioEntity::~CAEScriptAudioEntity()` (0x506BA0, vtable 0x862E58; called
/// by `CAudioEngine::~CAudioEngine`): vtable 0x862E58, the two embedded audio entities at +0x19C and
/// +0x114 get vtable 0x859988 and their sounds (+0x1A4, +0x11C) destroyed, then vtable 0x859988 and
/// the entity sound (+8). Store order as the bytes (+0x19C vtable before its sound, +0x114 likewise).
#[hook(addr = 0x506BA0, conv = thiscall, name = "FUN_00506ba0", expect = "6A FF 68 96 C5 83 00")]
pub fn script_audio_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x862E58);
    wr::<u32>(this, 0x19C, AUDIO_ENTITY_VTABLE);
    sound_destructor(this.byte_add(0x1A4));
    wr::<u32>(this, 0x114, AUDIO_ENTITY_VTABLE);
    sound_destructor(this.byte_add(0x11C));
    wr::<u32>(this, 0, AUDIO_ENTITY_VTABLE);
    sound_destructor(this.byte_add(8));
}

// ---- CAEPedAudioEntity::TurnOnJetPack -------------------------------------------------------------

/// `CAEPedAudioEntity::TurnOnJetPack()` (0x4E28A0): with a ped, not on yet (+0x98) and none of the three
/// jet-pack sounds (+0x9C, +0xA0, +0xA4) set: the four jet-pack parameters (+0x88 = [0x8AE5F0], +0x84 =
/// [0x8AE600], on = 1, +0x8C = [0x8AE610], +0x90 = [0x8AE618]) and three looping sounds through the
/// member sound (+8) at the ped's position, volume -100, 1.0, 1.0, 1.0, 0, 0, 0, 0, speed +0x24 = 1.0 and
/// flags word +0x5E = 4 after `Initialise`: (slot 0x13, id 0x1A) -> +0x9C, (5, 0xA) -> +0xA0, (0, 0) ->
/// +0xA4.
#[hook(addr = 0x4E28A0, conv = thiscall, name = "CAEPedAudioEntity::TurnOnJetPack", expect = "56 8B F1 8B 8E 94 00 00 00")]
pub fn ped_turn_on_jet_pack(this: GPtr<u8>) {
    if rd::<u32>(this, 0x94) == 0
        || rd::<u8>(this, 0x98) != 0
        || rd::<u32>(this, 0xA0) != 0
        || rd::<u32>(this, 0x9C) != 0
        || rd::<u32>(this, 0xA4) != 0
    {
        return;
    }
    wr::<u32>(this, 0x88, g::<u32>(0x8AE5F0).read());
    wr::<u32>(this, 0x84, g::<u32>(0x8AE600).read());
    wr::<u8>(this, 0x98, 1);
    wr::<u32>(this, 0x8C, g::<u32>(0x8AE610).read());
    wr::<u32>(this, 0x90, g::<u32>(0x8AE618).read());
    for (slot, id, off) in [(0x13u32, 0x1Au32, 0x9Cisize), (5, 0xA, 0xA0), (0, 0, 0xA4)] {
        let p = entity_position(g(rd::<u32>(this, 0x94) as usize));
        let s = this.byte_add(8);
        sound_initialise(
            s,
            slot,
            id,
            this.addr() as u32,
            rd::<u32>(p, 0),
            rd::<u32>(p, 4),
            rd::<u32>(p, 8),
            0xC2C8_0000,
            0x3F80_0000,
            0x3F80_0000,
            0x3F80_0000,
            0,
            0,
            0,
            0,
        );
        wr::<u32>(this, 0x24, 0x3F80_0000);
        wr::<u16>(this, 0x5E, 4);
        let r = sm_request_new_sound(g(SM), s);
        wr::<u32>(this, off, r);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PED_TURN_ON_JET_PACK,
    HOOK_SCRIPT_AUDIO_DESTRUCTOR,
    HOOK_WEAPON_REPORT_CHAINSAW_EVENT,
    HOOK_WEAPON_REPORT_STEALTH_KILL,
    HOOK_PED_HANDLE_PED_JACKED,
    HOOK_COLLISION_REPORT_WATER_SPLASH_PHYSICAL,
    HOOK_COLLISION_PLAY_BULLET_HIT,
    HOOK_SPEECH_IS_CJ_DRESSED_IN_FOR_GANG_SPEECH,
    HOOK_TWIN_PLAY_TWIN_LOOP_SOUND,
    HOOK_WATER_CANNON_UPDATE_GENERIC_SOUND,
    HOOK_GLOBAL_WEAPON_CONSTRUCTOR,
    HOOK_SPEECH_STOP_CURRENT_SPEECH,
    HOOK_SPEECH_INITIALISE,
    HOOK_SPEECH_SET_UP_CONVERSATION,
    HOOK_SPEECH_REQUEST_PED_CONVERSATION,
    HOOK_COLLISION_SURFACE_VOLUME,
    HOOK_COLLISION_UPDATE_LOOPING,
    HOOK_COLLISION_PLAY_LOOPING,
    HOOK_COLLISION_PLAY_ONE_SHOT,
    HOOK_COLLISION_REPORT_COLLISION,
    HOOK_PED_PLAY_SHIRT_FLAP,
    HOOK_PED_SERVICE,
    HOOK_WEATHER_SERVICE,
];
