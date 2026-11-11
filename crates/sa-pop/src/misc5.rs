//! CGangWars, CRoadBlocks and CDarkel leftovers (WRITER-POP-4, helper B). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{find_player_coors, g, jne_mask, jp5, model_info, rand_at, rd, wr, x, xi};
use crate::gangs::{find_sequence_element, init_sequence};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CTheZones::Calc2DDistanceBetween2Zones(CZone*, CZone*)` (0x5725B0, ST0)
    fn calc_2d_distance_between_2_zones(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x5725B0 => unported;
    /// `CGangWars::PickZoneToAttack()` (0x443B00)
    fn pick_zone_to_attack_o() -> u8 = cdecl 0x443B00 => crate::misc5::pick_zone_to_attack;
    /// `CText::Get(const char*)` (0x6A0050, `this` = TheText 0xC1B340)
    fn text_get(this: GPtr<u8>, key: u32) -> u32 = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool quick, bool permanent, bool addToBrief)` (0x588BE0)
    fn hud_set_help_message(text: u32, quick: u32, permanent: u32, brief: u32) = cdecl 0x588BE0 => unported;
    /// `CRadar::SetCoordBlip(eBlipType, CVector, eBlipColour, eBlipDisplay, const char*)` (0x583820; the
    /// vector as raw dwords)
    fn set_coord_blip(ty: i32, px: u32, py: u32, pz: u32, colour: u32, display: i32, name: u32) -> i32 = cdecl 0x583820 => unported;
    /// `CRadar::SetBlipSprite(int32, eRadarSprite)` (0x583D70)
    fn set_blip_sprite(blip: i32, sprite: i32) = cdecl 0x583D70 => unported;
    /// `CPedGroup::FindDistanceToNearestMember(CPed**)` (0x5FB0A0, `ret 4`, ST0)
    fn ped_group_find_distance_to_nearest_member(this: GPtr<u8>, out: GPtr<GPtr<u8>>) -> f64 = thiscall 0x5FB0A0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`; float as raw bits)
    fn ped_say(ped: GPtr<u8>, id: u32, delay: u32, prob: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CTheZones::GetZoneInfo(const CVector&, CZone**)` (0x572400)
    fn get_zone_info(p: GPtr<u32>, out: GPtr<u8>) -> GPtr<u8> = cdecl 0x572400 => unported;
    /// `CGangWars::CanPlayerStartAGangWarHere(CZoneInfo*)` (0x443F80)
    fn can_player_start_a_gang_war_here(zi: GPtr<u8>) -> u8 = cdecl 0x443F80 => crate::gangs::can_player_start_a_gang_war_here;
    /// `CMessages::AddMessageQ(const char*, uint32, uint16, bool)` (0x69F0B0)
    fn add_message_q(text: u32, time: u32, flag: u32, b: u32) = cdecl 0x69F0B0 => unported;
    /// `CMessages::AddToPreviousBriefArray(const char*, int32 x6, const char*)` (0x69DD50)
    #[allow(clippy::too_many_arguments)]
    fn add_to_previous_brief_array(text: u32, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, s: u32) = cdecl 0x69DD50 => unported;
    /// `CGangWars::ClearTheStreets()` (0x4444B0)
    fn clear_the_streets() = cdecl 0x4444B0 => crate::misc2::clear_the_streets;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CGangWars::TellGangMembersTo(bool)` (0x444530; protection VM body)
    fn tell_gang_members_to(b: u32) = cdecl 0x444530 => unported;
}

/// `CTheZones::TotalNumberOfNavigationZones` (u16)
const NUM_NAV_ZONES: usize = 0xBA3794;
/// `CTheZones::NavigationZoneArray` (`CZone`, 0x20 each)
const NAV_ZONES: usize = 0xBA3798;
/// `CTheZones::ZoneInfoArray` (`CZoneInfo`, 0x11 each; +0..+9 gang densities)
const ZONE_INFO: usize = 0xBA1DF0;
/// `CGangWars::TimeTillNextAttack` (float)
const TIME_TILL_NEXT_ATTACK: usize = 0x96AB48;
/// `CGangWars::State2`
const STATE2: usize = 0x96AB4C;
/// `CGangWars::Gang2`
const GANG2: usize = 0x96AB50;
/// `CGangWars::WarFerocity`
const WAR_FEROCITY: usize = 0x96AB58;
/// `CGangWars::State`
const STATE: usize = 0x96AB64;
/// `CGangWars::Gang1`
const GANG1: usize = 0x96AB84;
/// `CGangWars::pZoneToFightOver` (`CZone*`)
const ZONE_TO_FIGHT_OVER: usize = 0x96AB88;
/// `CGangWars::pZoneInfoToFightOver` (`CZoneInfo*`)
const ZONE_INFO_TO_FIGHT_OVER: usize = 0x96AB8C;
/// `CGangWars::TerritoryUnderControlPercentage` (float)
const TERRITORY_PERCENT: usize = 0x96AB9C;
/// `CGangWars::CoorsOfPlayerAtStartOfWar` (CVector)
const COORS_AT_START: usize = 0x96ABBC;
/// `CGangWars::PointOfAttack` (CVector)
const POINT_OF_ATTACK: usize = 0x96ABC8;
/// `TheText`
const THE_TEXT: usize = 0xC1B340;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

fn gp(addr: usize) -> GPtr<u8> {
    GPtr::from_addr(addr)
}

/// `rand() * (1/32767.5) * 0.9 * 1080000 + 648000`, `_ftol`, stored back as an unsigned int converted to
/// float (`fild; jge; fadd 2^32; fstp`) into `TimeTillNextAttack` (the inlined `CalculateTimeTillNextAttack`).
fn store_time_till_next_attack(r: i32) {
    let t = xi(r)
        .fmul_m(f32::from_bits(0x3800_0100))
        .fmul_m(f32::from_bits(0x3F66_6666))
        .fmul_m(1_080_000.0)
        .fadd_m(648_000.0)
        .ftol() as u32;
    let mut f = xi(t as i32);
    if (t as i32) < 0 {
        f = f.fadd_m(4_294_967_296.0);
    }
    g::<f32>(TIME_TILL_NEXT_ATTACK).write(f.to_f32());
}

/// `sqrt(dx*dx + dy*dy)` of the x87 sequences `fld a; fsub b` (x, then y) used by the gang war code
/// (addition order is irrelevant for the bits; the squares are 80-bit intermediates).
fn dist2d(ax: X87, ay: X87) -> X87 {
    ax.fmul(ax).fadd(ay.fmul(ay)).fsqrt()
}

/// `CGangWars::PickZoneToAttack()` (0x443B00, AL): walks the navigation zones in `CCarCtrl` sequence
/// order; a zone whose info has gang 0 + gang 2 density >= 20 (unsigned bytes, int sum) attacks the first
/// zone (array order) with an info index, not containing the point (2500, -1666) (x87 tests `test ah,
/// 0x41; je` / `test ah, 5; jnp` per edge), with the player's gang density > 15 and
/// `Calc2DDistanceBetween2Zones(src, dst) < 10`. For such a pair it stores `pZoneToFightOver`,
/// `pZoneInfoToFightOver`, `Gang1 = d2 < d0 ? 0 : 2` (from the source info), `PointOfAttack` = the
/// zone's centre (int sums * 0.5) with z = 10, and returns 1 when the player is more than 60 (2D) from
/// that point; otherwise it goes on searching with the globals left written (as the original). Callers
/// test only AL (the one caller, StartDefensiveGangWar, `test al, al`).
#[hook(addr = 0x443B00, conv = cdecl, name = "CGangWars::PickZoneToAttack", expect = "83 EC 28 0F B7 05 94 37 BA 00",
       unverified = "popw4-gang-1", route = "L2", hazards = "x87-transcendental", recs = "grove,police-gang")]
pub fn pick_zone_to_attack() -> u8 {
    let count = || i32::from(g::<u16>(NUM_NAV_ZONES).read());
    init_sequence(count());
    let mut i = 0;
    while i < count() {
        let src = (find_sequence_element(i) as u32).wrapping_shl(5) as usize;
        let src_info = g::<u16>(NAV_ZONES + src + 0x1C).read();
        let si = usize::from(src_info) * 0x11;
        if src_info != 0
            && i32::from(g::<u8>(ZONE_INFO + si + 2).read()) + i32::from(g::<u8>(ZONE_INFO + si).read()) >= 20
        {
            let mut j = 0;
            while j < count() {
                let z = NAV_ZONES + (j as usize) * 0x20;
                j += 1;
                let info = g::<u16>(z + 0x1C).read();
                if info == 0 {
                    continue;
                }
                let w = |o: usize| i32::from(g::<i16>(z + o).read());
                let contains = jne_mask(xi(w(0x10)), x(2500.0), 0x41)
                    && jp5(xi(w(0x16)), x(2500.0))
                    && jne_mask(xi(w(0x12)), x(-1666.0), 0x41)
                    && jp5(xi(w(0x18)), x(-1666.0));
                if contains {
                    continue;
                }
                let ti = usize::from(info) * 0x11;
                if g::<u8>(ZONE_INFO + ti + 1).read() <= 15 {
                    continue;
                }
                let d = X87::from_f64_exact(calc_2d_distance_between_2_zones(gp(NAV_ZONES + src), gp(z)));
                if jp5(d, x(10.0)) {
                    continue;
                }
                g::<u32>(ZONE_TO_FIGHT_OVER).write(z as u32);
                g::<u32>(ZONE_INFO_TO_FIGHT_OVER).write((ZONE_INFO + ti) as u32);
                let d0 = g::<u8>(ZONE_INFO + si).read();
                let gang = if g::<u8>(ZONE_INFO + si + 2).read() < d0 {
                    0
                } else {
                    2
                };
                g::<i32>(GANG1).write(gang);
                let cx = xi(w(0x16).wrapping_add(w(0x10))).fmul_m(0.5).to_f32();
                let cy = xi(w(0x18).wrapping_add(w(0x12))).fmul_m(0.5).to_f32();
                g::<f32>(POINT_OF_ATTACK + 8).write(10.0);
                g::<f32>(POINT_OF_ATTACK).write(cx);
                g::<f32>(POINT_OF_ATTACK + 4).write(cy);
                let mut out = [0f32; 3];
                let p = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
                let dx = x(g::<f32>(POINT_OF_ATTACK).read()).fsub_m(p.read());
                let dy = x(g::<f32>(POINT_OF_ATTACK + 4).read()).fsub_m(p.wrapping_index(1).read());
                if !jne_mask(dist2d(dx, dy), x(60.0), 0x41) {
                    return 1;
                }
            }
        }
        i += 1;
    }
    0
}

/// The zone labels `CheerVictory` compares `pZoneToFightOver` with (`_stricmp`), and the phrase each one
/// says (0xD0..0xDB).
const CHEER_LABELS: [usize; 12] = [
    0x8598F0, 0x8598EC, 0x8598E8, 0x8598E4, 0x8598E0, 0x8598DC, 0x8598D8, 0x8598D4, 0x8598CC, 0x8598C4,
    0x8598C0, 0x8598BC,
];

/// `CGangWars::CheerVictory()` (0x444040): the member of the player's group (`CPedGroup` at 0xC09920 +
/// group * 0x2D4) nearest to its leader (`FindDistanceToNearestMember`, distance discarded) says phrase
/// 0xD0 + k for the first of 12 zone labels (`_stricmp` with the zone's label, one call per label until a
/// match) equal to `pZoneToFightOver`'s label: `Say(0xD0 + k, 0, 1.0, false, false, false)`. Nothing without
/// a member.
#[hook(addr = 0x444040, conv = cdecl, name = "CGangWars::CheerVictory", expect = "51 6A FF C7 44 24 04 00 00 00 00",
       unverified = "popw4-gang-1", route = "L2", hazards = "heap-writes", recs = "grove,police-gang")]
