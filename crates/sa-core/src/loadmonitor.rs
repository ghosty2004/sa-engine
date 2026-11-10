//! `CLoadMonitor` (0x53CFA0–0x53D480, destructor 0x856430): per-frame load statistics that set the
//! processing level (`g_LoadMonitor` at 0xB72978).
//!
//! Layout (0x2B0, from the constructor's stores; names as gta-reversed's `LoadMonitor.h`):
//! `+0x00 m_bInFrame, +0x04 m_bUseLoadMonitor, +0x08 m_bForceProcLevel, +0x0C m_DisplayType,
//! +0x10 m_VarConsoleDisplayType, +0x14 m_eProcLevel, +0x18 m_eProcLevelToForce,
//! +0x1C m_bEnableAmbientCrime (byte), +0x20 m_LastTime, +0x24 m_NumFramesThisSec, +0x28 m_FPS,
//! +0x2C m_iStartTimes[4], +0x3C m_iCyclesThisFrame[4], +0x4C m_iMaxCycles[4],
//! +0x5C m_fSmoothedValues[4], +0x6C m_fPeakLevels[4], +0x7C m_fNormalizedPeakRangeValues[4],
//! +0x8C m_fAveragedCyclesThisSecond[4], +0x9C m_iCyclesHistory[4][8], +0x11C m_GraphPoints[4][100],
//! +0x2AC m_iCurrentGraphIndex`.
//!
//! Ported from the 1.0 US bytes; gta-reversed differs and the original is kept:
//! - the constructor also zeroes `m_iCyclesHistory` (32 dwords) and leaves `+0x08`, `+0x18`, the
//!   graph points and `+0x2AC` untouched;
//! - `EndFrame` compares the normalised level against **0.66666 (0x3F2AAA3B)** and **0.33333
//!   (0x3EAAAA3B)** from `.rdata` (gr: `2.f / 3.f`, `1.f / 3.f`); the move-speed magnitude is
//!   `sqrt((x*50)^2 + (y*50)^2 + (z*50)^2)` with each component scaled first (gr: `|v| * 50`); every
//!   unsigned-to-float conversion is `fild` + 2^32 when bit 31 is set; all arithmetic x87 PC=24.

use crate::mem::GPtr;
use crate::timer::TIME_MS;
use crate::{globals, hook, orig};
use sa_math::x87::{Cmp, X87};

globals! {
    list GLOBALS;
}

/// `.rdata` constants (f32 bits).
pub mod k {
    /// 0x858B40: 50.0
    pub const FIFTY: u32 = 0x4248_0000;
    /// 0x858C54: 2^32
    pub const TWO_POW_32: u32 = 0x4F80_0000;
    /// 0x858624: 1.0
    pub const ONE: u32 = 0x3F80_0000;
    /// 0x858C48: 0.125
    pub const EIGHTH: u32 = 0x3E00_0000;
    /// 0x863AD8: 0.66666
    pub const HIGH: u32 = 0x3F2A_AA3B;
    /// 0x863AD4: 0.33333
    pub const MED: u32 = 0x3EAA_AA3B;
}

orig! {
    /// `CTimer::GetCurrentTimeInCycles` (0x561A80)
    pub fn get_current_time_in_cycles() -> u32 = cdecl 0x561A80 => crate::timer::get_current_time_in_cycles;
    /// `CTimer::GetCyclesPerMillisecond` (0x561A40)
    pub fn get_cycles_per_millisecond() -> u32 = cdecl 0x561A40 => crate::timer::get_cycles_per_millisecond;
    /// `FindPlayerPed(int32 playerId)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
}

/// Byte-offset access to a `CLoadMonitor`.
#[derive(Clone, Copy)]
struct Lm(GPtr<u8>);

