//! WRITER-MVP-B (helperB) `CStats` ports: `IncrementStat`, `CheckForStatsMessage`, `UpdateSexAppealStat`. Ported
//! from the 1.0 US bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use super::common::{g, rd, wr, x, xi};

/// `CStats::StatTypesFloat` (float stats 0..0x51)
const STATS_FLOAT: usize = 0xB79380;
/// `CStats::StatTypesInt` biased so that `[STATS_INT + 4 * id]` is int stat `id` (ids >= 0x52)
const STATS_INT: usize = 0xB78E20;
/// `CWorld::Players[0]` (+0x190 each); +0 ped, +0x144 the max health byte, +0x14F (word at +0x144) health bonus
const PLAYER_INFO: usize = 0xB7CD98;
const K1000: f32 = 1000.0;

orig! {
    /// `CStats::IsStatCapped(uint16)` (0x559630, cdecl)
    fn is_stat_capped(id: u32) -> u8 = cdecl 0x559630 => unported;
    /// `CStats::ProcessReactionStatsOnIncrement(uint8)` (0x55B900, cdecl)
    fn process_reaction_stats_on_increment(id: u32) = cdecl 0x55B900 => unported;
    /// `CStats::CheckForStatsMessage` (0x559760, cdecl)
    fn check_for_stats_message_o() = cdecl 0x559760 => crate::world::mvpb_stats::check_for_stats_message;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180, cdecl)
    fn increment_stat_o(id: u32, v: f32) = cdecl 0x55C180 => crate::world::mvpb_stats::increment_stat;
    /// `CStats::UpdateStatsAddToHealth(uint32)` (0x55D030, cdecl)
    fn update_stats_add_to_health(v: i32) = cdecl 0x55D030 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, cdecl, float in ST0)
    fn get_stat_value(id: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(i: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CHud::HelpMessageDisplayed` (0x588B50, cdecl)
    fn hud_help_message_displayed() -> u8 = cdecl 0x588B50 => unported;
    /// `CText::Get(const char*)` (0x6A0050, `ret 4`)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool, bool, bool)` (0x588BE0, cdecl)
    fn hud_set_help_message(s: GPtr<u8>, a: u32, b: u32, c: u32) = cdecl 0x588BE0 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CWorld::FindPlayerCoors(CVector* out, int32)` (0x56E010, cdecl)
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CTagManager::GetPercentageTagged()` (0x49CDA0, cdecl)
    fn tag_manager_get_percentage_tagged() -> i32 = cdecl 0x49CDA0 => unported;
    /// `CStats::CheckForThreshold(float*, float)` (0x5595F0, cdecl)
    fn check_for_threshold(stat: GPtr<u32>, t: u32) -> u8 = cdecl 0x5595F0 => crate::world::stats::check_for_threshold;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8, uint32, float)` (0x55B980, cdecl)
    fn display_script_stat_update_message(up: u32, stat: u32, v: u32) = cdecl 0x55B980 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

fn info() -> GPtr<u8> {
    GPtr::from_addr(PLAYER_INFO + 0x190 * g::<u8>(0xB7CD74).read() as usize)
}

/// `value < 1000` keeps the x87 value, anything else (incl. NaN) becomes 1000 (`test ah, 5; jnp`).
fn cap_1000(v: X87) -> X87 {
    if v.fcom(x(K1000)) == Cmp::Less { v } else { x(K1000) }
}

/// `CStats::IncrementStat(uint16 id, float value)` (0x55C180, cdecl): nothing unless value > 0. Float stats (< 0x52)
/// add the value (capped stats clamp to 1000 when not below it). 0xF5 (fat?) also increments 0x14F and, when the value
/// exceeds the free health room (`max health byte - ped health`, truncated), increments 0x15 by half the excess.
/// 0x14F lowers the health bonus word by value/2 (clamped 0..36) and raises the ped's health (+0x540) by the value,
/// at most the free room (then the bonus word is 0), then `UpdateStatsAddToHealth`. Other int stats add the
/// truncated value (as int16), capped ones clamp to 1000. Every path that got past the first test ends with
/// `ProcessReactionStatsOnIncrement(id)` and tail-calls `CheckForStatsMessage`.
#[hook(addr = 0x55C180, conv = cdecl, name = "CStats::IncrementStat", expect = "51 D9 44 24 0C",
       unverified = "mvpb-world-5", route = "L2", hazards = "heap-writes",
       note = "the original keeps ECX (stat index) across IsStatCapped and writes temporaries into its value argument slot")]
pub fn increment_stat(id: u32, value: f32) {
    if x(value).fcom(X87::zero()).test_any(0x41) {
        return;
    }
    let s = id & 0xFFFF;
    if s < 0x52 {
        let cell = g::<f32>(STATS_FLOAT + 4 * s as usize);
        let v = x(value).fadd_m(cell.read());
        cell.write(v.to_f32());
        if is_stat_capped(id) != 0 {
            cell.write(cap_1000(v).to_f32());
        }
    } else if s == 0xF5 {
        let inf = info();
        let ped = rd::<GPtr<u8>>(inf, 0);
        let room = xi(rd::<u8>(inf, 0x14F) as i32).fsub_m(rd::<f32>(ped, 0x540)).ftol() as u32 as i32;
        increment_stat_o(0x14F, value);
        let r = xi(room);
        if !x(value).fcom(r).test_any(0x41) {
            let half = x(value).fsub(r).fmul_m(0.5).to_f32();
            increment_stat_o(0x15, half);
        }
    } else if s == 0x14F {
        let inf = info();
        let bonus = at16(inf);
        let t = xi(bonus.read() as i32).fsub(x(value).fmul_m(0.5)).to_f32();
        let c = if X87::zero().fcom(x(t)).test_any(0x41) {
            if x(t).fcom(x(36.0)) == Cmp::Less { x(t) } else { x(36.0) }
        } else {
            X87::zero()
        };
        bonus.write(c.ftol() as u16 as i16);
        let ped = rd::<GPtr<u8>>(inf, 0);
        let room = xi(rd::<u8>(inf, 0x14F) as i32).fsub_m(rd::<f32>(ped, 0x540)).ftol() as u32 as i32;
        let mut val = value;
        if !x(value).fcom(xi(room)).c0() {
            val = xi(room).to_f32();
            bonus.write(0);
        }
        let ped = rd::<GPtr<u8>>(info(), 0);
        wr::<f32>(ped, 0x540, x(val).fadd_m(rd::<f32>(ped, 0x540)).to_f32());
        update_stats_add_to_health(x(val).ftol() as u32 as i32);
    } else {
        let cell = g::<i32>(STATS_INT + 4 * s as usize);
        let inc = x(value).ftol() as u16 as i16 as i32;
        let n = cell.read().wrapping_add(inc);
        cell.write(n);
        if is_stat_capped(id) != 0 {
            cell.write(cap_1000(xi(n)).ftol() as u32 as i32);
        }
    }
    process_reaction_stats_on_increment(id);
    check_for_stats_message_o();
}

/// The player's health bonus word (`CPlayerInfo` +0x144).
fn at16(inf: GPtr<u8>) -> GPtr<i16> {
    inf.byte_add(0x144).cast()
}

/// `CStats::CheckForStatsMessage` (0x559760, cdecl): unless the pad has controls disabled (+0x117), stat messages are
/// off (0x8CDE56), the camera fades (0xB6F065), 0xB5F852 / 0xBA82E3 are set or a help message is shown, the first
/// not-yet-shown entry (of `count` 0xB794D0, at most 128, 0x10 bytes at 0xB78200: stat word, shown, "less" flag,
/// threshold, text key) whose stat reached its threshold (`<` for less-flagged entries, else `<=` the value)
/// is marked shown, its text set as help message and 0xB794D4 = 1; every reached entry is processed.
#[hook(addr = 0x559760, conv = cdecl, name = "CStats::CheckForStatsMessage", expect = "6A 00 E8 09 64 FE FF",
       route = "L2", hazards = "none")]
pub fn check_for_stats_message() {
    if rd::<u8>(pad_get_pad(0), 0x117) != 0
        || g::<u8>(0x8CDE56).read() == 0
        || g::<u8>(0xB6F065).read() != 0
        || g::<u8>(0xB5F852).read() != 0
        || hud_help_message_displayed() != 0
        || g::<u8>(0xBA82E3).read() != 0
    {
        return;
    }
    let mut i = 0u32;
    while i < g::<u32>(0xB794D0).read() {
        let e: GPtr<u8> = GPtr::from_addr(0xB78200 + 0x10 * i as usize);
        if rd::<u8>(e, 2) == 0 {
            let v = X87::from_f64_exact(get_stat_value(rd::<u16>(e, 0) as u32));
            let c = v.fcom(x(rd::<f32>(e, 4)));
            let show = if rd::<u8>(e, 3) != 0 { !c.c0() } else { !c.test_parity(0x41) };
            if show {
                wr::<u8>(e, 2, 1);
                let s = text_get(GPtr::from_addr(0xC1B340), e.byte_add(8));
                hud_set_help_message(s, 0, 0, 0);
                g::<u8>(0xB794D4).write(1);
            }
        }
        i += 1;
        if i >= 0x80 {
            break;
        }
    }
}

/// `CStats::UpdateSexAppealStat` (0x55BF20, cdecl): the player's current vehicle is remembered in 0xB79530 (reference
/// registered, the old one is not released); within 35 (2D) of that car its health term `clamp(2*health - 1000,
/// 0..1000)` (x87, NaN kept) scaled by the vehicle class factor (0.1 / 1 / 0.3 / 0.5) counts, else 0; sex appeal
/// (0xB793E4) = `clamp(term*0.5 + stat 0xB794C0 * 0.5, 0..1000)`, 1000 while 0x969180 is set. Tail-calls
/// `CheckForStatsMessage`.
#[hook(addr = 0x55BF20, conv = cdecl, name = "CStats::UpdateSexAppealStat", expect = "83 EC 14 D9 05 C0 94 B7 00",
       unverified = "mvpb-world-5", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "fsqrt result compared, not stored")]