pub fn cheer_victory() {
    let mut nearest = GPtr::<u8>::NULL;
    let pd = rd::<GPtr<u8>>(crate::common::find_player_ped(-1), 0x480);
    let grp = rd::<i32>(pd, 0x38);
    let group = gp((0xC09920 + grp.wrapping_mul(0x2D4)) as u32 as usize);
    let _ = ped_group_find_distance_to_nearest_member(group, GPtr::from_raw(&mut nearest));
    if nearest.is_null() {
        return;
    }
    for (k, &label) in CHEER_LABELS.iter().enumerate() {
        if stricmp(g::<GPtr<u8>>(ZONE_TO_FIGHT_OVER).read(), gp(label)) == 0 {
            ped_say(nearest, 0xD0 + k as u32, 0, 0x3F80_0000, 0, 0, 0);
            return;
        }
    }
}

/// `CGangWars::StartDefensiveGangWar()` (0x444300): with `PickZoneToAttack()`: `Difficulty`-like copy
/// 0x96AB80 = `TerritoryUnderControlPercentage` (raw dword), help message `GW_ATK` (`SetHelpMessage(text, true,
/// false, true)`), `State2 = 1`, 0x96AB44 = `_ftol(dist2d(player - PointOfAttack) * 200 + 240000)` (x87
/// `fsqrt`), a coord blip (type 4) at `PointOfAttack` coloured by `Gang1` (`r<<24 | g<<16 | b<<8 | 0xFF` from
/// 0x8D1344 / 0x8D1350 / 0x8D135C), display 2, script name "CODE", stored in 0x96AB98, its sprite by `Gang1`
/// (jump table 0x444494: 0 → 0x3B, 2 / 4 → 0x3C, 3 → 0x3A, else 0x13), 0x96AB92 = 0, and the zone info's
/// flags +0xF (bit 5 cleared, bit 6 set) and colour (+0xB..+0xE = FF 00 00 A0). Otherwise a new
/// `TimeTillNextAttack` (rand site 0x444453).
#[hook(addr = 0x444300, conv = cdecl, name = "CGangWars::StartDefensiveGangWar", expect = "83 EC 10 E8 F8 F7 FF FF",
       unverified = "popw4-gang-1", route = "L2", hazards = "x87-transcendental", recs = "grove,police-gang")]
pub fn start_defensive_gang_war() {
    if pick_zone_to_attack_o() == 0 {
        store_time_till_next_attack(rand_at(0x444453));
        return;
    }
    g::<u32>(0x96AB80).write(g::<u32>(TERRITORY_PERCENT).read());
    let text = text_get(gp(THE_TEXT), 0x859900);
    hud_set_help_message(text, 1, 0, 1);
    g::<i32>(STATE2).write(1);
    let mut out = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let dx = x(p.read()).fsub_m(g::<f32>(POINT_OF_ATTACK).read());
    let dy = x(p.wrapping_index(1).read()).fsub_m(g::<f32>(POINT_OF_ATTACK + 4).read());
    let v = dist2d(dx, dy).fmul_m(200.0).fadd_m(240_000.0).ftol() as u32;
    g::<u32>(0x96AB44).write(v);
    let gang = g::<i32>(GANG1).read() as usize;
    let colour = (u32::from(g::<u8>(0x8D1344usize.wrapping_add(gang)).read()) << 24)
        | (u32::from(g::<u8>(0x8D1350usize.wrapping_add(gang)).read()) << 16)
        | (u32::from(g::<u8>(0x8D135Cusize.wrapping_add(gang)).read()) << 8)
        | 0xFF;
    let blip = set_coord_blip(
        4,
        g::<u32>(POINT_OF_ATTACK).read(),
        g::<u32>(POINT_OF_ATTACK + 4).read(),
        g::<u32>(POINT_OF_ATTACK + 8).read(),
        colour,
        2,
        0x8598F4,
    );
    g::<i32>(0x96AB98).write(blip);
    let sprite = match g::<u32>(GANG1).read() {
        0 => 0x3B,
        2 | 4 => 0x3C,
        3 => 0x3A,
        _ => 0x13,
    };
    set_blip_sprite(blip, sprite);
    mark_zone_info_under_attack();
    g::<u8>(0x96AB92).write(0);
}

/// The zone info of `pZoneInfoToFightOver`: flags +0xF bit 5 cleared and bit 6 set, colour +0xB..+0xE =
/// FF 00 00 A0 (each write re-reads the pointer, as the original).
fn mark_zone_info_under_attack() {
    let zi = || g::<GPtr<u8>>(ZONE_INFO_TO_FIGHT_OVER).read();
    let z = zi();
    wr::<u8>(z, 0xF, (rd::<u8>(z, 0xF) & 0xDF) | 0x40);
    wr::<u8>(zi(), 0xB, 0xFF);
    wr::<u8>(zi(), 0xC, 0);
    wr::<u8>(zi(), 0xD, 0);
    wr::<u8>(zi(), 0xE, 0xA0);
}

/// `CGangWars::StartOffensiveGangWar()` (0x446050): `CoorsOfPlayerAtStartOfWar` = player position,
/// `GetZoneInfo(&coors, &pZoneToFightOver)` → `pZoneInfoToFightOver`, 0x96AB80 = territory percentage;
/// unless `CanPlayerStartAGangWarHere(info)`: 0x96AB5C = 0, done. Then the strongest enemy gang of the
/// densities 0, 2..9 (strict `>` keeps the first maximum; the runner-up into `Gang2`, the bytes' unrolled
/// tracking incl. the `Gang1`/`Gang2` writes when density 0 is zero), 0x96AB5C = 0, and the war starts only
/// with a non-zero total, `Gang1` 0 or 2 (from the register copy) and `State2 == 0`: message `GW_PROV`
/// (`AddMessageQ(text, 4500, 1, true)`, `AddToPreviousBriefArray`), 0x96AB60 = now, `State = 1`,
/// `ClearTheStreets()`, `WarFerocity = min(total / 15, 2)`; when stat 0xB5 < 0.0 (`test ah, 5; jp` skips
/// otherwise): territory <= 0.4 → 0 (`test ah, 0x41; jp`), else territory >= 0.5 → 5 (`test ah, 1; jne`);
/// 0 with 0x96AB91 set; `Gang2 = Gang1` when the strongest density > runner-up / 3 (signed division);
/// `TellGangMembersTo(false)`; the zone info marked as in StartDefensiveGangWar; 0x96AB28 = 0.
#[hook(addr = 0x446050, conv = cdecl, name = "CGangWars::StartOffensiveGangWar", expect = "83 EC 0C 53 55 56 57 8D",
       unverified = "popw4-gang-1", route = "L2", hazards = "none", recs = "grove,police-gang")]
pub fn start_offensive_gang_war() {
    let mut out = [0u32; 3];
    let p = find_player_coors(GPtr::from_raw(out.as_mut_ptr()).cast(), -1).cast::<u32>();
    g::<u32>(COORS_AT_START).write(p.read());
    g::<u32>(COORS_AT_START + 4).write(p.wrapping_index(1).read());
    g::<u32>(COORS_AT_START + 8).write(p.wrapping_index(2).read());
    let zi = get_zone_info(g::<u32>(COORS_AT_START), gp(ZONE_TO_FIGHT_OVER));
    g::<u32>(ZONE_INFO_TO_FIGHT_OVER).write(zi.addr() as u32);
    g::<u32>(0x96AB80).write(g::<u32>(TERRITORY_PERCENT).read());
    if can_player_start_a_gang_war_here(zi) == 0 {
        g::<i32>(0x96AB5C).write(0);
        return;
    }
    let d = |i: isize| i32::from(rd::<u8>(zi, i));
    let mut total = d(0);
    let mut best = 0;
    let mut second = 0;
    let mut gang1;
    if total > 0 {
        g::<i32>(GANG2).write(g::<i32>(GANG1).read());
        gang1 = 0;
        g::<i32>(GANG1).write(0);
        best = d(0);
    } else {
        gang1 = g::<i32>(GANG1).read();
    }
    for i in 2..10 {
        let c = d(i as isize);
        total += c;
        if c > best {
            g::<i32>(GANG2).write(gang1);
            gang1 = i;
            second = best;
            g::<i32>(GANG1).write(i);
            best = c;
        } else if c > second {
            second = c;
            g::<i32>(GANG2).write(i);
        }
    }
    g::<i32>(0x96AB5C).write(0);
    if total <= 0 || (gang1 != 0 && gang1 != 2) || g::<i32>(STATE2).read() != 0 {
        return;
    }
    let text = text_get(gp(THE_TEXT), 0x859920);
    add_message_q(text, 0x1194, 1, 1);
    let text = text_get(gp(THE_TEXT), 0x859920);
    add_to_previous_brief_array(text, -1, -1, -1, -1, -1, -1, 0);
    g::<u32>(0x96AB60).write(g::<u32>(TIME_MS).read());
    g::<i32>(STATE).write(1);
    clear_the_streets();
    let q = total / 15;
    g::<i32>(WAR_FEROCITY).write(2);
    if q <= 2 {
        g::<i32>(WAR_FEROCITY).write(q);
    }
    let stat = X87::from_f64_exact(get_stat_value(0xB5));
    if !jp5(stat, x(0.0)) {
        let t = || x(g::<f32>(TERRITORY_PERCENT).read());
        if !t().fcom(x(0.4)).test_parity(0x41) {
            g::<i32>(WAR_FEROCITY).write(0);
        } else if !t().fcom(x(0.5)).test_any(0x01) {
            g::<i32>(WAR_FEROCITY).write(5);
        }
    }
    if g::<u8>(0x96AB91).read() != 0 {
        g::<i32>(WAR_FEROCITY).write(0);
    }
    if best > second / 3 {
        g::<i32>(GANG2).write(g::<i32>(GANG1).read());
    }
    tell_gang_members_to(0);
    mark_zone_info_under_attack();
    g::<i32>(0x96AB28).write(0);
}