impl Lm {
    fn u(self, off: u32) -> u32 {
        self.0.byte_add(off as isize).cast::<u32>().read()
    }
    fn set_u(self, off: u32, v: u32) {
        self.0.byte_add(off as isize).cast::<u32>().write(v)
    }
    fn f(self, off: u32) -> f32 {
        self.0.byte_add(off as isize).cast::<f32>().read()
    }
    fn set_f(self, off: u32, v: f32) {
        self.0.byte_add(off as isize).cast::<f32>().write(v)
    }
}

/// `fild dword` of an unsigned value: + 2^32 when bit 31 is set (`jge` skips the add).
pub fn fild_u32(v: u32) -> X87 {
    let x = X87::from_i32(v as i32);
    if (v as i32) < 0 {
        x.fadd_m(f32::from_bits(k::TWO_POW_32))
    } else {
        x
    }
}

/// `CLoadMonitor::CLoadMonitor()` (0x53CFA0, thiscall, returns `this`).
#[hook(addr = 0x53CFA0, conv = thiscall, name = "CLoadMonitor::Constructor", expect = "8B D1 53 33 C0", this = "CLoadMonitor")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    construct(this, TIME_MS.get())
}

fn construct(this: GPtr<u8>, time_ms: u32) -> GPtr<u8> {
    let m = Lm(this);
    m.set_u(0x00, 0);
    m.set_u(0x04, 1);
    m.set_u(0x14, 0);
    this.byte_add(0x1C).write(1);
    m.set_u(0x20, time_ms);
    m.set_u(0x24, 0);
    m.set_u(0x28, 0);
    for t in 0..4 {
        m.set_u(0x4C + 4 * t, 0);
        m.set_u(0x8C + 4 * t, 0);
        for j in 0..8 {
            m.set_u(0x9C + 0x20 * t + 4 * j, 0);
        }
    }
    m.set_u(0x0C, 0);
    m.set_u(0x10, 0);
    m.set_u(0x6C, 0x4170_0000); // 15.0
    m.set_u(0x70, 0x41A0_0000); // 20.0
    m.set_u(0x74, 0x4120_0000); // 10.0
    m.set_u(0x78, 0x4248_0000); // 50.0
    this
}

/// `CLoadMonitor::~CLoadMonitor()` (0x856430): empty (`ret`). Not hookable (lone `ret`).
pub fn destructor(_this: GPtr<u8>) {}

/// `CLoadMonitor::Render()` (0x53D480): empty (`ret`). Not hookable (lone `ret`).
pub fn render(_this: GPtr<u8>) {}

/// `CLoadMonitor::BeginFrame()` (0x53D030, thiscall).
#[hook(addr = 0x53D030, conv = thiscall, name = "CLoadMonitor::BeginFrame", expect = "33 C0 89 41 3C", this = "CLoadMonitor")]
pub fn begin_frame(this: GPtr<u8>) {
    let m = Lm(this);
    for t in 0..4 {
        m.set_u(0x3C + 4 * t, 0);
    }
    m.set_u(0, 1);
}

/// `CLoadMonitor::StartTimer(type)` (0x53D050, thiscall, `ret 4`).
#[hook(addr = 0x53D050, conv = thiscall, name = "CLoadMonitor::StartTimer", expect = "56 8B F1 E8 28 4A 02 00", this = "CLoadMonitor")]
pub fn start_timer(this: GPtr<u8>, t: i32) {
    let now = get_current_time_in_cycles();
    this.byte_add(0x2C).cast::<u32>().wrapping_index(t).write(now);
}

/// `CLoadMonitor::EndTimer(type)` (0x53D070, thiscall, `ret 4`): `cycles[t] += now - start[t]`.
#[hook(addr = 0x53D070, conv = thiscall, name = "CLoadMonitor::EndTimer", expect = "56 57 8B 7C 24 0C", this = "CLoadMonitor")]
pub fn end_timer(this: GPtr<u8>, t: i32) {
    let now = get_current_time_in_cycles();
    let start = this.byte_add(0x2C).cast::<u32>().wrapping_index(t);
    let cyc = this.byte_add(0x3C).cast::<u32>().wrapping_index(t);
    let d = now.wrapping_sub(start.read());
    cyc.write(cyc.read().wrapping_add(d));
}