pub fn update_sex_appeal_stat() {
    let half = x(g::<f32>(0xB794C0).read()).fmul_m(0.5).to_f32();
    let cell = g::<GPtr<u8>>(0xB79530);
    if !find_player_vehicle(-1, 0).is_null() {
        let v = find_player_vehicle(-1, 0);
        cell.write(v);
        register_reference(v, GPtr::from_addr(0xB79530));
    }
    let car = cell.read();
    let mut term = X87::zero();
    if !car.is_null() {
        let m = rd::<GPtr<u8>>(car, 0x14);
        let pos = if m.is_null() { car.byte_add(4) } else { m.byte_add(0x30) };
        let mut tmp = [0.0f32; 3];
        let p = find_player_coors(GPtr::from_raw(tmp.as_mut_ptr()), -1);
        let dx = x(rd::<f32>(pos, 0)).fsub_m(p.read());
        let dy = x(rd::<f32>(pos, 4)).fsub_m(p.wrapping_index(1).read());
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        if d.fcom(x(35.0)) == Cmp::Less {
            let car = cell.read();
            let t = x(rd::<f32>(car, 0x4C0)).fdouble().fsub_m(K1000).to_f32();
            let mut val = if !x(K1000).fcom(x(t)).test_parity(5) {
                x(K1000)
            } else if X87::zero().fcom(x(t)).test_any(0x41) {
                x(t)
            } else {
                X87::zero()
            };
            let mi = rd::<GPtr<u8>>(g::<u8>(0xA9B0C8), 4 * rd::<i16>(car, 0x22) as isize);
            let k = (rd::<i8>(mi, 0x4D) as i32).wrapping_sub(1);
            val = match k {
                0 | 4 | 6 | 10 => val.fmul_m(0.1),
                3 | 5 | 9 => val.fmul_m(0.3),
                1 | 2 | 8 => val,
                _ => val.fmul_m(0.5),
            };
            term = val;
        }
    }
    let mut r = term.fmul_m(0.5).fadd_m(half);
    if !r.fcom(X87::zero()).test_parity(5) {
        r = X87::zero();
    } else if !r.fcom(x(K1000)).test_any(0x41) {
        r = x(K1000);
    }
    if g::<u8>(0x969180).read() != 0 {
        r = x(K1000);
    }
    g::<f32>(0xB793E4).write(r.to_f32());
    check_for_stats_message_o();
}