orig! {
    /// `CGangWars::PedStreamedInForThisGang(int32)` (0x4439D0)
    fn ped_streamed_in_for_this_gang(gang: i32) -> u8 = cdecl 0x4439D0 => crate::misc2::ped_streamed_in_for_this_gang;
    /// `CGangWars::PickStreamedInPedForThisGang(int32, int32*)` (0x443A20)
    fn pick_streamed_in_ped_for_this_gang(gang: i32, out: GPtr<i32>) -> u8 = cdecl 0x443A20 => crate::misc2::pick_streamed_in_ped_for_this_gang;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn request_model(m: i32, flags: i32) = cdecl 0x4087E0 => sa_stream::streaming::request_model;
    /// `CPathFind::AreNodesLoadedForArea(float, float, float, float)` (0x44DD10)
    fn are_nodes_loaded_for_area(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32) -> u8 = thiscall 0x44DD10 => crate::pathfind::are_nodes_loaded_for_area;
    /// `CPathFind::FindNodeClosestToCoors(CVector, uint8, float, uint16, int32, uint16, bool, bool)` (0x44F460;
    /// hidden return pointer, vector and floats as raw dwords)
    #[allow(clippy::too_many_arguments)]
    fn find_node_closest_to_coors(this: GPtr<u8>, out: GPtr<u32>, x: u32, y: u32, z: u32, ty: u32, dist: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u32> = thiscall 0x44F460 => unported;
    /// `CPathFind::FindNthNodeClosestToCoors(CVector, uint8, float, bool, bool, int32, bool, bool, CNodeAddress*)`
    /// (0x44F8C0; hidden return pointer, raw dwords)
    #[allow(clippy::too_many_arguments)]
    fn find_nth_node_closest_to_coors(this: GPtr<u8>, out: GPtr<u32>, x: u32, y: u32, z: u32, ty: u32, dist: u32, a: u32, b: u32, n: u32, c: u32, d: u32, e: u32) -> GPtr<u32> = thiscall 0x44F8C0 => unported;
    /// `CPathNode::GetPosition()` (0x420A10; hidden return pointer)
    fn path_node_get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => crate::pathfind::get_position;
    /// `CWorld::FindGroundZFor3DCoord(float, float, float, bool*, CEntity**)` (0x5696C0, ST0; raw dwords)
    fn find_ground_z_for_3d_coord(x: u32, y: u32, z: u32, found: GPtr<u8>, ent: GPtr<u8>) -> f64 = cdecl 0x5696C0 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020; raw bits)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, a: u32) = thiscall 0x59B020 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn ped_set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    fn get_ped_flag(ty: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CAcquaintance::SetAsAcquaintance(int32, uint32)` (0x608DA0)
    fn set_as_acquaintance(a: GPtr<u8>, k: u32, flag: u32) = thiscall 0x608DA0 => unported;
    /// `CRadar::SetEntityBlip(eBlipType, int32, uint32, eBlipDisplay, const char*)` (0x5839A0)
    fn set_entity_blip(ty: u32, handle: u32, colour: u32, display: u32, name: u32) -> i32 = cdecl 0x5839A0 => unported;
    /// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0)
    fn change_blip_scale(blip: i32, scale: i32) = cdecl 0x583CC0 => unported;
    /// `CRadar::ChangeBlipColour(int32, uint32)` (0x583AB0)
    fn change_blip_colour(blip: i32, colour: u32) = cdecl 0x583AB0 => unported;
    /// `CPopulation::PickGangCar(int32)` (0x614490)
    fn pick_gang_car(gang: i32) -> i32 = cdecl 0x614490 => crate::population::pick_gang_car;
    /// `CCarCtrl::CreateCarForScript(int32, CVector, bool)` (0x431F80; raw dwords)
    fn create_car_for_script(model: i32, x: u32, y: u32, z: u32, mission: u32) -> GPtr<u8> = cdecl 0x431F80 => unported;
    /// `CAutomobile::PlaceOnRoadProperly()` (0x6AF420)
    fn automobile_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6AF420 => unported;
    /// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70)
    fn vehicle_set_created_by(v: GPtr<u8>, by: i32) = thiscall 0x6D5D70 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32, uint8, uint32, uint32, bool, const char*)` (0x456F20; raw dwords)
    #[allow(clippy::too_many_arguments)]
    fn generate_new_pickup(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
}

/// `ThePaths` (0x96F050)
const THE_PATHS: usize = 0x96F050;
/// `CGangWars::Difficulty` (float, 0x96AB80)
const DIFFICULTY: usize = 0x96AB80;

/// The gang colour of `Gang1` as a blip colour (`r << 24 | g << 16 | b << 8 | 0xFF`, 0x8D1344 / 0x8D1350 /
/// 0x8D135C).
fn gang1_colour() -> u32 {
    let gang = g::<i32>(GANG1).read() as usize;
    (u32::from(g::<u8>(0x8D1344usize.wrapping_add(gang)).read()) << 24)
        | (u32::from(g::<u8>(0x8D1350usize.wrapping_add(gang)).read()) << 16)
        | (u32::from(g::<u8>(0x8D135Cusize.wrapping_add(gang)).read()) << 8)
        | 0xFF
}

/// `CGangWars::CreateDefendingGroup(int32 unused)` (0x4453D0, SEH frame; AL, the caller tests AL). Returns 0
/// (after `RequestModel(first model of the gang's group, 8)`) when no ped of `Gang1` is streamed in, when
/// the path nodes around `PointOfAttack` are not loaded, when no node is found within 400 of it, or when the
/// node (`GetPosition`) is within 40 of the player (2D, `fsqrt`, `test ah, 0x41; jne`). Otherwise
/// `n = ftol((Difficulty * 0.4 + 0.6) * 10)` gang peds (`PickStreamedInPedForThisGang` each, skipped when it
/// fails): angle `i * 2pi / n` (80-bit for `fsin`, the f32-rounded copy for `fcos` and the heading, as the
/// bytes), position `node + (sin, cos) * (rand frac * 3 + 2)` (rand sites 0x445578 / 0x4455B7, 1/32767.5),
/// z = ground below (node z + 2) + 1; `CCivilianPed(Gang1 + 7, model)` placed, heading in +0x558/+0x55C and
/// the matrix (`SetRotateZOnly`) or placement, created by 2, `CWorld::Add`, a `KillPedOnFoot(player, -1, 0,
/// 0, 0, 2)` script command event, +0x71A = `ftol(90 - Difficulty * 60)`, acquaintance 4 with ped type 0's
/// flag, a delayed weapon by `i % 4` (0x16, 0x1C, 0x19, 0x1D; 5000 rounds) made current, +0x474 |= 0x3000,
/// an entity blip (pool handle, gang colour, scale 2). Then up to three gang cars at the 1st..3rd nodes
/// within 100 of `PointOfAttack` (no 0xFFFF check, as the bytes) that are more than 25 (2D) from the player:
/// `PickGangCar(Gang1)` loaded, `CreateCarForScript(model, node, false)` facing the defending node
/// (forward = normalised `(ny - Ny, Nx - nx, 0)`, right = `(forward.y, -forward.x, 0)`),
/// `PlaceOnRoadProperly`, +0x428 bit 3 cleared, created by 2, +0x42D |= 8. Last a pickup (type 5,
/// model `rand() & 1` ? 0x8CD5C8 : 0x8CD5C0; rand sites 0x445A52 / 0x445A79 / 0x445ACE) at
/// node ± 2 (rand frac * 4 - 2), z = ground below (z + 1) + 0.75; 1.
#[hook(addr = 0x4453D0, conv = cdecl, name = "CGangWars::CreateDefendingGroup", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "popw4-gang-1", route = "split", hazards = "heap-writes,x87-transcendental", recs = "grove,police-gang")]
pub fn create_defending_group(_unused: i32) -> u8 {
    let gang = g::<i32>(GANG1).read();
    if ped_streamed_in_for_this_gang(gang) == 0 {
        let grp = g::<i32>(0x8D2618usize.wrapping_add((gang.wrapping_mul(12)) as usize)).read();
        let m = g::<u16>(0xC0F358usize.wrapping_add(grp.wrapping_mul(0x2A) as usize)).read();
        request_model(i32::from(m), 8);
        return 0;
    }
    let (px, py, pz) = (
        g::<u32>(POINT_OF_ATTACK).read(),
        g::<u32>(POINT_OF_ATTACK + 4).read(),
        g::<u32>(POINT_OF_ATTACK + 8).read(),
    );
    if are_nodes_loaded_for_area(
        gp(THE_PATHS),
        f32::from_bits(px),
        f32::from_bits(px),
        f32::from_bits(py),
        f32::from_bits(py),
    ) == 0
    {
        return 0;
    }
    let mut ret = [0u32; 1];
    let node = find_node_closest_to_coors(
        gp(THE_PATHS),
        GPtr::from_raw(ret.as_mut_ptr()),
        px,
        py,
        pz,
        0,
        400f32.to_bits(),
        0,
        0,
        0,
        0,
        1,
    )
    .read();
    if node & 0xFFFF == 0xFFFF {
        return 0;
    }
    let pn = gp(path_nodes(node & 0xFFFF).wrapping_add((node >> 16).wrapping_mul(0x1C)) as usize);
    let mut npos_buf = [0f32; 3];
    let np = path_node_get_position(pn, GPtr::from_raw(npos_buf.as_mut_ptr()));
    let npos = [
        np.read(),
        np.wrapping_index(1).read(),
        np.wrapping_index(2).read(),
    ];
    let mut out = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let dy = x(npos[1]).fsub_m(p.wrapping_index(1).read());
    let dx = x(npos[0]).fsub_m(p.read());
    if jne_mask(dist2d(dx, dy), x(40.0), 0x41) {
        return 0;
    }
    let n = x(g::<f32>(DIFFICULTY).read())
        .fmul_m(f32::from_bits(0x3ECC_CCCD))
        .fadd_m(f32::from_bits(0x3F19_999A))
        .fmul_m(10.0)
        .ftol() as i32;
    let mut i = 0;
    while i < n {
        let mut model = [0i32; 1];
        if pick_streamed_in_ped_for_this_gang(g::<i32>(GANG1).read(), GPtr::from_raw(model.as_mut_ptr())) == 0
        {
            i += 1;
            continue;
        }
        let frac = |r: i32| xi(r).fmul_m(f32::from_bits(0x3800_0100));
        let r = rand_at(0x445578);
        let angle = xi(i).fmul_m(f32::from_bits(0x40C9_0FDB)).fdiv(xi(n));
        let angle_f = angle.to_f32();
        let ped_x = angle
            .fsin()
            .fmul(frac(r).fmul_m(3.0).fadd_m(2.0))
            .fadd_m(npos[0])
            .to_f32();
        let r = rand_at(0x4455B7);
        let ped_y = x(angle_f)
            .fcos()
            .fmul(frac(r).fmul_m(3.0).fadd_m(2.0))
            .fadd_m(npos[1])
            .to_f32();
        let zarg = x(npos[2]).fadd_m(2.0).to_f32();
        let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
            ped_x.to_bits(),
            ped_y.to_bits(),
            zarg.to_bits(),
            GPtr::NULL,
            GPtr::NULL,
        ));
        let ped_z = gz.fadd_m(1.0).to_f32();
        let p = ped_new(0x79C);
        let ped = if p.is_null() {
            GPtr::NULL
        } else {
            civilian_ped_ctor(p, (g::<i32>(GANG1).read() + 7) as u32, model[0] as u32)
        };
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            wr::<f32>(ped, 4, ped_x);
            wr::<f32>(ped, 8, ped_y);
            wr::<f32>(ped, 0xC, ped_z);
        } else {
            wr::<f32>(m, 0x30, ped_x);
            wr::<f32>(rd::<GPtr<u8>>(ped, 0x14), 0x34, ped_y);
            wr::<f32>(rd::<GPtr<u8>>(ped, 0x14), 0x38, ped_z);
        }
        let a = angle_f.to_bits();
        wr::<u32>(ped, 0x55C, a);
        wr::<u32>(ped, 0x558, a);
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            wr::<u32>(ped, 0x10, a);
        } else {
            matrix_set_rotate_z_only(m, a);
        }
        ped_set_char_created_by(ped, 2);
        world_add(ped);
        let t = task_new(0x38);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            kill_ped_on_foot_ctor(t, crate::common::find_player_ped(-1), -1, 0, 0, 0, 2)
        };
        let mut ev = [0u32; 12];
        let evp = GPtr::from_raw(ev.as_mut_ptr());
        event_script_command_ctor(evp, 3, t, 0);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
        let acc = x(g::<f32>(DIFFICULTY).read()).fmul_m(60.0).fsubr_m(90.0).ftol() as u8;
        wr::<u8>(ped, 0x71A, acc);
        let flag = get_ped_flag(0);
        set_as_acquaintance(ped.byte_add(0x4E0), 4, flag);
        let w = [0x16, 0x1C, 0x19, 0x1D][(i & 3) as usize];
        ped_give_delayed_weapon(ped, w, 0x1388);
        ped_set_current_weapon(ped, w);
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x3000);
        let colour = gang1_colour();
        let pool = g::<GPtr<u8>>(0xB74490).read();
        let index = (ped.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / 0x7C4;
        let handle = ((index << 8) as u32).wrapping_add(u32::from(
            g::<u8>(rd::<u32>(pool, 4).wrapping_add(index as u32) as usize).read(),
        ));
        let blip = set_entity_blip(2, handle, colour, 2, 0x859908);
        change_blip_scale(blip, 2);
        change_blip_colour(blip, colour);
        event_script_command_dtor(evp);
        i += 1;
    }
    for nth in 1..=3u32 {
        let mut ret = [0u32; 1];
        find_nth_node_closest_to_coors(
            gp(THE_PATHS),
            GPtr::from_raw(ret.as_mut_ptr()),
            g::<u32>(POINT_OF_ATTACK).read(),
            g::<u32>(POINT_OF_ATTACK + 4).read(),
            g::<u32>(POINT_OF_ATTACK + 8).read(),
            0,
            100f32.to_bits(),
            0,
            0,
            nth,
            0,
            1,
            0,
        );
        let cn = gp(path_nodes(ret[0] & 0xFFFF).wrapping_add((ret[0] >> 16).wrapping_mul(0x1C)) as usize);
        let c = |o: isize| xi(i32::from(rd::<i16>(cn, o))).fmul_m(0.125).to_f32();
        let (cx, cy, cz) = (c(8), c(0xA), c(0xC));
        let mut out = [0f32; 3];
        let p = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
        let dx = x(p.read()).fsub_m(cx);
        let dy = x(p.wrapping_index(1).read()).fsub_m(cy);
        if jne_mask(dist2d(dx, dy), x(25.0), 0x41) {
            continue;
        }
        let model = pick_gang_car(g::<i32>(GANG1).read());
        if model < 0 || g::<u8>(0x8E4CD0 + model as usize * 20).read() != 1 {
            continue;
        }
        let mut dir = [
            x(cy).fsub_m(npos[1]).to_f32(),
            x(npos[0]).fsub_m(cx).to_f32(),
            0.0,
        ];
        vector_normalise(GPtr::from_raw(dir.as_mut_ptr()));
        let veh = create_car_for_script(model, cx.to_bits(), cy.to_bits(), cz.to_bits(), 0);
        let m = rd::<GPtr<u8>>(veh, 0x14);
        wr::<f32>(m, 0x10, dir[0]);
        wr::<f32>(m, 0x14, dir[1]);
        wr::<f32>(m, 0x18, dir[2]);
        let m = rd::<GPtr<u8>>(veh, 0x14);
        wr::<f32>(m, 0, dir[1]);
        wr::<f32>(m, 4, x(dir[0]).fchs().to_f32());
        wr::<u32>(m, 8, 0);
        automobile_place_on_road_properly(veh);
        wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0xF7);
        vehicle_set_created_by(veh, 2);
        wr::<u8>(veh, 0x42D, rd::<u8>(veh, 0x42D) | 8);
    }
    let frac4 = |r: i32| xi(r).fmul_m(f32::from_bits(0x3800_0100)).fmul_m(4.0);
    let r = rand_at(0x445A52);
    let kx = frac4(r).fadd_m(npos[0]).fsub_m(2.0).to_f32();
    let r = rand_at(0x445A79);
    let ky = frac4(r).fadd_m(npos[1]).fsub_m(2.0).to_f32();
    let zarg = x(npos[2]).fadd_m(1.0).to_f32();
    let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
        kx.to_bits(),
        ky.to_bits(),
        zarg.to_bits(),
        GPtr::NULL,
        GPtr::NULL,
    ));
    let kz = gz.fadd_m(0.75).to_f32();
    let model = if rand_at(0x445ACE) & 1 != 0 {
        g::<u16>(0x8CD5C8).read()
    } else {
        g::<u16>(0x8CD5C0).read()
    };
    generate_new_pickup(
        kx.to_bits(),
        ky.to_bits(),
        kz.to_bits(),
        u32::from(model),
        5,
        0,
        0,
        0,
        0,
    );
    1
}