/// The move-speed sample: `_ftol(sqrt((x*50)^2 + (y*50)^2 + (z*50)^2))`, low dword.
pub fn move_speed_sample(v: [f32; 3]) -> u32 {
    let fifty = f32::from_bits(k::FIFTY);
    let a = X87::from_f32(v[0]).fmul_m(fifty);
    let b = X87::from_f32(v[1]).fmul_m(fifty);
    let c = X87::from_f32(v[2]).fmul_m(fifty);
    c.fmul(c).fadd(b.fmul(b)).fadd(a.fmul(a)).fsqrt().ftol() as u32
}

/// `CLoadMonitor::EndFrame()` (0x53D0B0, thiscall).
#[hook(addr = 0x53D0B0, conv = thiscall, name = "CLoadMonitor::EndFrame", expect = "83 EC 10 53 56 57", this = "CLoadMonitor")]
pub fn end_frame(this: GPtr<u8>) {
    let ped = find_player_ped(-1);
    let speed = if ped.is_null() {
        None
    } else {
        // `m_nPedFlags` bit 8 (`bInVehicle`) and `m_pVehicle`: the vehicle's move speed (+0x44)
        let flags = ped.byte_add(0x46C).cast::<u32>().read();
        let veh = ped.byte_add(0x58C).cast::<GPtr<u8>>().read();
        let src = if flags & 0x100 != 0 && !veh.is_null() {
            veh
        } else {
            ped
        };
        Some(src.byte_add(0x44).cast::<[f32; 3]>().read())
    };
    end_frame_with(Lm(this), speed, || TIME_MS.get(), get_cycles_per_millisecond)
}