fn kf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `CStats::UpdateRespectStat(uint8 reset)` (0x55BC50, cdecl): `reset` sets the last/shown respect (0x8CDEC8 /
/// 0x8CDEC4) to -99. Otherwise respect = gang/weapon/mission/tag/... terms (x87 PC24, see the original's operand
/// order), clamped at 0 (1000 while 0x96917F is set); when its truncation changed it is stored (0xB79480),
/// `CheckForStatsMessage` runs, and a stat-update message (up = 1 / down = 0, stat 0x40) is shown when a
/// threshold was crossed or the value is below 2 (neither tracked value -99); the tracked values follow.
#[hook(addr = 0x55BC50, conv = cdecl, name = "CStats::UpdateRespectStat", expect = "8A 44 24 04 83 EC 10",
       unverified = "mvpb-world-5", route = "L2", hazards = "heap-writes")]
pub fn update_respect_stat(reset: u8) {
    let last = g::<u32>(0x8CDEC8);
    let shown = g::<u32>(0x8CDEC4);
    if reset != 0 {
        last.write(0xC2C6_0000);
        shown.write(0xC2C6_0000);
        return;
    }
    let a = x(g::<f32>(0xB79490).read()).fmul_m(kf(0x3ECC_CCCD));
    let b = xi(g::<i32>(0xB791A0).read()).fmul_m(K1000);
    let mut c = xi(g::<i32>(0xB791B0).read());
    if !x(1.0).fcom(c).test_any(0x41) {
        c = x(1.0);
    }
    let s1 = a.fadd(b.fdiv(c).fmul_m(kf(0x3EB8_51EC)));
    let s1 = s1.fadd(x(g::<f32>(0xB79484).read()).fmul_m(kf(0x3CF5_C28F)));
    let mut e = x(g::<f32>(0x96AB9C).read()).fsub_m(kf(0x3E4C_CCCD));
    if !X87::zero().fcom(e).test_any(0x41) {
        e = X87::zero();
    }
    let t1 = e.fmul_m(1250.0).fmul_m(kf(0x3D4C_CCCD)).fadd(s1).to_f32();
    let f = xi(rd::<i32>(info(), 0xB8)).fmul_m(kf(0x33D6_BF95));
    let l0 = if x(1.0).fcom(f).test_parity(5) { f.to_f32() } else { 1.0 };
    let l8 = g::<f32>(0xB793DC).read();
    let lc = g::<f32>(0xB79488).read();
    let gk = xi(tag_manager_get_percentage_tagged().wrapping_mul(10)).fmul_m(kf(0x3D4C_CCCD));
    let h = x(l0).fmul_m(K1000).fmul_m(kf(0x3D4C_CCCD)).fadd_m(t1);
    let h = h.fadd(x(l8).fmul_m(kf(0x3CF5_C28F))).fadd(x(lc).fmul_m(kf(0x3CF5_C28F)));
    let r = gk.fadd(h);
    let mut v = r.to_f32();
    if !r.fcom(X87::zero()).test_parity(5) {
        v = 0.0;
    }
    if g::<u8>(0x96917F).read() != 0 {
        v = K1000;
    }
    let prev = x(f32::from_bits(last.read())).ftol() as u32;
    let now = x(v).ftol() as u32;
    if prev == now {
        return;
    }
    g::<f32>(0xB79480).write(v);
    check_for_stats_message_o();
    let k = kf(0xC2C6_0000);
    let lastf = |c: GPtr<u32>| x(f32::from_bits(c.read()));
    let up = x(v).fcom(lastf(shown)) == Cmp::Greater;
    let is99 = |c: GPtr<u32>| !lastf(c).fcom(x(k)).test_parity(0x44);
    if !up && lastf(last).fcom(x(2.0)) != Cmp::Less {
        if !is99(last) && !is99(shown) && (check_for_threshold(shown.cast(), v.to_bits()) != 0 || x(v).fcom(x(2.0)) == Cmp::Less) {
            display_script_stat_update_message(0, 0x40, v.to_bits());
        }
        if x(v).fcom(lastf(last)) == Cmp::Greater {
            shown.write(v.to_bits());
            last.write(v.to_bits());
            return;
        }
        last.write(v.to_bits());
        return;
    }
    if !is99(last) && !is99(shown) && (check_for_threshold(shown.cast(), v.to_bits()) != 0 || lastf(last).fcom(x(2.0)) == Cmp::Less) {
        display_script_stat_update_message(1, 0x40, v.to_bits());
    }
    if x(v).fcom(lastf(last)) == Cmp::Less {
        shown.write(v.to_bits());
    }
    last.write(v.to_bits());
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_UPDATE_RESPECT_STAT, HOOK_INCREMENT_STAT, HOOK_CHECK_FOR_STATS_MESSAGE, HOOK_UPDATE_SEX_APPEAL_STAT];