orig! {
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `CWorld::FindObjectsIntersectingCube(const CVector&, const CVector&, int16*, int16, CEntity**, bool x5)` (0x568DD0)
    #[allow(clippy::too_many_arguments)]
    fn find_objects_intersecting_cube(a: GPtr<u32>, b: GPtr<u32>, count: GPtr<i16>, max: u32, out: GPtr<u32>, bb: u32, bv: u32, bp: u32, bo: u32, bd: u32) = cdecl 0x568DD0 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180)
    fn vehicle_can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CPed::CanBeDeleted()` (0x5DF100)
    fn ped_can_be_deleted(p: GPtr<u8>) -> u8 = thiscall 0x5DF100 => unported;
    /// `CObject::CanBeDeleted()` (0x59F120)
    fn object_can_be_deleted(o: GPtr<u8>) -> u8 = thiscall 0x59F120 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// virtual slot 0 (scalar deleting destructor) of a `CEntity`
    fn delete_entity(e: GPtr<u8>, flags: i32) = thiscall vslot 0 => unported;
}

/// `ThePaths.m_pPathNodes[area]` (0x96F854)
fn path_nodes(area: u32) -> u32 {
    g::<u32>(0x96F854 + area as usize * 4).read()
}

/// `CRoadBlocks::GetRoadBlockNodeInfo(CNodeAddress node, float& width, CVector& dir)` (0x460EE0, AL): the
/// node's first two navi links (`m_pNaviLinks[area][baseLinkId]`, u16 `area << 10 | id`); 0 when either link's
/// area has no path nodes loaded (`m_pPathNodes[area]` tested, as the bytes; the node itself is read without
/// a test). Otherwise `width = (max(lanes(first), lanes(second)) + 1) * 5` (lanes = the two 3-bit counts of
/// navi byte +0xB), `dir = (Sy - Fy, -(Sx - Fx), 0)` from the navi positions (int16 * 0.125, `fchs` keeps the
/// sign of a zero), `dir.Normalise()`, 1. The original uses its first argument slot as the `fild`
/// temporary (left holding the first navi node's y); its callers never read it again, the port does not
/// write it. Callers test AL only.
#[hook(addr = 0x460EE0, conv = cdecl, name = "CRoadBlocks::GetRoadBlockNodeInfo", expect = "8B 44 24 04 0F B7 4C 24 04",
       unverified = "popw4-gang-1", route = "L2", hazards = "none", recs = "police-gang")]
pub fn get_road_block_node_info(node: u32, width: GPtr<f32>, dir: GPtr<f32>) -> u8 {
    let area = node & 0xFFFF;
    let id = node >> 16;
    let n = path_nodes(area).wrapping_add(id.wrapping_mul(0x1C));
    let base = i32::from(rd::<i16>(gp(n as usize), 0x10));
    let links = g::<u32>(0x96FDF4 + area as usize * 4).read();
    let l = gp(links.wrapping_add(base.wrapping_mul(2) as u32) as usize);
    let a = u32::from(rd::<u16>(l, 0));
    let b = u32::from(rd::<u16>(l, 2));
    if path_nodes(a >> 10) == 0 || path_nodes(b >> 10) == 0 {
        return 0;
    }
    let navi = |v: u32| {
        let base = g::<u32>(0x96F974 + (v >> 10) as usize * 4).read();
        gp(base.wrapping_add((v & 0x3FF) * 0xE) as usize)
    };
    let (fa, fb) = (navi(a), navi(b));
    let lanes = |p: GPtr<u8>| {
        let v = i32::from(rd::<u8>(p, 0xB));
        ((v >> 3) & 7) + (v & 7)
    };
    let (la, lb) = (lanes(fa), lanes(fb));
    let w = if la > lb { la } else { lb };
    width.write(xi(w).fadd_m(1.0).fmul_m(5.0).to_f32());
    let c = |p: GPtr<u8>, o: isize| xi(i32::from(rd::<i16>(p, o))).fmul_m(0.125);
    let t = c(fb, 0).fsub(c(fa, 0));
    dir.wrapping_index(2).write(0.0);
    let u = c(fb, 2).fsub(c(fa, 2));
    dir.write(u.to_f32());
    dir.wrapping_index(1).write(t.fchs().to_f32());
    vector_normalise(dir);
    1
}

/// `CRoadBlocks::ClearSpaceForRoadBlockObject(CVector cornerA, CVector cornerB)` (0x461020, AL; the vectors by
/// value): `FindObjectsIntersectingCube(&a, &b, &n, 2, list, vehicles, peds, objects)`; more than 2 → 0. Each
/// found entity: a deletable vehicle is removed unless its +0x42A bit 4 is set (then kept), a deletable ped
/// is removed, an object is removed when +0x13C != 1 and deletable; anything else (non-deletable vehicle /
/// ped, object with +0x13C == 1, other types) returns 0 at once (entities removed before stay removed).
/// Removal: `CWorld::Remove` + virtual scalar deleting destructor (1). Then 1. Callers test AL only.
#[hook(addr = 0x461020, conv = cdecl, name = "CRoadBlocks::ClearSpaceForRoadBlockObject", expect = "83 EC 0C 6A 00 6A 01 6A",
       unverified = "popw4-gang-1", route = "split", hazards = "heap-writes", recs = "police-gang")]
pub fn clear_space_for_road_block_object(ax: u32, ay: u32, az: u32, bx: u32, by: u32, bz: u32) -> u8 {
    let mut a = [ax, ay, az];
    let mut b = [bx, by, bz];
    let mut count = [0i16; 2];
    let mut list = [0u32; 2];
    find_objects_intersecting_cube(
        GPtr::from_raw(a.as_mut_ptr()),
        GPtr::from_raw(b.as_mut_ptr()),
        GPtr::from_raw(count.as_mut_ptr()),
        2,
        GPtr::from_raw(list.as_mut_ptr()),
        0,
        1,
        1,
        1,
        0,
    );
    let n = count[0];
    if n > 2 {
        return 0;
    }
    let mut i: i16 = 0;
    while i < n {
        let e = gp(list[i as usize] as usize);
        i += 1;
        let ty = || rd::<u8>(e, 0x36) & 7;
        if ty() == 2 && vehicle_can_be_deleted(e) != 0 {
            if rd::<u8>(e, 0x42A) & 0x10 != 0 {
                continue;
            }
        } else if !(ty() == 3 && ped_can_be_deleted(e) != 0)
            && (ty() != 4 || rd::<u8>(e, 0x13C) == 1 || object_can_be_deleted(e) == 0)
        {
            return 0;
        }
        world_remove(e);
        delete_entity(e, 1);
    }
    1
}

orig! {
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `CGangs::ChooseGangPedModel(int16)` (0x5DE5A0; pushed as a dword)
    fn choose_gang_ped_model(gang: i32) -> i32 = cdecl 0x5DE5A0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CStreaming::GetDefaultCopCarModel(bool)` (0x407C50)
    fn get_default_cop_car_model(b: u32) -> i32 = cdecl 0x407C50 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, returns `out`)
    fn matrix_mul_vector(out: GPtr<u32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<u32> = cdecl 0x59C890 => unported;
    /// `CPed::operator new(uint32)` (0x5E4720)
    fn ped_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CCopPed::CCopPed(eCopType)` (0x5DDC60)
    fn cop_ped_ctor(p: GPtr<u8>, cop_type: u32) -> GPtr<u8> = thiscall 0x5DDC60 => unported;
    /// `CCivilianPed::CCivilianPed(ePedType, uint32 model)` (0x5DDB70)
    fn civilian_ped_ctor(p: GPtr<u8>, ty: u32, model: u32) -> GPtr<u8> = thiscall 0x5DDB70 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector&)` (0x616920)
    fn find_z_coor_for_ped(v: GPtr<u32>) -> u8 = cdecl 0x616920 => unported;
    /// `CMatrix::SetRotate(float, float, float)` (0x59B120; raw float bits)
    fn matrix_set_rotate(m: GPtr<u8>, ax: u32, ay: u32, az: u32) = thiscall 0x59B120 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexWander::CTaskComplexWander(int32, uint8, bool, float)` (0x66F450; float as raw bits)
    fn wander_ctor(t: GPtr<u8>, ms: u32, dir: u32, b: u32, r: u32) -> GPtr<u8> = thiscall 0x66F450 => unported;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310; float as raw bits)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CVisibilityPlugins::SetClumpAlpha(RpClump*, int32)` (0x732B00)
    fn set_clump_alpha(clump: GPtr<u8>, a: i32) = cdecl 0x732B00 => unported;
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32)` (0x5E89B0)
    fn ped_give_delayed_weapon(p: GPtr<u8>, w: i32, ammo: u32) = thiscall 0x5E89B0 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(CPed*, int32, uint32, uint32, uint32, bool)` (0x620E30)
    #[allow(clippy::too_many_arguments)]
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, target: GPtr<u8>, time: i32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00)
    fn event_script_command_ctor(e: GPtr<u32>, index: u32, task: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(group: GPtr<u8>, ev: GPtr<u32>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u32>) = thiscall 0x4B0A50 => unported;
}