/// `EndFrame` after the player lookup; `now` reads `CTimer::m_snTimeInMilliseconds` (re-read where
/// the original re-reads it).
fn end_frame_with(m: Lm, speed: Option<[f32; 3]>, now: impl Fn() -> u32, cycles_per_ms: impl Fn() -> u32) {
    if let Some(v) = speed {
        m.set_u(0x48, move_speed_sample(v));
    }
    for t in 0..4 {
        let c = m.u(0x3C + 4 * t);
        if c > m.u(0x4C + 4 * t) {
            m.set_u(0x4C + 4 * t, c);
        }
        m.set_f(0x8C + 4 * t, fild_u32(c).fadd_m(m.f(0x8C + 4 * t)).to_f32());
    }
    let delta = now().wrapping_sub(m.u(0x20));
    if delta > 2000 {
        m.set_u(0x24, 0);
        m.set_u(0x20, now());
        return;
    }
    if delta < 1000 {
        m.set_u(0x24, m.u(0x24).wrapping_add(1));
    } else {
        let n = m.u(0x24);
        m.set_u(0x28, n);
        m.set_u(0x20, now());
        if n == 0 {
            m.set_u(0x24, 1);
        }
        let recip = fild_u32(m.u(0x24)).fdivr_m(f32::from_bits(k::ONE));
        for t in 0..4 {
            m.set_f(0x8C + 4 * t, recip.fmul_m(m.f(0x8C + 4 * t)).to_f32());
        }
        m.set_u(0x24, 0);
        for t in 0..4 {
            let h = 0x9C + 0x20 * t;
            for j in (1..8).rev() {
                m.set_u(h + 4 * j, m.u(h + 4 * (j - 1)));
            }
            m.set_u(h, X87::from_f32(m.f(0x8C + 4 * t)).ftol() as u32);
            m.set_u(0x8C + 4 * t, 0);
        }
        m.set_u(0x14, 0);
        for t in 0..4u32 {
            let max = m.u(0x4C + 4 * t);
            if max == 0 {
                m.set_u(0x5C + 4 * t, 0);
            } else {
                let h = 0x9C + 0x20 * t;
                let mut sum = fild_u32(m.u(h));
                for j in 1..8 {
                    sum = sum.fadd(fild_u32(m.u(h + 4 * j)));
                }
                let avg = sum.fmul_m(f32::from_bits(k::EIGHTH));
                let maxf = fild_u32(max).to_f32();
                // fcom m32; test ah, 41h; jne keep -> replace only when avg > max
                let v = if avg.fcom(X87::from_f32(maxf)) == Cmp::Greater {
                    X87::from_f32(maxf)
                } else {
                    avg
                };
                m.set_f(0x5C + 4 * t, v.to_f32());
            }
            let val = match t {
                3 => X87::from_f32(m.f(0x68)),
                2 => X87::from_f32(m.f(0x64)),
                _ => fild_u32(cycles_per_ms()).fdivr_m(m.f(0x5C + 4 * t)),
            };
            let norm = val.fdiv_m(m.f(0x6C + 4 * t));
            let level = m.u(0x14) as i32;
            if norm.fcom(X87::constant(k::HIGH)) == Cmp::Greater && level < 2 {
                m.set_u(0x14, 2);
            } else if norm.fcom(X87::constant(k::MED)) == Cmp::Greater && (m.u(0x14) as i32) < 1 {
                m.set_u(0x14, 1);
            }
            m.set_f(0x7C + 4 * t, norm.to_f32());
            let max = m.u(0x4C + 4 * t);
            m.set_f(0x5C + 4 * t, fild_u32(max).fdivr_m(m.f(0x5C + 4 * t)).to_f32());
        }
    }
    if m.u(0x08) != 0 {
        m.set_u(0x14, m.u(0x18));
    }
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_BEGIN_FRAME,
    HOOK_START_TIMER,
    HOOK_END_TIMER,
    HOOK_END_FRAME,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn frame_cycle() {
        let mut buf = vec![0xEEu8; 0x2B0];
        let p = GPtr::from_raw(buf.as_mut_ptr());
        let m = Lm(p);
        construct(p, 5000);
        assert_eq!(m.u(0x20), 5000);
        assert_eq!(m.u(0x08), 0xEEEE_EEEE);
        assert_eq!(m.u(0x9C + 0x7C), 0);
        assert_eq!(m.f(0x78), 50.0);
        m.set_u(0x08, 0);
        begin_frame(p);
        m.set_u(0x3C, 3000); // PED_AI cycles
        m.set_u(0x44, 5); // streaming requests
        end_frame_with(m, Some([0.3, 0.4, 0.0]), || 5500, || 1000);
        assert_eq!(m.u(0x48), 25); // |(15, 20, 0)|
        assert_eq!(m.u(0x24), 1);
        assert_eq!(m.f(0x8C), 3000.0);
        end_frame_with(m, None, || 6600, || 1000);
        assert_eq!(m.u(0x28), 1);
        assert_eq!(m.u(0x9C), 6000); // averaged over 1 frame: 3000 + 3000
        assert_eq!(m.u(0xA0), 0);
        assert_eq!(m.f(0x5C), 750.0 / 3000.0); // min(6000/8, 3000) / 3000
        assert_eq!(m.f(0x7C), 0.75 / 15.0); // 750 cycles / 1000 per ms / 15
        assert_eq!(m.u(0x14), 0);
        end_frame_with(m, None, || 9000, || 1000);
        assert_eq!((m.u(0x20), m.u(0x24)), (9000, 0));
        destructor(p);
        render(p);
    }

    #[test]
    fn fild_unsigned() {
        assert_eq!(fild_u32(0x8000_0000).to_f32(), 2147483648.0);
        assert_eq!(move_speed_sample([f32::NAN, 0.0, 0.0]), 0);
    }
}