/// The two ped offset tables of `GenerateRoadBlockPedsForCar` (stack locals, contiguous in the original:
/// set A then set B), as f32 bits: A = (-1.5, 1.9), (-1.5, -2.6), (1.5, 1.9), (1.5, -2.6), (-1.5, 0),
/// (1.5, 0); B = (0, 3.2), (1.5, -1.8), (0, 3.2), (-1.5, -1.8), (-1.5, 0), (1.5, 0); z = 0.
const ROAD_BLOCK_PED_OFFSETS: [[u32; 3]; 12] = [
    [0xBFC0_0000, 0x3FF3_3333, 0],
    [0xBFC0_0000, 0xC026_6666, 0],
    [0x3FC0_0000, 0x3FF3_3333, 0],
    [0x3FC0_0000, 0xC026_6666, 0],
    [0xBFC0_0000, 0, 0],
    [0x3FC0_0000, 0, 0],
    [0, 0x404C_CCCD, 0],
    [0x3FC0_0000, 0xBFE6_6666, 0],
    [0, 0x404C_CCCD, 0],
    [0xBFC0_0000, 0xBFE6_6666, 0],
    [0xBFC0_0000, 0, 0],
    [0x3FC0_0000, 0, 0],
];

/// `GenerateRoadBlockPedsForCar`'s cop vehicle cases (jump tables 0x4618FC / 0x461910 on model - 0x1AB):
/// (cop type, ped model, use offset set B).
fn road_block_cop_case(model: i32) -> (u32, i32, bool) {
    match model {
        0x1AB => (2, 0x11D, true),
        0x1B1 => (5, 0x11F, true),
        0x1EA => (4, 0x11E, true),
        0x257 => (0, 0, true),
        _ => (0, 0, false),
    }
}

/// `CRoadBlocks::GenerateRoadBlockPedsForCar(CVehicle* veh, int32 pair, ePedType type)` (0x461170, SEH frame;
/// AL, its one caller ignores it). Type 6: the cop type / model / offset set from the vehicle model
/// (Enforcer: SWAT 0x11D, Barracks: army 0x11F, FBI Rancher: FBI 0x11E, Ranger: city cop; set B for those
/// four, else set A, city cop); types 7..16: the model of the first gang (0..9) whose byte at
/// `[0xC0BC68] + gang` is non-zero (`ChooseGangPedModel(gang)`; -1 → 0 returned; no such gang → model 0).
/// `ratio = vehicle bound radius / default cop car bound radius` (f32). For `i < veh byte +0x4AE` (re-read
/// each round): position = `veh matrix * (offset[pair*2 + i] * ratio)`, a new ped (cop: `CCopPed(copType)`,
/// or `CCopPed(0)` when the special model is not loaded, with `SetCurrentWeapon(0x16)` for city cops; other
/// types: `CCivilianPed(type, model)` + `SetCurrentWeapon(0x16)`), `FindZCoorForPed`, placed with heading
/// -pi/2 (`SetRotate(0, 0, -pi/2)` and the position added back), cops: +0x79C = 1 and a primary
/// `CTaskComplexWanderCop` (wander ctor (1, ftol(rand frac * 8), 1, 0.5), vtable 0x859C80, fields zeroed;
/// rand site 0x46165D); every ped: secondary stand-still (0, 1, 0, 8.0) task, flags +0x46C |= 0x1200000,
/// bit 25 cleared for set B with pair 2 else set, +0x54C = now + 10000, +0x58C = the vehicle (registered
/// reference), +0x470 |= 0x40, clump alpha 0; gang peds: a delayed gang weapon (`rand` site 0x4617A3,
/// percent of the gang's three weapons at 0xC091F4, 25001 rounds) selected by slot; `CWorld::Add`, and a
/// `CEventScriptCommand(3, KillPedOnFoot(player, -1, 0, 0, 0, true))` added to its event group. The local
/// `CMatrix` (never used) is destroyed on both exits. NOT PORTED (ADR-002 fallback, decided at entry from pure
/// reads): offset indices beyond the 12 table entries (set A) or the 6 of set B, which read other stack
/// locals in the original.
#[hook(addr = 0x461170, conv = cdecl, name = "CRoadBlocks::GenerateRoadBlockPedsForCar", expect = "6A FF 68 07 7C 83 00 64",
       unverified = "popw4-gang-1", route = "split", hazards = "heap-writes", recs = "police-gang", fallback)]
pub fn generate_road_block_peds_for_car(veh: GPtr<u8>, pair: i32, ped_type: i32) -> u8 {
    let n = i32::from(rd::<u8>(veh, 0x4AE));
    let set_b = ped_type == 6 && road_block_cop_case(i32::from(rd::<i16>(veh, 0x22))).2;
    let limit = if set_b { 6 } else { 12 };
    if n > 0 && (!(0..=6).contains(&pair) || pair * 2 + n > limit) {
        return generate_road_block_peds_for_car_original(veh, pair, ped_type);
    }
    let mut local_matrix = [0u32; 0x12];
    let mut model: i32 = 0;
    let mut cop_type: u32 = 0;
    if !find_player_vehicle(-1, 0).is_null() {
        let _ = find_player_vehicle(-1, 0);
    }
    if ped_type == 6 {
        let (t, m, _) = road_block_cop_case(i32::from(rd::<i16>(veh, 0x22)));
        cop_type = t;
        model = m;
    } else if (7..=16).contains(&ped_type) {
        for gang in 0..10 {
            let counts = g::<u32>(0xC0BC68).read();
            if g::<u8>(counts.wrapping_add(gang) as usize).read() != 0 {
                model = choose_gang_ped_model(gang as i32);
                if model != -1 {
                    break;
                }
            }
        }
        if model == -1 {
            matrix_dtor(GPtr::from_raw(local_matrix.as_mut_ptr()).cast());
            return 0;
        }
    }
    let veh_r = rd::<f32>(get_col_model(veh), 0x24);
    let cop_r = rd::<f32>(
        rd::<GPtr<u8>>(model_info(get_default_cop_car_model(0)), 0x14),
        0x24,
    );
    let ratio = x(veh_r).fdiv_m(cop_r).to_f32();
    let mut i = 0;
    while i < i32::from(rd::<u8>(veh, 0x4AE)) {
        let k = (pair * 2 + i) as usize + if set_b { 6 } else { 0 };
        let o = ROAD_BLOCK_PED_OFFSETS[k];
        let mut off = [0f32; 3];
        off[2] = x(ratio).fmul_m(f32::from_bits(o[2])).to_f32();
        off[1] = x(ratio).fmul_m(f32::from_bits(o[1])).to_f32();
        off[0] = x(ratio).fmul_m(f32::from_bits(o[0])).to_f32();
        let mut res = [0u32; 3];
        let r = matrix_mul_vector(
            GPtr::from_raw(res.as_mut_ptr()),
            rd::<GPtr<u8>>(veh, 0x14),
            GPtr::from_raw(off.as_mut_ptr()),
        );
        let mut pos = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
        let ped = if ped_type == 6 {
            let special = cop_type != 0 && g::<u8>(0x8E4CD0 + model as u32 as usize * 20).read() != 1;
            let p = ped_new(0x7C4);
            let p = if p.is_null() {
                GPtr::NULL
            } else {
                cop_ped_ctor(p, if special { 0 } else { cop_type })
            };
            if special || cop_type == 0 {
                ped_set_current_weapon(p, 0x16);
            }
            p
        } else {
            let p = ped_new(0x79C);
            let p = if p.is_null() {
                GPtr::NULL
            } else {
                civilian_ped_ctor(p, ped_type as u32, model as u32)
            };
            ped_set_current_weapon(p, 0x16);
            p
        };
        find_z_coor_for_ped(GPtr::from_raw(pos.as_mut_ptr()));
        let z = x(f32::from_bits(pos[2])).to_f32();
        let y = x(f32::from_bits(pos[1])).to_f32();
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            wr::<f32>(ped, 8, y);
            wr::<u32>(ped, 4, pos[0]);
            wr::<f32>(ped, 0xC, z);
        } else {
            wr::<u32>(m, 0x30, pos[0]);
            wr::<f32>(rd::<GPtr<u8>>(ped, 0x14), 0x34, y);
            wr::<f32>(rd::<GPtr<u8>>(ped, 0x14), 0x38, z);
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            wr::<u32>(ped, 0x10, 0xBFC9_0FDB);
        } else {
            let saved = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
            matrix_set_rotate(m, 0, 0, 0xBFC9_0FDB);
            let m = rd::<GPtr<u8>>(ped, 0x14);
            for (j, s) in saved.iter().enumerate() {
                let o = 0x30 + j as isize * 4;
                wr::<f32>(m, o, x(*s).fadd_m(rd::<f32>(m, o)).to_f32());
            }
        }
        if ped_type == 6 {
            wr::<u8>(ped, 0x79C, 1);
            let t = task_new(0x50);
            if !t.is_null() {
                let r = rand_at(0x46165D) & 0xFFFF;
                let dir = xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(8.0).ftol() as u32;
                wander_ctor(t, 1, dir, 1, g::<u32>(0x86FCB0).read());
                wr::<u32>(t, 0, 0x859C80);
                for o in [0x28, 0x2C, 0x30, 0x38, 0x3C, 0x40, 0x44, 0x48] {
                    wr::<u32>(t, o, 0);
                }
                for o in [0x34, 0x35, 0x4C, 0x4D] {
                    wr::<u8>(t, o, 0);
                }
            }
            task_manager_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
        }
        let t = task_new(0x20);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            stand_still_ctor(t, 0, 1, 0, 0x4100_0000)
        };
        task_manager_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4, 0);
        let f = rd::<u32>(ped, 0x46C) | 0x0120_0000;
        wr::<u32>(ped, 0x46C, f);
        wr::<u32>(ped, 0x54C, g::<u32>(TIME_MS).read().wrapping_add(10000));
        let f = if set_b && pair == 2 {
            f & 0xFDFF_FFFF
        } else {
            f | 0x0200_0000
        };
        wr::<u32>(ped, 0x46C, f);
        wr::<u32>(ped, 0x58C, veh.addr() as u32);
        register_reference(veh, ped.byte_add(0x58C));
        wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x40);
        set_clump_alpha(rd::<GPtr<u8>>(ped, 0x18), 0);
        if ped_type != 6 {
            let r = rand_at(0x4617A3) & 0xFFFF;
            let pct = xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(100.0).ftol() as i32;
            let base = 0xC091F4usize.wrapping_add(((ped_type - 7) as isize * 16) as usize);
            let (w1, w2, w3) = (
                g::<i32>(base).read(),
                g::<i32>(base + 4).read(),
                g::<i32>(base + 8).read(),
            );
            let w = if w3 != 0 {
                if pct < 0x21 {
                    w1
                } else if pct < 0x42 {
                    w2
                } else {
                    w3
                }
            } else if w2 != 0 && pct >= 0x32 {
                w2
            } else {
                w1
            };
            if w != 0 {
                ped_give_delayed_weapon(ped, w, 0x61A9);
                ped_set_current_weapon_slot(ped, rd::<i32>(weapon_info_get(w, 1), 0x14));
            }
        }
        world_add(ped);
        let t = task_new(0x38);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            kill_ped_on_foot_ctor(t, crate::common::find_player_ped(-1), -1, 0, 0, 0, 1)
        };
        let mut ev = [0u32; 12];
        let evp = GPtr::from_raw(ev.as_mut_ptr());
        event_script_command_ctor(evp, 3, t, 0);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
        event_script_command_dtor(evp);
        i += 1;
    }
    matrix_dtor(GPtr::from_raw(local_matrix.as_mut_ptr()).cast());
    1
}

orig! {
    /// `CGameLogic::ClearSkip(bool)` (0x441560)
    fn clear_skip(b: u32) = cdecl 0x441560 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CGameLogic::DoWeaponStuffAtStartOf2PlayerGame(bool)` (0x4428B0)
    fn do_weapon_stuff_at_start_of_2player_game(b: u32) = cdecl 0x4428B0 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn weapon_info_get(ty: i32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CBaseModelInfo::AddRef()` (0x4C4BA0)
    fn model_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    fn model_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `CPed::RemoveWeaponAnims(eWeaponType, float blendDelta)` (0x5F0250)
    fn ped_remove_weapon_anims(p: GPtr<u8>, ty: i32, delta: f32) = thiscall 0x5F0250 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080)
    fn ped_give_weapon(p: GPtr<u8>, ty: i32, ammo: u32, b: u32) -> u32 = thiscall 0x5E6080 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280)
    fn ped_set_current_weapon(p: GPtr<u8>, ty: i32) = thiscall 0x5E6280 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0)
    fn ped_set_current_weapon_slot(p: GPtr<u8>, slot: i32) = thiscall 0x5E61F0 => unported;
    /// `CPlayerPed::MakeChangesForNewWeapon(eWeaponType)` (0x60B460)
    fn make_changes_for_new_weapon(p: GPtr<u8>, ty: i32) = thiscall 0x60B460 => unported;
    /// `CPlayerPed::ClearWeaponTarget()` (0x609C80)
    fn clear_weapon_target(p: GPtr<u8>) = thiscall 0x609C80 => unported;
    /// `CPed::RemoveWeaponModel(int32 model)` (0x5E3990)
    fn ped_remove_weapon_model(p: GPtr<u8>, model: i32) = thiscall 0x5E3990 => unported;
}

/// `FindPlayerPed(-1)`.
fn player() -> GPtr<u8> {
    crate::common::find_player_ped(-1)
}

/// `ped->m_aWeapons[slot]` (0x1C each at +0x5A0).
fn weapon(ped: GPtr<u8>, slot: i32) -> GPtr<u8> {
    ped.byte_add(0x5A0 + slot as isize * 0x1C)
}

/// `ped->m_aWeapons[ped->m_nActiveWeaponSlot]` (slot byte +0x718, signed).
fn cur_weapon(ped: GPtr<u8>) -> GPtr<u8> {
    weapon(ped, i32::from(rd::<i8>(ped, 0x718)))
}

/// `CDarkel::StartFrenzy(eWeaponType, int32 timeLimit, uint16 killsNeeded, int32 model, const char* text,
/// int32 model2, int32 model3, int32 model4, bool standardSound, bool needHeadShot)` (0x43D3B0):
/// `ClearSkip(false)`; `Status` 4 (+ `DoWeaponStuffAtStartOf2PlayerGame(false)`) in a co-op game, else 1;
/// the frenzy globals (`WeaponType` keeps the original weapon, 0x34 is handled as 0x1C below); the start
/// message is dropped (0x96A6D0 = 0, 0x969A48 = 1) when it is `TheText.Get("PAGE_00")`'s pointer; times. For a
/// weapon < 0x2F (signed): the interrupted weapon (active slot) is remembered (0x96A6DC) and its anims
/// removed (blend -1000), the frenzy weapon slot's type / total ammo saved (0x96A6D8 / 0x96A6D4, slot from
/// the info's dword +0x14), that saved type's model AddRef'd when non-zero, both players (`FindPlayerPed(0)`,
/// `(1)`) get 30000 rounds and select it with `MakeChangesForNewWeapon`; in a vehicle the player selects the
/// slot of `playerData+0x20` and the active weapon's clip becomes `min(totalAmmo, info clip)` (signed),
/// then `ClearWeaponTarget` (a tail jump in the original).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x43D3B0, conv = cdecl, name = "CDarkel::StartFrenzy", expect = "56 57 6A 00 E8 A7 41 00 00",
       unverified = "popw4-gang-1", route = "L2", hazards = "heap-writes", recs = "grove")]
pub fn start_frenzy(
    weapon_type: i32,
    time: i32,
    kills: u32,
    model: i32,
    text: u32,
    model2: i32,
    model3: i32,
    model4: i32,
    standard: u32,
    head_shot: u32,
) {
    clear_skip(0);
    let w = if weapon_type == 0x34 { 0x1C } else { weapon_type };
    if is_coop_game_going_on() != 0 {
        g::<u16>(0x96A704).write(4);
        do_weapon_stuff_at_start_of_2player_game(0);
    } else {
        g::<u16>(0x96A704).write(1);
    }
    g::<u32>(0x96A6EC).write(kills & 0xFFFF);
    g::<i32>(0x96A6FC).write(model);
    g::<i32>(0x96A6F8).write(model2);
    g::<i32>(0x96A6F0).write(model4);
    g::<i32>(0x96A700).write(weapon_type);
    g::<i32>(0x96A6F4).write(model3);
    g::<u32>(0x96A6D0).write(text);
    if g::<u32>(0x96A6D0).read() == text_get(gp(THE_TEXT), 0x859528) {
        g::<u32>(0x96A6D0).write(0);
        g::<u8>(0x969A48).write(1);
    } else {
        g::<u8>(0x969A48).write(0);
    }
    g::<u8>(0x969A4A).write(standard as u8);
    g::<u8>(0x969A49).write(head_shot as u8);
    g::<u32>(0x96A6E0).write(g::<u32>(TIME_MS).read());
    g::<i32>(0x96A6E8).write(time);
    g::<i32>(0x96A6E4).write(time / 1000);
    if w >= 0x2F {
        return;
    }
    let interrupted = rd::<i32>(cur_weapon(player()), 0);
    g::<i32>(0x96A6DC).write(interrupted);
    ped_remove_weapon_anims(player(), interrupted, -1000.0);
    let slot = rd::<i32>(weapon_info_get(w, 1), 0x14);
    g::<i32>(0x96A6D8).write(rd::<i32>(weapon(player(), slot), 0));
    let slot = rd::<i32>(weapon_info_get(w, 1), 0x14);
    g::<i32>(0x96A6D4).write(rd::<i32>(weapon(player(), slot), 0xC));
    let saved = g::<i32>(0x96A6D8).read();
    if saved != 0 {
        model_add_ref(model_info(rd::<i32>(weapon_info_get(saved, 1), 0xC)));
    }
    for i in 0..2 {
        if crate::common::find_player_ped(i).is_null() {
            continue;
        }
        ped_give_weapon(crate::common::find_player_ped(i), w, 30000, 1);
        ped_set_current_weapon(crate::common::find_player_ped(i), w);
        make_changes_for_new_weapon(crate::common::find_player_ped(i), w);
    }
    if sa_world::world::common::find_player_vehicle(-1, 0).is_null() {
        return;
    }
    let slot = i32::from(rd::<i8>(rd::<GPtr<u8>>(player(), 0x480), 0x20));
    ped_set_current_weapon_slot(player(), slot);
    let p1 = player();
    let clip = i32::from(rd::<i16>(
        weapon_info_get(rd::<i32>(cur_weapon(player()), 0), 1),
        0x20,
    ));
    let ammo = if rd::<i32>(cur_weapon(p1), 0xC) < clip {
        rd::<i32>(cur_weapon(player()), 0xC)
    } else {
        i32::from(rd::<i16>(
            weapon_info_get(rd::<i32>(cur_weapon(player()), 0), 1),
            0x20,
        ))
    };
    wr::<i32>(cur_weapon(player()), 8, ammo);
    clear_weapon_target(player());
}

/// `CDarkel::DealWithWeaponChangeAtEndOfFrenzy()` (0x43D7A0; `WeaponType` read through an obfuscated
/// fragment): for a frenzy weapon < 0x2F (0x34 as 0x1C): the saved type's model RemoveRef'd (when 0x96A6D8 is
/// non-zero), the frenzy weapon's slot (info byte +0x14) emptied (model removed, type / total / clip /
/// state = 0), its anims removed (blend -1000), its model RemoveRef'd while its ref count (int16 +8) > 0
/// (once), `playerData+0x20` = the interrupted weapon's slot, and the saved weapon given back
/// (`GiveWeapon(saved type, saved ammo, true)`). Then in a vehicle: the active weapon's model removed,
/// `playerData+0x20` = 0 when the ped's +0x610 is 0, else 4, `SetCurrentWeapon(slot)` and
/// `MakeChangesForNewWeapon(active weapon type)`.
#[hook(addr = 0x43D7A0, conv = cdecl, name = "CDarkel::DealWithWeaponChangeAtEndOfFrenzy", expect = "53 57 90 E9 E2 62 FC FF",
       unverified = "popw4-gang-1", route = "L2", hazards = "heap-writes", recs = "grove")]
pub fn deal_with_weapon_change_at_end_of_frenzy() {
    let mut w = g::<i32>(0x96A700).read();
    if w == 0x34 {
        w = 0x1C;
    }
    if w < 0x2F {
        let saved = g::<i32>(0x96A6D8).read();
        if saved != 0 {
            model_remove_ref(model_info(rd::<i32>(weapon_info_get(saved, 1), 0xC)));
        }
        let slot = i32::from(rd::<u8>(weapon_info_get(w, 1), 0x14));
        let ty = rd::<i32>(weapon(player(), slot), 0);
        let m = rd::<i32>(weapon_info_get(ty, 1), 0xC);
        ped_remove_weapon_model(player(), m);
        wr::<i32>(weapon(player(), slot), 0, 0);
        wr::<i32>(weapon(player(), slot), 0xC, 0);
        wr::<i32>(weapon(player(), slot), 8, 0);
        wr::<i32>(weapon(player(), slot), 4, 0);
        ped_remove_weapon_anims(player(), w, -1000.0);
        let mi = model_info(rd::<i32>(weapon_info_get(w, 1), 0xC));
        if rd::<i16>(mi, 8) > 0 {
            model_remove_ref(model_info(rd::<i32>(weapon_info_get(w, 1), 0xC)));
        }
        let pd = rd::<GPtr<u8>>(player(), 0x480);
        let s = rd::<u8>(weapon_info_get(g::<i32>(0x96A6DC).read(), 1), 0x14);
        wr::<u8>(pd, 0x20, s);
        let (ammo, ty) = (g::<u32>(0x96A6D4).read(), g::<i32>(0x96A6D8).read());
        ped_give_weapon(player(), ty, ammo, 1);
    }
    if sa_world::world::common::find_player_vehicle(-1, 0).is_null() {
        return;
    }
    let m = rd::<i32>(weapon_info_get(rd::<i32>(cur_weapon(player()), 0), 1), 0xC);
    ped_remove_weapon_model(player(), m);
    let v = if rd::<u32>(player(), 0x610) == 0 { 0 } else { 4 };
    wr::<u8>(rd::<GPtr<u8>>(player(), 0x480), 0x20, v);
    let slot = i32::from(rd::<i8>(rd::<GPtr<u8>>(player(), 0x480), 0x20));
    ped_set_current_weapon_slot(player(), slot);
    let ty = rd::<i32>(cur_weapon(player()), 0);
    make_changes_for_new_weapon(player(), ty);
}

orig! {
    /// `0x406E80` (stub into `.HOODLUM`; uncatalogued): the model `CreateAttackWave` requests for `Gang2`
    /// (`f(Gang2 + 0x12, 0, 0)`)
    fn gang2_request_model_o(a: i32, b: i32, c: i32) -> i32 = cdecl 0x406E80 => unported;
    /// `CCarCtrl::GenerateCarCreationCoors2(CVector, float, float, float, bool, float, float, CVector*,
    /// CNodeAddress*, CNodeAddress*, float*, bool, bool)` (0x424210; floats and the vector as raw dwords)
    #[allow(clippy::too_many_arguments)]
    fn generate_car_creation_coors2(px: u32, py: u32, pz: u32, dx: u32, dy: u32, dot: u32, ahead: u32, r1: u32, r2: u32, out: GPtr<f32>, n1: GPtr<u32>, n2: GPtr<u32>, frac: GPtr<f32>, restrict: u32, no_random: u32) -> u8 = cdecl 0x424210 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40; `TheCamera` 0xB6F028)
    fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<f32>, r: u32) -> u8 = thiscall 0x420D40 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90, ST0; raw bits)
    fn random_float_in_range(lo: u32, hi: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(e: GPtr<u8>, p: GPtr<f32>) = thiscall 0x4241C0 => unported;
    /// `CPool<CPed>::GetRef(CPed*)` (0x4442D0; ECX = the ped pool)
    fn ped_pool_get_ref(pool: GPtr<u8>, p: GPtr<u8>) -> u32 = thiscall 0x4442D0 => unported;
    /// `CCarCtrl::GenerateOneEmergencyServicesCar(uint32, CVector)` (0x42B7D0; raw dwords)
    fn generate_one_emergency_services_car(model: i32, x: u32, y: u32, z: u32) -> GPtr<u8> = cdecl 0x42B7D0 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystemGotoCoors(CVehicle*, const CVector&, bool, bool)` (0x42F870)
    fn join_car_with_road_system_goto_coors(v: GPtr<u8>, c: GPtr<f32>, a: u32, b: u32) -> u8 = cdecl 0x42F870 => unported;
    /// `CCarCtrl::SetUpDriverAndPassengersForVehicle(CVehicle*, int32, int32, bool, bool, int32)` (0x4217C0)
    fn set_up_driver_and_passengers_for_vehicle(veh: GPtr<u8>, pt: i32, min: i32, a: u32, b: u32, max: i32) = cdecl 0x4217C0 => crate::carctrl2::set_up_driver_and_passengers_for_vehicle;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    fn set_ped_decision_maker_type(intel: GPtr<u8>, t: i32) = thiscall 0x600B50 => unported;
    /// `CTaskSimpleGangDriveBy::CTaskSimpleGangDriveBy(CEntity*, const CVector*, float, int8, int8, bool)`
    /// (0x6217D0; float as raw bits)
    #[allow(clippy::too_many_arguments)]
    fn gang_drive_by_ctor(t: GPtr<u8>, target: GPtr<u8>, pos: u32, dist: u32, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x6217D0 => unported;
}

/// The 32-bit stack residue at `entry ESP + off` (footprint snapshot, window 0x1000).
fn residue32(off: i32) -> i32 {
    let mut v = 0u32;
    for k in 0..4 {
        v |= u32::from(sa_core::abi::footprint_residue_u8(off + k, 0x1000)) << (8 * k);
    }
    v as i32
}

/// `ms_aInfoForModel[m].m_nLoadState == LOADED` (`cmp byte [m*20 + 0x8E4CD0], 1`, wrapping like the bytes).
fn streamed(m: i32) -> bool {
    g::<u8>(0x8E4CD0u32.wrapping_add((m as u32).wrapping_mul(20)) as usize).read() == 1
}

/// The blip colour of gang `gang` (`r << 24 | g << 16 | b << 8 | 0xFF`).
fn gang_colour(gang: i32) -> u32 {
    let gang = gang as usize;
    (u32::from(g::<u8>(0x8D1344usize.wrapping_add(gang)).read()) << 24)
        | (u32::from(g::<u8>(0x8D1350usize.wrapping_add(gang)).read()) << 16)
        | (u32::from(g::<u8>(0x8D135Cusize.wrapping_add(gang)).read()) << 8)
        | 0xFF
}

/// `CGangWars::CreateAttackWave(int32 ferocity, int32 wave)` (0x444810, SEH frame; AL, the callers test it):
/// - 0 with `[0xB72914]` set, the player above z 950 (`test ah, 0x41; je`), the nodes of the player's
///   ±20 square not loaded. `Gang1`'s first group model requested (8) when none of it is streamed, the
///   model `0x406E80(Gang2 + 0x12, 0, 0)` when `Gang2 != Gang1` has none.
/// - weapon models / types by `ferocity` (jump table 0x4453B4: 0: 0x150/5 + 0x15A/0x16, 1: 0x15A/0x16 +
///   0x160/0x1C, 2: 0x160/0x1C + 0x161/0x1D, 3: 0x161/0x1D + 0x163/0x1E, 4: 0x163/0x1E twice, 5: 0x15C/0x18 +
///   0x163/0x1E); above 5 (unsigned) both models are the player ped's ADDRESS (local `[esp+0x44]`) and the two
///   weapon locals are left uninitialised: the port reads them from the stack residue (footprint, entry ESP
///   -0xC8 / -0xC4) and records their final values. With 0x96AB91 set: 0 → 0x150/5 twice, 1 → 0x15A/0x16
///   twice, else 0x174/0x20 twice. Either model not loaded: both requested (0), 0. Either gang without a
///   streamed ped: 0.
/// - for `Gang1` then `Gang2`: up to 20 `GenerateCarCreationCoors2(player, (1, 0), -1, ahead, 50, 50)` tries
///   (node outputs' area words set to 0xFFFF each try); a visible point (`IsSphereVisible(p, 7)`) is kept only
///   as the first fallback, an invisible one becomes the point and ends the search when more than 15 (3D,
///   `fsqrt`) from the previous wave point (initially 999999.875). Without a point: next gang. Else the
///   wave point is remembered, the spread direction is `normalise(py - Py, Px - px, 0) * 1.2`, and
///   `count = ftol((Difficulty * 0.3 + 0.7) * (ferocity + 3))` (2 with 0x96AB91; 10 for ferocity 5): peds at
///   `point + dir * k` (k = -(count / 2) * 2, +2 per ped; x / y each `± rand frac * 3 - 1.5`, rand sites
///   0x444D2C / 0x444D59), ground z (+2 above, +1.3) when `PickStreamedInPedForThisGang(gang)` succeeds:
///   `CCivilianPed(Gang1 + 7, model)` (Gang1 also for Gang2's peds, as the bytes), `SetPosn`, created by 2,
///   `CWorld::Add`, a `KillPedOnFoot(player, -1, 0, 0, 0, 2)` script event, weapon 1 when
///   `GetRandomNumberInRange(-0.3, 1.3) > Difficulty` else weapon 2 (5000 rounds, current), +0x71A =
///   `ftol(Difficulty * 25 + 70)`, +0x474 |= 0x3000, +0x540 / +0x544 = 120, an entity blip in the gang's
///   colour (scale 2); then a pickup (wave 0: health 0x8CD5C8 for Gang1 only, 1: 0x8CD5C0 for Gang1 only,
///   2: 0x8CD5C8 / 0x8CD5C0 by gang) at the point ± 2 (rand sites 0x444FBA / 0x444FE1), ground (+1) + 0.75.
/// - no ped created: 0. Unless 0x96AB91 or a drive-by car (0x96AB28) exists: a loaded `PickGangCar(Gang1)`
///   becomes `GenerateOneEmergencyServicesCar(model, player)` → 0x96AB28 (registered reference), joined to the
///   road system towards the player, its fields set (+0x42D |= 8, +0x3BA = 0x3D, +0x3D0 = 10, +0x41C =
///   player, +0x3B9 = 2, +0x36 status 3, +0x42C |= 0x40, +0x3DD = 0x1E), `SetUpDriverAndPassengers(Gang1 +
///   0xE, 1, 0, 0, 1)`, the driver's decision maker -1, each passenger a delayed Tec-9 (0x1C, 1500 rounds)
///   and a `CTaskSimpleGangDriveBy(player, 0, 100, 0x32, 8, even seat)` script event, the driver's blip (the
///   LAST gang's colour, scale 3) and +0x474 |= 0x2000. 1.
///
/// Its entry dword 0x444813 is a code key of the no-CD protection (`adc edi, [0x444813]` at 0x403A92),
/// redirected by sa-hook's keycells `READERS`.
#[hook(addr = 0x444810, conv = cdecl, name = "CGangWars::CreateAttackWave", expect = "6A FF E9 C2 22 FC FF",
       footprint = 0x1000,
       unverified = "popw4-gang-1", route = "split",
       hazards = "heap-writes,uninit-locals,stack-residue,x87-transcendental,entry-protection-read",
       recs = "grove,police-gang",
       note = "gang war attack waves; creates peds, a pickup and a drive-by car (split); weapon locals are residue for ferocity > 5 (unreachable from the three callers, which clamp it); entry dword is a keycell (0x403A92)")]
pub fn create_attack_wave(ferocity: i32, wave: i32) -> u8 {
    let mut w = (residue32(-0xC8), residue32(-0xC4));
    let mut touched = false;
    let r = create_attack_wave_body(ferocity, wave, &mut w, &mut touched);
    if touched {
        sa_core::abi::stack_footprint(&[
            (-0xC8, sa_core::abi::Fp::Val(w.0 as u32)),
            (-0xC4, sa_core::abi::Fp::Val(w.1 as u32)),
        ]);
    }
    r
}

#[allow(clippy::too_many_lines)]
fn create_attack_wave_body(ferocity: i32, wave: i32, w: &mut (i32, i32), touched: &mut bool) -> u8 {
    let mut pbuf = [0f32; 3];
    find_player_coors(GPtr::from_raw(pbuf.as_mut_ptr()), -1);
    let ppos = pbuf;
    let player_ped = crate::common::find_player_ped(-1);
    if g::<u32>(0xB72914).read() != 0 {
        return 0;
    }
    if !jne_mask(x(ppos[2]), x(950.0), 0x41) {
        return 0;
    }
    let (y1, y0) = (x(ppos[1]).fadd_m(20.0).to_f32(), x(ppos[1]).fsub_m(20.0).to_f32());
    let (x1, x0) = (x(ppos[0]).fadd_m(20.0).to_f32(), x(ppos[0]).fsub_m(20.0).to_f32());
    if are_nodes_loaded_for_area(gp(THE_PATHS), x0, x1, y0, y1) == 0 {
        return 0;
    }
    let mut gang1 = g::<i32>(GANG1).read();
    if ped_streamed_in_for_this_gang(gang1) == 0 {
        let grp = g::<i32>(0x8D2618usize.wrapping_add(gang1.wrapping_mul(12) as usize)).read();
        let m = g::<u16>(0xC0F358usize.wrapping_add(grp.wrapping_mul(0x2A) as usize)).read();
        request_model(i32::from(m), 8);
        gang1 = g::<i32>(GANG1).read();
    }
    let gang2 = g::<i32>(GANG2).read();
    if gang1 != gang2 && ped_streamed_in_for_this_gang(gang2) == 0 {
        let m = gang2_request_model_o(gang2 + 0x12, 0, 0);
        request_model(m, 8);
        gang1 = g::<i32>(GANG1).read();
    }
    *touched = true;
    let (mut m1, mut m2) = match ferocity {
        0 => {
            *w = (5, 0x16);
            (0x150, 0x15A)
        }
        1 => {
            *w = (0x16, 0x1C);
            (0x15A, 0x160)
        }
        2 => {
            *w = (0x1C, 0x1D);
            (0x160, 0x161)
        }
        3 => {
            *w = (0x1D, 0x1E);
            (0x161, 0x163)
        }
        4 => {
            *w = (0x1E, 0x1E);
            (0x163, 0x163)
        }
        5 => {
            *w = (0x18, 0x1E);
            (0x15C, 0x163)
        }
        _ => (player_ped.addr() as i32, player_ped.addr() as i32),
    };
    if g::<u8>(0x96AB91).read() != 0 {
        let (m, t) = match ferocity {
            0 => (0x150, 5),
            1 => (0x15A, 0x16),
            _ => (0x174, 0x20),
        };
        m1 = m;
        m2 = m;
        *w = (t, t);
    }
    if !streamed(m1) || !streamed(m2) {
        request_model(m1, 0);
        request_model(m2, 0);
        return 0;
    }
    if ped_streamed_in_for_this_gang(gang1) == 0 || ped_streamed_in_for_this_gang(g::<i32>(GANG2).read()) == 0
    {
        return 0;
    }
    let far = f32::from_bits(0x4974_23FE);
    let mut prev = [far, far, far];
    let mut point = [0f32; 3];
    let mut any = false;
    let mut cur_gang = 0;
    for gi in 0..2 {
        if gi > 0 {
            gang1 = g::<i32>(GANG1).read();
        }
        cur_gang = if gi == 0 { gang1 } else { g::<i32>(GANG2).read() };
        let mut found = false;
        let mut cand = [0f32; 3];
        let mut n1 = [0u32; 1];
        let mut n2 = [0u32; 1];
        let mut frac = [0f32; 1];
        for _ in 0..20 {
            n1[0] = (n1[0] & 0xFFFF_0000) | 0xFFFF;
            n2[0] = (n2[0] & 0xFFFF_0000) | 0xFFFF;
            let ok = generate_car_creation_coors2(
                ppos[0].to_bits(),
                ppos[1].to_bits(),
                ppos[2].to_bits(),
                0x3F80_0000,
                0,
                0xBF80_0000,
                1,
                50f32.to_bits(),
                50f32.to_bits(),
                GPtr::from_raw(cand.as_mut_ptr()),
                GPtr::from_raw(n1.as_mut_ptr()),
                GPtr::from_raw(n2.as_mut_ptr()),
                GPtr::from_raw(frac.as_mut_ptr()),
                0,
                1,
            );
            if ok == 0 {
                continue;
            }
            if camera_is_sphere_visible(gp(0xB6F028), GPtr::from_raw(cand.as_mut_ptr()), 7f32.to_bits()) != 0
            {
                if !found {
                    point = cand;
                    found = true;
                }
                continue;
            }
            let dz = x(prev[2]).fsub_m(cand[2]);
            let dy = x(prev[1]).fsub_m(cand[1]);
            let dx = x(prev[0]).fsub_m(cand[0]);
            point = cand;
            found = true;
            let d = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
            if !jne_mask(d, x(15.0), 0x41) {
                break;
            }
        }
        if !found {
            continue;
        }
        let da = x(point[0]).fsub_m(ppos[0]);
        let db = x(ppos[1]).fsub_m(point[1]);
        prev = point;
        let mut dir = [db.to_f32(), da.to_f32(), 0.0f32];
        vector_normalise(GPtr::from_raw(dir.as_mut_ptr()));
        for d in dir.iter_mut() {
            *d = x(*d).fmul_m(f32::from_bits(0x3F99_999A)).to_f32();
        }
        let diff = || x(g::<f32>(DIFFICULTY).read());
        let mut count = diff()
            .fmul_m(f32::from_bits(0x3E99_999A))
            .fadd_m(f32::from_bits(0x3F33_3333))
            .fmul(xi(ferocity.wrapping_add(3)))
            .ftol() as i32;
        if g::<u8>(0x96AB91).read() != 0 {
            count = 2;
        }
        if ferocity == 5 {
            count = 10;
        }
        if count > 0 {
            let mut k = (count / 2).wrapping_neg().wrapping_mul(2);
            for _ in 0..count {
                let kx = xi(k);
                let dzk = x(dir[2]).fmul(kx);
                let dyk = x(dir[1]).fmul(kx).to_f32();
                let dxk = x(dir[0]).fmul(kx).to_f32();
                let zf = x(point[2]).fadd(dzk).to_f32();
                let y0 = x(dyk).fadd_m(point[1]).to_f32();
                let x0 = x(dxk).fadd_m(point[0]).to_f32();
                let rf = |r: i32| xi(r).fmul_m(f32::from_bits(0x3800_0100)).fmul_m(3.0);
                let px = rf(rand_at(0x444D2C)).fadd_m(x0).fsub_m(1.5).to_f32();
                let py = rf(rand_at(0x444D59)).fadd_m(y0).fsub_m(1.5).to_f32();
                let mut model = [0i32; 1];
                if pick_streamed_in_ped_for_this_gang(cur_gang, GPtr::from_raw(model.as_mut_ptr())) == 0 {
                    k = k.wrapping_add(2);
                    continue;
                }
                let zarg = x(zf).fadd_m(2.0).to_f32();
                let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
                    px.to_bits(),
                    py.to_bits(),
                    zarg.to_bits(),
                    GPtr::NULL,
                    GPtr::NULL,
                ));
                let mut pos = [px, py, gz.fadd_m(f32::from_bits(0x3FA6_6666)).to_f32()];
                let p = ped_new(0x79C);
                let ped = if p.is_null() {
                    GPtr::NULL
                } else {
                    civilian_ped_ctor(p, (g::<i32>(GANG1).read() + 7) as u32, model[0] as u32)
                };
                placeable_set_posn(ped, GPtr::from_raw(pos.as_mut_ptr()));
                ped_set_char_created_by(ped, 2);
                world_add(ped);
                let t = task_new(0x38);
                let t = if t.is_null() {
                    GPtr::NULL
                } else {
                    kill_ped_on_foot_ctor(t, player_ped, -1, 0, 0, 0, 2)
                };
                let mut ev = [0u32; 12];
                let evp = GPtr::from_raw(ev.as_mut_ptr());
                event_script_command_ctor(evp, 3, t, 0);
                event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
                let rnd = X87::from_f64_exact(random_float_in_range(0xBE99_999A, 0x3FA6_6666));
                let wt = if jne_mask(rnd, diff(), 0x41) { w.1 } else { w.0 };
                ped_give_weapon(ped, wt, 5000, 0);
                ped_set_current_weapon(ped, wt);
                wr::<u8>(ped, 0x71A, diff().fmul_m(25.0).fadd_m(70.0).ftol() as u8);
                wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x3000);
                wr::<u32>(ped, 0x544, 0x42F0_0000);
                wr::<u32>(ped, 0x540, 0x42F0_0000);
                let colour = gang_colour(cur_gang);
                let handle = ped_pool_get_ref(g::<GPtr<u8>>(0xB74490).read(), ped);
                let blip = set_entity_blip(2, handle, colour, 2, 0x859910);
                change_blip_scale(blip, 2);
                change_blip_colour(blip, colour);
                any = true;
                event_script_command_dtor(evp);
                k = k.wrapping_add(2);
            }
        }
        let rf = |r: i32| xi(r).fmul_m(f32::from_bits(0x3800_0100)).fmul_m(4.0);
        let kx = rf(rand_at(0x444FBA)).fadd_m(point[0]).fsub_m(2.0).to_f32();
        let ky = rf(rand_at(0x444FE1)).fadd_m(point[1]).fsub_m(2.0).to_f32();
        let zarg = x(point[2]).fadd_m(1.0).to_f32();
        let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
            kx.to_bits(),
            ky.to_bits(),
            zarg.to_bits(),
            GPtr::NULL,
            GPtr::NULL,
        ));
        let kz = gz.fadd_m(0.75).to_f32();
        let model = match wave {
            0 if gi == 0 => Some(g::<u16>(0x8CD5C8).read()),
            1 if gi == 0 => Some(g::<u16>(0x8CD5C0).read()),
            2 => Some(g::<u16>(if gi == 0 { 0x8CD5C8 } else { 0x8CD5C0 }).read()),
            _ => None,
        };
        if let Some(m) = model {
            generate_new_pickup(
                kx.to_bits(),
                ky.to_bits(),
                kz.to_bits(),
                u32::from(m),
                5,
                0,
                0,
                0,
                0,
            );
        }
    }
    if !any {
        return 0;
    }
    if g::<u8>(0x96AB91).read() != 0 || g::<u32>(0x96AB28).read() != 0 {
        return 1;
    }
    let model = pick_gang_car(g::<i32>(GANG1).read());
    if model < 0 || !streamed(model) {
        return 1;
    }
    let mut cbuf = [0u32; 3];
    let c = find_player_coors(GPtr::from_raw(cbuf.as_mut_ptr()).cast(), -1).cast::<u32>();
    let veh = generate_one_emergency_services_car(
        model,
        c.read(),
        c.wrapping_index(1).read(),
        c.wrapping_index(2).read(),
    );
    g::<u32>(0x96AB28).write(veh.addr() as u32);
    if veh.is_null() {
        return 1;
    }
    register_reference(veh, gp(0x96AB28));
    let mut cbuf = [0f32; 3];
    let c = find_player_coors(GPtr::from_raw(cbuf.as_mut_ptr()), -1);
    let car = || g::<GPtr<u8>>(0x96AB28).read();
    join_car_with_road_system_goto_coors(car(), c, 0, 0);
    wr::<u8>(car(), 0x42D, rd::<u8>(car(), 0x42D) | 8);
    wr::<u8>(car(), 0x3BA, 0x3D);
    wr::<u8>(car(), 0x3D0, 0xA);
    let pp = crate::common::find_player_ped(-1);
    wr::<u32>(car(), 0x41C, pp.addr() as u32);
    wr::<u8>(car(), 0x3B9, 2);
    wr::<u8>(car(), 0x36, (rd::<u8>(car(), 0x36) & 7) | 0x18);
    wr::<u8>(car(), 0x42C, rd::<u8>(car(), 0x42C) | 0x40);
    wr::<u8>(car(), 0x3DD, 0x1E);
    set_up_driver_and_passengers_for_vehicle(car(), g::<i32>(GANG1).read() + 0xE, 1, 0, 0, 1);
    let driver = rd::<GPtr<u8>>(car(), 0x460);
    if !driver.is_null() {
        set_ped_decision_maker_type(rd::<GPtr<u8>>(driver, 0x47C), -1);
    }
    for i in 0..8u32 {
        let slot = 0x464 + i as isize * 4;
        let p = rd::<GPtr<u8>>(car(), slot);
        if p.is_null() {
            continue;
        }
        ped_give_delayed_weapon(p, 0x1C, 0x5DC);
        ped_set_current_weapon(rd::<GPtr<u8>>(car(), slot), 0x1C);
        let t = task_new(0x44);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            let even = u32::from(!(i as u8) & 1);
            gang_drive_by_ctor(
                t,
                crate::common::find_player_ped(-1),
                0,
                100f32.to_bits(),
                0x32,
                8,
                even,
            )
        };
        let mut ev = [0u32; 12];
        let evp = GPtr::from_raw(ev.as_mut_ptr());
        event_script_command_ctor(evp, 3, t, 0);
        let p = rd::<GPtr<u8>>(car(), slot);
        event_group_add(rd::<GPtr<u8>>(p, 0x47C).byte_add(0x68), evp, 0);
        event_script_command_dtor(evp);
    }
    let colour = gang_colour(cur_gang);
    let handle = ped_pool_get_ref(g::<GPtr<u8>>(0xB74490).read(), rd::<GPtr<u8>>(car(), 0x460));
    let blip = set_entity_blip(2, handle, colour, 2, 0x859908);
    change_blip_scale(blip, 3);
    change_blip_colour(blip, colour);
    let d = rd::<GPtr<u8>>(car(), 0x460);
    wr::<u32>(d, 0x474, rd::<u32>(d, 0x474) | 0x2000);
    1
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[
    HOOK_CREATE_ATTACK_WAVE,
    HOOK_PICK_ZONE_TO_ATTACK,
    HOOK_CHEER_VICTORY,
    HOOK_START_DEFENSIVE_GANG_WAR,
    HOOK_START_OFFENSIVE_GANG_WAR,
    HOOK_CREATE_DEFENDING_GROUP,
    HOOK_GET_ROAD_BLOCK_NODE_INFO,
    HOOK_CLEAR_SPACE_FOR_ROAD_BLOCK_OBJECT,
    HOOK_GENERATE_ROAD_BLOCK_PEDS_FOR_CAR,
    HOOK_START_FRENZY,
    HOOK_DEAL_WITH_WEAPON_CHANGE_AT_END_OF_FRENZY,
];
