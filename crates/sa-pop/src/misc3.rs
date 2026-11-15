//! CGangWars, CRoadBlocks, CTrafficLights, CDarkel, car generators (PORT-POP-3, helper B). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::{find_player_coors, g, jne_mask, jp5, model_info, rand_at, rd, wr, x, xi};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

use sa_core::bindings::display_script_stat_update_message;
use sa_world::bindings::matrix_dtor;
use sa_world::bindings::stats_increment_stat as increment_stat;
use sa_world::bindings::find_player_wanted;
use sa_world::bindings::find_objects_kinda_colliding;
use sa_world::bindings::is_player_on_a_mission;
use sa_world::bindings::set_char_created_by as ped_set_char_created_by;
use sa_core::bindings::audio_report_frontend_event as report_frontend_audio_event;
use crate::bindings::end_gang_war;
use crate::bindings::check_damaged_weapon_type;
use sa_world::bindings::decrement_stat;
use sa_world::bindings::membership_is_member;
use sa_world::bindings::find_player_ped;
use sa_world::bindings::find_player_vehicle;
orig! {
    /// `CGangWars::UpdateTerritoryUnderControlPercentage()` (0x443DE0)
    fn update_territory_under_control_percentage() = cdecl 0x443DE0 => crate::misc2::update_territory_under_control_percentage;
    /// `CDarkel::ThisPedShouldBeKilledForFrenzy(const CPed&)` (0x43D2F0)
    pub fn this_ped_should_be_killed_for_frenzy(ped: GPtr<u8>) -> u8 = cdecl 0x43D2F0 => crate::misc2::this_ped_should_be_killed_for_frenzy;
    /// `CGangWars::AddKillToProvocation(ePedType)` (0x443950)
    fn add_kill_to_provocation(pt: i32) = cdecl 0x443950 => crate::misc2::add_kill_to_provocation;
    /// `CPopCycle::PlayerKilledADealer()` (0x610490)
    fn player_killed_a_dealer() = cdecl 0x610490 => crate::population2::player_killed_a_dealer;
}

use sa_world::world::common::entity_pos_f as entity_pos;

/// `(float)(int16)p[off] * 0.125f` (`fild` + `fmul [0x858C48]`).
fn ncoord(p: GPtr<u8>, off: isize) -> X87 {
    xi(i32::from(rd::<i16>(p, off))).fmul_m(0.125)
}

/// `CModelInfo::ms_modelInfoPtrs[mi]->m_pColModel` (+0x14).
fn col_model(mi: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(model_info(mi), 0x14)
}

/// `CTimer::m_FrameCounter` (0xB7CB4C).
fn frame_counter() -> u32 {
    g::<u32>(0xB7CB4C).read()
}

/// `CGangWars::Update()` (0x446610), partial port (ADR-002 fallback). PORTED: the prologue and the
/// "gang wars inactive" exit: `EndGangWar(true)` when the player is on a mission, the previous-frame mission
/// flag 0x96ABA0 is clear and `State` (0x96AB94) is 0; `0x96ABA0 = IsPlayerOnAMission()`; return when
/// 0xB5F852 is set; `UpdateTerritoryUnderControlPercentage()` when the frame counter byte 0xB7CB4C is 0x38;
/// return when `bGangWarsActive` (0x96AB90) is 0. NOT PORTED (fallback to the whole original body, decided at
/// entry): `bGangWarsActive != 0` (co-op test, the war timer, the state machine, attack waves). None of the
/// prologue's callees writes 0x96AB90, so the entry test selects the same path the original takes at 0x446662.
#[hook(addr = 0x446610, conv = cdecl, name = "CGangWars::Update", expect = "83 EC 34 53 E8 37 E7 01 00", fallback)]
pub fn gang_wars_update() {
    if g::<u8>(0x96AB90).read() != 0 {
        return gang_wars_update_original();
    }
    if is_player_on_a_mission() != 0 && g::<u8>(0x96ABA0).read() == 0 && g::<u32>(0x96AB94).read() == 0 {
        end_gang_war(1);
    }
    let m = is_player_on_a_mission() != 0;
    g::<u8>(0x96ABA0).write(u8::from(m));
    if g::<u8>(0xB5F852).read() != 0 {
        return;
    }
    if g::<u8>(0xB7CB4C).read() == 0x38 {
        update_territory_under_control_percentage();
    }
}

orig! {
    /// `CRoadBlocks::CreateRoadBlockBetween2Points(CVector a, CVector b, uint32 gang)` (0x4619C0, cdecl, both vectors
    /// by value: six stack dwords, then the flag)
    fn create_road_block_between_2_points(ax: u32, ay: u32, az: u32, bx: u32, by: u32, bz: u32, gang: u32) = cdecl 0x4619C0 => crate::road_block_between::create_road_block_between_2_points;
    /// `CRoadBlocks::GetRoadBlockNodeInfo(CNodeAddress, float&, CVector&)` (0x460EE0, AL)
    fn road_block_node_info(node: u32, width: GPtr<f32>, dir: GPtr<f32>) -> u8 = cdecl 0x460EE0 => crate::misc5::get_road_block_node_info;
}

use crate::bindings::get_position as path_node_get_position;
use sa_core::bindings::vector_normalise;
use sa_math::fpu::{fabs, fcom_ne, ge_or_nan, le_or_nan};
use sa_world::bindings::{process_line_of_sight, vector_add, vector_scale, vector_sub};

/// `CRoadBlocks::InOrOut[325]` (0xA43438)
const IN_OR_OUT: usize = 0xA43438;
/// `CRoadBlocks::NumRoadBlocks` (0xA43580; the loop bound reads it through the no-CD protection's obfuscated pointer
/// at 0x402033, which decodes to this address)
const NUM_ROAD_BLOCKS: usize = 0xA43580;
/// `CRoadBlocks::bGenerateDynamicRoadBlocks` (0xA43584)
const GENERATE_DYNAMIC: usize = 0xA43584;
/// `CRoadBlocks::RoadBlockNodes[325]` (0xA435A0, `CNodeAddress`: area in the low word, node in the high word)
const ROAD_BLOCK_NODES: usize = 0xA435A0;
/// `CRoadBlocks::aScriptRoadBlocks[16]` (0xA43AB8, stride 0x1C: corner A +0, corner B +0xC, active +0x18, safe to create
/// +0x19, gang +0x1A)
const SCRIPT_ROAD_BLOCKS: usize = 0xA43AB8;

type V3 = [f32; 3];

use sa_core::field::vec3f_ptr as vp;

fn v3(p: GPtr<u8>, i: i32) -> f32 {
    p.cast::<f32>().wrapping_index(i).read()
}

/// `ThePaths.m_pPathNodes[area] + node * 0x1C` of a node address (null when the area is not loaded).
fn path_node(node: u32) -> GPtr<u8> {
    let base = g::<u32>(0x96F854 + (node & 0xFFFF) as usize * 4).read();
    if base == 0 {
        GPtr::NULL
    } else {
        GPtr::from_addr(base.wrapping_add((node >> 16) * 0x1C) as usize)
    }
}

/// `CPathNode::GetPosition` into `buf`.
fn node_pos(n: GPtr<u8>, buf: &mut V3) -> GPtr<u8> {
    path_node_get_position(n, GPtr::from_raw(buf.as_mut_ptr())).cast()
}

/// `CreateRoadBlockBetween2Points(a, b, gang)` with both vectors copied onto the stack (b first, as the bytes).
fn create_between(a: GPtr<u8>, b: GPtr<u8>, gang: u32) {
    let (a, b) = (a.cast::<u32>(), b.cast::<u32>());
    let (bx, by, bz) = (b.read(), b.wrapping_index(1).read(), b.wrapping_index(2).read());
    let (ax, ay, az) = (a.read(), a.wrapping_index(1).read(), a.wrapping_index(2).read());
    create_road_block_between_2_points(ax, ay, az, bx, by, bz, gang);
}

/// `pos(n) + k * dir` (`operator*(float, CVector)` 0x40FE90, `GetPosition`, `operator+` 0x40FE30) or, with `minus`,
/// `pos(n) - k * dir` (`operator-` 0x40FE60), into `out`.
fn offset(n: GPtr<u8>, k: f32, dir: &mut V3, minus: bool, out: &mut V3) -> GPtr<u8> {
    let (mut s, mut p) = ([0f32; 3], [0f32; 3]);
    let sc = vector_scale(vp(&mut s), k, vp(dir));
    let pos = node_pos(n, &mut p);
    if minus {
        vector_sub(vp(out), pos, sc)
    } else {
        vector_add(vp(out), pos, sc)
    }
}

/// `CRoadBlocks::GenerateRoadBlocks()` (0x4629E0, SEH frame for two stack `CMatrix` locals that are only destroyed).
/// Whole port since FRONTIER-2 turn 2 lane A (the ADR-002 fallback is gone). With a wanted level (`FindPlayerWanted(-1)`
/// word +0x1C, the road block chance) and a player vehicle: [`dynamic_road_blocks`]; else `bGenerateDynamicRoadBlocks`
/// = 0. Then the script road block of this frame (`aScriptRoadBlocks[m_FrameCounter & 15]`), when active: its
/// centre `(A + B) * 0.5` (x summed into an f32 first, y / z multiplied on the x87 stack) 90 or more from the player
/// (3D, NaN included) marks it safe to create; nearer and safe, it is created between its corners with its gang flag
/// and marked unsafe.
#[hook(addr = 0x4629E0, conv = cdecl, name = "CRoadBlocks::GenerateRoadBlocks", expect = "6A FF 68 73 7C 83 00")]
pub fn generate_road_blocks() {
    // the two stack CMatrix locals (m_pAttachMatrix and m_bOwnsAttachedMatrix cleared by the prologue)
    let mut m1 = [0u32; 0x12];
    let mut m2 = [0u32; 0x12];
    if rd::<u16>(find_player_wanted(-1), 0x1C) != 0 && !find_player_vehicle(-1, 0).is_null() {
        dynamic_road_blocks();
    } else {
        g::<u8>(GENERATE_DYNAMIC).write(0);
    }
    script_road_block();
    matrix_dtor(GPtr::from_raw(m1.as_mut_ptr()).cast());
    matrix_dtor(GPtr::from_raw(m2.as_mut_ptr()).cast());
}

/// The wanted-level road blocks of 0x4629E0 (0x462A53..0x462D93). The first frame sets every `InOrOut` byte (325) and
/// `bGenerateDynamicRoadBlocks`; each frame handles the sixteenth `[(F & 15) * 325 / 16, ((F & 15) + 1) * 325 / 16)` of
/// the nodes, capped by `NumRoadBlocks` (re-read every iteration).
fn dynamic_road_blocks() {
    if g::<u8>(GENERATE_DYNAMIC).read() == 0 {
        for k in 0..0x145 {
            g::<u8>(IN_OR_OUT + k).write(1);
        }
    }
    let f = (frame_counter() & 0xF) as i32 * 0x145;
    let mut i = f / 16;
    let hi = (f + 0x145) / 16;
    g::<u8>(GENERATE_DYNAMIC).write(1);
    loop {
        let n = g::<i32>(NUM_ROAD_BLOCKS).read();
        if i >= n.min(hi) {
            break;
        }
        road_block_node(i as usize);
        i += 1;
    }
}

/// One node of [`dynamic_road_blocks`]. Skipped while its area is not loaded. Out of range of the player (x or y
/// difference not inside (-90, 90): the first test on the x87 value, the second on its f32 store; or the 2D distance
/// of the stored differences 90 or more, NaN included) it is marked out (`InOrOut = 0`). An out node coming in is
/// marked in and, when `rand() & 127` (return address 0x462BE9) is below the chance and its road block info exists:
/// with a path width `w` (node +0x16, `w / 16`) two blocks, `pos + dir * (width / 2 + w)` to `pos + dir * w` and
/// `pos - dir * w` to `pos - dir * (width / 2 + w)`; without, [`join_node`].
fn road_block_node(i: usize) {
    let node = g::<u32>(ROAD_BLOCK_NODES + i * 4).read();
    let main = path_node(node);
    if main.is_null() {
        return;
    }
    let mut t = [0f32; 3];
    let out = |i: usize| g::<u8>(IN_OR_OUT + i).write(0);
    let nx = xi(i32::from(rd::<i16>(main, 8))).fmul_m(0.125).to_f32();
    let p = find_player_coors(GPtr::from_raw(t.as_mut_ptr()), -1);
    let dx = x(p.read()).fsub_m(nx);
    let dxs = dx.to_f32();
    if le_or_nan(dx, x(-90.0)) || ge_or_nan(x(dxs), x(90.0)) {
        return out(i);
    }
    let ny = xi(i32::from(rd::<i16>(main, 0xA))).fmul_m(0.125).to_f32();
    let p = find_player_coors(GPtr::from_raw(t.as_mut_ptr()), -1);
    let dy = x(p.wrapping_index(1).read()).fsub_m(ny);
    let dys = dy.to_f32();
    if le_or_nan(dy, x(-90.0)) || ge_or_nan(x(dys), x(90.0)) {
        return out(i);
    }
    let d = x(dys).fmul_m(dys).fadd(x(dxs).fmul_m(dxs)).fsqrt();
    if ge_or_nan(d, x(90.0)) {
        return out(i);
    }
    if g::<u8>(IN_OR_OUT + i).read() != 0 {
        return;
    }
    g::<u8>(IN_OR_OUT + i).write(1);
    let chance = i32::from(rd::<u16>(find_player_wanted(-1), 0x1C));
    if rand_at(0x462BE9) & 0x7F >= chance {
        return;
    }
    let mut width = 0f32;
    let mut dir = [0f32; 3];
    if road_block_node_info(node, GPtr::from_raw(&mut width), GPtr::from_raw(dir.as_mut_ptr())) == 0 {
        return;
    }
    let w8 = rd::<u8>(main, 0x16);
    if w8 == 0 {
        return join_node(i, main, width, &mut dir);
    }
    let wf = xi(i32::from(w8)).fmul_m(0.0625).to_f32();
    let (mut a, mut b) = ([0f32; 3], [0f32; 3]);
    let p1 = offset(main, wf, &mut dir, false, &mut a);
    let h = x(width).fmul_m(0.5).fadd_m(wf).to_f32();
    let p2 = offset(main, h, &mut dir, false, &mut b);
    create_between(p2, p1, 0);
    let (mut a, mut b) = ([0f32; 3], [0f32; 3]);
    let q2 = offset(main, h, &mut dir, true, &mut a);
    let q1 = offset(main, wf, &mut dir, true, &mut b);
    create_between(q1, q2, 0);
}

/// A road block node without a path width (0x462EBC): the first other node `j` (in index order, `NumRoadBlocks`
/// re-read every iteration) that is out, loaded, within (-30, 30) in x and y (each difference tested on the x87
/// stack), whose road block info has exactly the same width and a direction with `|dot| > 0.7` (`fabs`; summed x, z,
/// y), and that the main node sees (`ProcessLineOfSight` from 1 above the main node to 1 above it, buildings only)
/// gets the block `pos(j) + d * width / 2` to `pos(main) - d * width / 2` (`d` = their normalised difference, written
/// over the direction) and is marked in; the search then ends. When no node matches, one block across the main node:
/// `pos + dir * width / 2` to `pos - dir * width / 2`. gta-reversed differs: no `fabs`, the search goes on after a
/// match, and the block across the node is made only for the index `NumRoadBlocks`.
fn join_node(i: usize, main: GPtr<u8>, width: f32, dir: &mut V3) {
    let mut j = 0usize;
    while (j as i32) < g::<i32>(NUM_ROAD_BLOCKS).read() {
        if j != i && try_join(j, main, width, dir) {
            return;
        }
        j += 1;
    }
    let h = x(width).fmul_m(0.5).to_f32();
    let (mut a, mut b) = ([0f32; 3], [0f32; 3]);
    let e = offset(main, h, dir, true, &mut a);
    let f = offset(main, h, dir, false, &mut b);
    create_between(f, e, 0);
}

/// One candidate of [`join_node`]; true when it made the block.
fn try_join(j: usize, main: GPtr<u8>, width: f32, dir: &mut V3) -> bool {
    let nd = g::<u32>(ROAD_BLOCK_NODES + j * 4).read();
    if g::<u8>(IN_OR_OUT + j).read() != 0 {
        return false;
    }
    let other = path_node(nd);
    if other.is_null() {
        return false;
    }
    let (mut a, mut b) = ([0f32; 3], [0f32; 3]);
    for k in 0..2 {
        let pa = node_pos(main, &mut a);
        let pb = node_pos(other, &mut b);
        let d = x(v3(pa, k)).fsub_m(v3(pb, k));
        if le_or_nan(d, x(-30.0)) || ge_or_nan(d, x(30.0)) {
            return false;
        }
    }
    let mut w2 = 0f32;
    let mut dir2 = [0f32; 3];
    if road_block_node_info(nd, GPtr::from_raw(&mut w2), GPtr::from_raw(dir2.as_mut_ptr())) == 0 {
        return false;
    }
    if fcom_ne(x(width), x(w2)) {
        return false;
    }
    let dot = x(dir2[0]).fmul_m(dir[0]).fadd(x(dir2[2]).fmul_m(dir[2])).fadd(x(dir2[1]).fmul_m(dir[1]));
    if le_or_nan(fabs(dot), x(0.7)) {
        return false;
    }
    let (mut up1, mut up2, mut s) = ([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0f32; 3]);
    let r = vector_add(vp(&mut s), node_pos(main, &mut a), vp(&mut up1));
    let mut pa = [v3(r, 0), v3(r, 1), v3(r, 2)];
    let r = vector_add(vp(&mut s), node_pos(other, &mut b), vp(&mut up2));
    let mut pb = [v3(r, 0), v3(r, 1), v3(r, 2)];
    let mut cp = [0u32; 0xB];
    let mut ent = 0u32;
    let cpp = GPtr::from_raw(cp.as_mut_ptr()).cast();
    if process_line_of_sight(vp(&mut pa), vp(&mut pb), cpp, GPtr::from_raw(&mut ent).cast(), 1, 0, 0, 0, 0, 0, 0, 0) != 0 {
        return false;
    }
    let m = node_pos(main, &mut a);
    let o = node_pos(other, &mut b);
    let d = vector_sub(vp(&mut s), o, m);
    *dir = [v3(d, 0), v3(d, 1), v3(d, 2)];
    vector_normalise(GPtr::from_raw(dir as *mut V3));
    let h = x(width).fmul_m(0.5).to_f32();
    let (mut e, mut f) = ([0f32; 3], [0f32; 3]);
    let e = offset(main, h, dir, true, &mut e);
    let f = offset(other, h, dir, false, &mut f);
    create_between(f, e, 0);
    g::<u8>(IN_OR_OUT + j).write(1);
    true
}

/// The script road block of 0x4629E0 (0x462D9F); see [`generate_road_blocks`].
fn script_road_block() {
    let b = SCRIPT_ROAD_BLOCKS + ((frame_counter() & 0xF) * 0x1C) as usize;
    if g::<u8>(b + 0x18).read() == 0 {
        return;
    }
    let f = |o: usize| g::<f32>(b + o).read();
    let sz = x(f(8)).fadd_m(f(0x14));
    let sy = x(f(4)).fadd_m(f(0x10));
    let sx = x(f(0xC)).fadd_m(f(0)).to_f32();
    let cz = sz.fmul_m(0.5).to_f32();
    let cy = sy.fmul_m(0.5).to_f32();
    let cx = x(sx).fmul_m(0.5).to_f32();
    let mut t = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(t.as_mut_ptr()), -1);
    let dz = x(cz).fsub_m(p.wrapping_index(2).read());
    let dy = x(cy).fsub_m(p.wrapping_index(1).read());
    let dx = x(cx).fsub_m(p.read());
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
    if ge_or_nan(d, x(90.0)) {
        g::<u8>(b + 0x19).write(1);
        return;
    }
    if g::<u8>(b + 0x19).read() == 0 {
        return;
    }
    let gang = u32::from(g::<u8>(b + 0x1A).read());
    create_between(GPtr::from_addr(b), GPtr::from_addr(b + 0xC), gang);
    g::<u8>(b + 0x19).write(0);
}

/// `CCarGenerator::CheckForBlockage(int32 model)` (0x6F32E0, thiscall, `ret 4`, AL): up to 8 vehicles / peds
/// within the model's bounding radius (2.0 without a col model) of the generator position (2D test) whose
/// vertical extent overlaps the model's (`entZ + ent.max.z + 1 > genZ + min.z` and `entZ + ent.min.z - 1 <
/// genZ + max.z`; ±1.0 around the generator z without a col model) block it: bit 0 of +0xD is set and 1
/// returned. x87 compares as the bytes (`test ah, 0x41; jne` / `test ah, 5; jp`).
#[hook(addr = 0x6F32E0, conv = thiscall, name = "CCarGenerator::CheckForBlockage", expect = "8B 44 24 04 83 EC 38", this = "CCarGenerator")]
pub fn check_for_blockage(this: GPtr<u8>, model: i32) -> u8 {
    let gen_col = col_model(model);
    let radius = if gen_col.is_null() {
        2.0
    } else {
        rd::<f32>(gen_col, 0x24)
    };
    let mut pos = [
        ncoord(this, 4).to_f32(),
        ncoord(this, 6).to_f32(),
        ncoord(this, 8).to_f32(),
    ];
    let mut count = [0i16; 2];
    let mut out = [0u32; 8];
    find_objects_kinda_colliding(
        GPtr::from_raw(pos.as_mut_ptr()),
        radius,
        1,
        GPtr::from_raw(count.as_mut_ptr()),
        8,
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(out.as_mut_ptr())),
        0,
        1,
        1,
        0,
        0,
    );
    for &e in out.iter().take(count[0].max(0) as usize) {
        let e = GPtr::<u8>::from_addr(e as usize);
        let ecol = col_model(i32::from(rd::<i16>(e, 0x22)));
        let z = ncoord(this, 8);
        let ez = || x(entity_pos(e).wrapping_index(2).read());
        let (lo, hi_add) = if gen_col.is_null() {
            (z.fsub_m(1.0), 1.0)
        } else {
            let c = col_model(model);
            (z.fadd_m(rd::<f32>(c, 8)), rd::<f32>(c, 0x14))
        };
        if jne_mask(ez().fadd_m(rd::<f32>(ecol, 0x14)).fadd_m(1.0), lo, 0x41) {
            continue;
        }
        let hi = z.fadd_m(hi_add);
        if !jp5(ez().fadd_m(rd::<f32>(ecol, 8)).fsub_m(1.0), hi) {
            wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 1);
            return 1;
        }
    }
    0
}

use sa_core::bindings::add_big_message;
use sa_core::bindings::ascii_to_gxt_char;
use sa_core::bindings::font_print_string;
use sa_core::bindings::font_set_background;
use sa_core::bindings::font_set_color;
use sa_core::bindings::font_set_edge;
use sa_core::bindings::font_set_font_style;
use sa_core::bindings::font_set_orientation;
use sa_core::bindings::font_set_proportional;
use sa_core::bindings::text_get;
use sa_core::bindings::font_set_drop_color;
use sa_core::bindings::font_set_right_justify_wrap;
use sa_core::bindings::font_set_scale;
use sa_core::bindings::font_set_wrapx;
use sa_core::bindings::rgba_ctor;
use sa_stream::bindings::hud_colours_get_rgb;
use crate::bindings::get_y_pos_based_on_health_st0 as get_y_pos_based_on_health;
orig! {
    /// CRT `sprintf` (0x821BB5) with two int arguments
    fn sprintf2(buf: GPtr<u8>, fmt: GPtr<u8>, a: u32, b: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf` (0x821BB5) with one int argument
    fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: u32) -> i32 = cdecl 0x821BB5 => unported;
}

/// `GetYPosBasedOnHealth(1, GetYPosBasedOnHealth(focusPlayer, y, 12), 12)` stored f32 (the first result is the
/// second call's float argument, left in place on the stack).
fn darkel_y(y: f32) -> f32 {
    let pif = u32::from(g::<u8>(0xB7CD74).read());
    let r = X87::from_f64_exact(get_y_pos_based_on_health(pif as u8, y, 0xC)).to_f32();
    X87::from_f64_exact(get_y_pos_based_on_health(1, r, 0xC)).to_f32()
}

/// `RsGlobal.width - RsGlobal.width * 0.0015625 * 32` (80-bit) as the f32 x of the HUD texts.
fn darkel_text_x() -> f32 {
    let w = xi(g::<i32>(0xC17044).read());
    w.fsub(x(f32::from_bits(0x3ACC_CCCD)).fmul(w).fmul_m(32.0))
        .to_f32()
}

/// `CDarkel::DrawMessages()` (0x43CEC0), from the bytes. Nothing when byte 0xA43088 == 1 or frenzy `Status`
/// (0x96A704, u16) is not 1..4 (jump table 0x43D1D0); status 3 draws nothing (verified behaviour of the former
/// partial port). Status 2 (passed): with byte 0x969A4A set and less than 5000 ms since `TimeOfFrenzyStart`
/// (0x96A6E0), the big message `TheText.Get(0x859500)` for 3000 ms. Statuses 1 and 4: the start message
/// (0x96A6D0, when set) for 3000 ms while `now - start` is < 8000 ms (with 0x969A4A: 3000..11000 ms); then the
/// font state (proportional, no background, scale `(w * 0.0015625 * 0.5, h * 0.0022321)`, alignment 2, style 2,
/// wrap `w * 0.0015625 * 640`, edge 2, drop colour black, HUD colour 4), the two y positions (`h * 0.0022321 *
/// 148` and `* 148 + * 20`, each through `GetYPosBasedOnHealth(focus, ., 12)` then `(1, ., 12)`, both moved down
/// by `h * 0.0022321 * 72` when player 2 exists), the timer `"%d:%02d"` of `TimeLimit - now + start` (shown when
/// above 4000 ms or on odd frames; only with `TimeLimit >= 0`) and the kills left `"%d"` (negative shown as 0),
/// printed right-aligned at `w - w * 0.0015625 * 32`.
#[hook(addr = 0x43CEC0, conv = cdecl, name = "CDarkel::DrawMessages", expect = "A0 88 30 A4 00",
       route = "L2", hazards = "none",
       note = "completed partial port: statuses 1, 2, 4 are new (frenzy HUD); the 0xA43088 / status exits were verified")]
pub fn darkel_draw_messages() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    let status = g::<u16>(0x96A704).read();
    let now = g::<u32>(0xB7CB84).read();
    let start = g::<u32>(0x96A6E0).read();
    let flag = g::<u8>(0x969A4A).read() != 0;
    match status {
        2 => {
            if flag && now.wrapping_sub(start) < 0x1388 {
                add_big_message(text_get(g::<u8>(0xC1B340), g::<u8>(0x859500)), 0xBB8, 0);
            }
            return;
        }
        1 | 4 => {}
        _ => return,
    }
    let t = now.wrapping_sub(start);
    let show = if flag {
        (0xBB8..0x2AF8).contains(&t)
    } else {
        t < 0x1F40
    };
    let msg = g::<GPtr<u8>>(0x96A6D0).read();
    if show && !msg.is_null() {
        add_big_message(msg, 0xBB8, 0);
    }
    let w = || xi(g::<i32>(0xC17044).read());
    let h = || xi(g::<i32>(0xC17048).read()).fmul_m(f32::from_bits(0x3B12_4925));
    font_set_proportional(1);
    font_set_background(0, 0);
    font_set_scale(
        (w().fmul_m(f32::from_bits(0x3ACC_CCCD)).fmul_m(0.5).to_f32()).to_bits(),
        (h().to_f32()).to_bits(),
    );
    font_set_orientation(2);
    font_set_right_justify_wrap(f32::from_bits(0));
    font_set_font_style(2);
    font_set_wrapx(w().fmul_m(f32::from_bits(0x3ACC_CCCD)).fmul_m(640.0).to_f32());
    font_set_edge(2);
    let mut rgba = [0u32; 2];
    let c = GPtr::<u8>::cast::<u32>(rgba_ctor(GPtr::<u32>::cast::<u8>(GPtr::from_raw(rgba.as_mut_ptr())), 0, 0, 0, 0xFF));
    font_set_drop_color(c.read());
    let mut col = [0u32; 2];
    let c = GPtr::<u8>::cast::<u32>(hud_colours_get_rgb(g::<u8>(0xBAB22C), GPtr::<u32>::cast::<u8>(GPtr::from_raw(col.as_mut_ptr())), 4));
    font_set_color(c.read());
    let mut y0 = darkel_y(h().fmul_m(148.0).to_f32());
    let hh = h();
    let mut y1 = darkel_y(x(148.0).fmul(hh).fadd(hh.fmul_m(20.0)).to_f32());
    if g::<u32>(0xB7CF28).read() != 0 {
        let off = h().fmul_m(72.0);
        y0 = x(y0).fadd(off).to_f32();
        y1 = off.fadd_m(y1).to_f32();
    }
    let limit = g::<i32>(0x96A6E8).read();
    if limit >= 0 {
        let left = (limit as u32).wrapping_sub(now).wrapping_add(start);
        sprintf2(
            g::<u8>(0xB71670),
            g::<u8>(0x859510),
            left / 60000,
            (left % 60000) / 1000,
        );
        ascii_to_gxt_char(g::<u8>(0xB71670), g::<u8>(0xC1B100));
        if left > 0xFA0 || g::<u8>(0xB7CB4C).read() & 1 != 0 {
            font_print_string(darkel_text_x(), y0, g::<u8>(0xC1B100));
        }
    }
    let kills = g::<i32>(0x96A6EC).read().max(0);
    sprintf1(g::<u8>(0xB71670), g::<u8>(0x859508), kills as u32);
    ascii_to_gxt_char(g::<u8>(0xB71670), g::<u8>(0xC1B100));
    font_print_string(darkel_text_x(), y1, g::<u8>(0xC1B100));
}

/// `CDarkel::RegisterKillByPlayer(const CPed* ped, eWeaponType weapon, bool headShot, int32 playerId)` (0x43DCD0):
/// respect / kill stats by ped type 6..17 (jump tables 0x43DF10 / 0x43DF04: cop and dealer +1.5 respect, gang
/// members +0.75 and stat 0xEF, the player's gang (type 8) -5 respect, stat 0xF0 and, when the ped is in the
/// player's group, stat 0xA3 with its update message); during a frenzy (status 1 or 4) a kill with the frenzy
/// weapon (or the paired weapons 0x33, 0x34/0x1C, 0x31/0x32, 0x25/0x12, `CheckDamagedWeaponType` for 0x38/0x39)
/// of a ped that counts lowers `KillsNeeded` (unless 0x969A49 requires head shots) with frontend sound 0x2B,
/// and stat 0xCE (cops) / 0xCD; then the per-model kill counter (0x969A50, u16, index `playerId + model*2`),
/// stat 0x79, the per-type counters (0xB78E90, or 0xB78EE0 for +0x470 bit 10), head shot stat 0x82, stat 0xB0,
/// `AddKillToProvocation` for player 0 unless the player's +0x46C bit 8, and `PlayerKilledADealer` (tail jump)
/// for types >= 17.
#[hook(addr = 0x43DCD0, conv = cdecl, name = "CDarkel::RegisterKillByPlayer", expect = "57 8B 7C 24 08")]
pub fn register_kill_by_player(ped: GPtr<u8>, weapon: i32, head_shot: u8, player_id: i32) {
    let ty = rd::<u32>(ped, 0x598);
    match ty.wrapping_sub(6) {
        0 | 11 => increment_stat(0x44, 1.5),
        2 => {
            decrement_stat(0x44, 5.0f32.to_bits());
            increment_stat(0xF0, 1.0);
            let grp = rd::<i32>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0x38);
            let membership = g::<u8>((0xC09928 + grp * 0x2D4) as usize);
            if (membership_is_member(membership, ped) as u8) as u32 & 0xFF != 0 {
                increment_stat(0xA3, 1.0);
                display_script_stat_update_message(0, 0x150, 1.0);
            }
        }
        1 | 3..=10 => {
            increment_stat(0x44, 0.75);
            increment_stat(0xEF, 1.0);
        }
        _ => {}
    }
    let status = g::<u16>(0x96A704).read();
    if status == 1 || status == 4 {
        let fw = g::<i32>(0x96A700).read();
        let counts = ((fw == 0x38 || fw == 0x39) && check_damaged_weapon_type(weapon, fw) != 0)
            || weapon == fw
            || weapon == 0x33
            || (weapon == 0x34 && fw == 0x1C)
            || (weapon == 0x31 && fw == 0x32)
            || (weapon == 0x32 && fw == 0x31)
            || (weapon == 0x25 && fw == 0x12);
        if counts {
            if this_ped_should_be_killed_for_frenzy(ped) != 0
                && (g::<u8>(0x969A49).read() == 0 || head_shot != 0)
            {
                let k = g::<i32>(0x96A6EC);
                k.write(k.read().wrapping_sub(1));
                report_frontend_audio_event(g(0xB6BC90), 0x2B, 0.0, 1.0);
            }
            increment_stat(if rd::<u32>(ped, 0x598) == 6 { 0xCE } else { 0xCD }, 1.0);
        }
    }
    let model = i32::from(rd::<i16>(ped, 0x22));
    let kc = g::<u16>((0x969A50 + (player_id + model * 2) * 2) as usize);
    kc.write(kc.read().wrapping_add(1));
    increment_stat(0x79, 1.0);
    let c = if rd::<u32>(ped, 0x470) & 0x400 != 0 {
        g::<u32>(0xB78EE0)
    } else {
        g::<u32>((0xB78E90 + rd::<u32>(ped, 0x598) * 4) as usize)
    };
    c.write(c.read().wrapping_add(1));
    if head_shot != 0 {
        increment_stat(0x82, 1.0);
    }
    increment_stat(0xB0, 1.0);
    if player_id == 0 && rd::<u32>(g::<GPtr<u8>>(0xB7CD98).read(), 0x46C) & 0x100 == 0 {
        add_kill_to_provocation(rd::<i32>(ped, 0x598));
    }
    if rd::<i32>(ped, 0x598) >= 0x11 {
        player_killed_a_dealer();
    }
}

use sa_core::bindings::boat_ctor;
use crate::bindings::find_node_closest_to_coors;
use crate::bindings::set_up_driver_and_passengers_for_vehicle;
use sa_core::bindings::train_ctor;
use sa_core::bindings::general_random_int_in_range as random_int_in_range;
use sa_world::bindings::world_add;
use sa_stream::bindings::is_boat_model;
use sa_stream::bindings::pick_random_car;
use sa_stream::bindings::set_custom_car_plate_text;
use sa_core::bindings::automobile_ctor;
use sa_core::bindings::bike_ctor;
use sa_core::bindings::bmx_ctor;
use sa_world::bindings::get_distance_from_centre_of_mass_to_base_of_model;
use sa_stream::bindings::get_zone_info;
use sa_core::bindings::heli_ctor;
use sa_core::bindings::monster_truck_ctor;
use sa_core::bindings::plane_ctor;
use sa_core::bindings::quad_bike_ctor;
use sa_world::bindings::set_clump_alpha;
use sa_world::bindings::set_posn as placeable_set_posn;
use sa_core::bindings::trailer_ctor;
use sa_world::bindings::vehicle_new;
use sa_world::bindings::vehicle_set_created_by;
use sa_world::bindings::find_player_coors as find_player_coors_out;
use sa_stream::bindings::streaming_request_model as request_model;
use sa_world::bindings::find_ground_z_for_coord;
use sa_world::bindings::set_orientation as placeable_set_orientation;
use sa_world::bindings::world_process_vertical_line as process_vertical_line;
orig! {
    /// `CCarGenerator::CheckForBlockage(int32)` (0x6F32E0)
    fn check_for_blockage_call(this: GPtr<u8>, model: i32) -> u8 = thiscall 0x6F32E0 => check_for_blockage;
    /// `CSpecialPlateHandler::Find(int32 index, char* out)` (0x6F2D30, `ret 8`)
    fn special_plate_find(this: GPtr<u8>, index: u32, out: GPtr<u8>) -> i32 = thiscall 0x6F2D30 => unported;
    /// `CVehicle::SetCollisionLighting(tColLighting)` (0x6D0CA0, `ret 4`)
    fn vehicle_set_collision_lighting(v: GPtr<u8>, l: u32) = thiscall 0x6D0CA0 => unported;
    /// `CEntity` vtable slot 0x10 with one argument (0)
    fn entity_vslot_10(e: GPtr<u8>, a: u32) = thiscall vslot 0x10 => unported;
}

/// `CStreaming::ms_aInfoForModel[mi].m_nLoadState == LOADSTATE_LOADED` (0x8E4CC0 + 0x10, stride 0x14).
fn model_loaded(mi: i32) -> bool {
    g::<u8>((0x8E4CD0 + mi * 20) as usize).read() == 1
}

/// `CCarGenerator::DoInternalProcessing()` (0x6F34D0, thiscall, SEH frame), partial port (ADR-002 fallback):
/// the hotdog van generators (`m_nModelId` == 0x24C: the vendor ped of model 0xA8 is created through
/// `AddPed(4, 0xA8, pos)` whose fourth argument is the caller's saved EDI, which the port cannot reproduce) run the
/// whole original. PORTED: everything else. Parked car limit `NumParkedCars` (0x9690A0) 10 at night (hour > 21
/// or < 7), else 5, unless the generator ignores it (+0xD bit 4); a negative model is a random-car generator
/// (`-model` if loaded, else `m_LoadedAppropriateCars.PickRandomCar(true, true)` after `GetZoneInfo` of the
/// player position, rejecting model-info +0x4D == 5 and col-model lengths > 8.0, stored back as `-model` with
/// colours 0xFF); `CheckForBlockage` pushes the next try 4 ms later; models are requested (priority 8) and
/// must be loaded. Land vehicles: `ProcessVerticalLine` from 1.0 above the generator z (1000.0 when z <= -100)
/// down to -1000.0, special plate text, the constructor by vehicle type with created-by 3 (bikes / BMXs set
/// +0x614 bit 4; a failed allocation writes it through null as the original), the result of
/// `GetDistanceFromCentreOfMassToBaseOfModel` discarded, +0x4A8 &= 0xE7, +0x428 &= 0xBF and
/// `SetCollisionLighting` with the dword at colpoint +0x25. Boats / helis / planes (IsBoatModel or models 0x1CC,
/// 0x1BF, 0x1A1): constructor by type 3 / 4 / 5 (other types take a stale stack slot in the original: null
/// here, unreachable with the shipped models), `FindGroundZForCoord` above -100, +0x4A6 = 0xFF. Then vtable
/// slot 0x10 (0), +0x428 &= 0xEF, position (ground + base distance), heading `+0xA * 2pi/256`, status 4,
/// unlocked, +0x42A bit 1 = generator +0xD bit 3; by day tractors / combines (0x213 / 0x214) near a road node
/// (`FindNodeClosestToCoors` 20.0) become driving cars with a farmer (ped type 4); `CWorld::Add`; alarm and
/// door-lock chances (`GetRandomNumberInRange(0, 100)` x2); colours copied or saved back; clump alpha 0; the
/// vehicle handle; the generate count is never decremented (signed compare with -1, original bug); next
/// time = now + 4.
#[hook(addr = 0x6F34D0, conv = thiscall, name = "CCarGenerator::DoInternalProcessing", expect = "6A FF 68 14 8A 84 00", this = "CCarGenerator", fallback)]
pub fn do_internal_processing(this: GPtr<u8>) {
    if rd::<i16>(this, 0) == 0x24C {
        return do_internal_processing_original(this);
    }
    let hours = g::<u8>(0xB70153).read();
    let night = !(7..=0x15).contains(&hours);
    if rd::<u8>(this, 0xD) & 0x10 == 0 && g::<i32>(0x9690A0).read() >= if night { 10 } else { 5 } {
        return;
    }
    let next_try = |this: GPtr<u8>| wr::<u32>(this, 0x14, rd::<u32>(this, 0x14).wrapping_add(4));
    let gm = rd::<i16>(this, 0);
    let model: i32;
    if gm < 0 {
        if gm != -1 && model_loaded(-i32::from(gm)) {
            model = -i32::from(gm);
        } else {
            let mut pc = [0f32; 3];
            let pcp = GPtr::from_raw(pc.as_mut_ptr());
            find_player_coors_out(pcp, -1);
            get_zone_info(pcp, GPtr::<GPtr<u8>>::cast::<u8>(GPtr::NULL));
            model = pick_random_car(GPtr::<u8>::cast::<sa_stream::loaded_car_group::CLoadedCarGroup>(g(0xC0E9F8)), 1, 1);
            if model < 0 {
                return;
            }
            let mi = model_info(model);
            if rd::<u8>(mi, 0x4D) == 5 {
                return;
            }
            let col = rd::<GPtr<u8>>(mi, 0x14);
            if !col.is_null() && !jne_mask(x(rd::<f32>(col, 0x10)).fsub_m(rd::<f32>(col, 4)), x(8.0), 0x41) {
                return;
            }
            wr::<i16>(this, 0, -(model as i16));
            wr::<u8>(this, 2, 0xFF);
            wr::<u8>(this, 3, 0xFF);
        }
        if check_for_blockage_call(this, model) != 0 {
            next_try(this);
            return;
        }
    } else {
        if check_for_blockage_call(this, i32::from(gm)) != 0 {
            next_try(this);
            return;
        }
        request_model(i32::from(rd::<i16>(this, 0)), 8);
        model = i32::from(rd::<i16>(this, 0));
        if model == 0x24C {
            request_model(0xA8, 8);
        }
    }
    if !model_loaded(model) {
        return;
    }
    // (the 0x24C / 0xA8 load test only matters for the hotdog generators, which took the fallback)
    let mi = model_info(model);
    let veh: GPtr<u8>;
    let mut ground: f32;
    if is_boat_model(model) || model == 0x1CC || model == 0x1BF || model == 0x1A1 {
        let v = match rd::<u32>(mi, 0x3C) {
            3 => (0xA18, heli_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
            4 => (0xA04, plane_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
            5 => (0x7E8, boat_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
            _ => (0, heli_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
        };
        veh = if v.0 == 0 {
            GPtr::NULL
        } else {
            let m = vehicle_new(v.0);
            if m.is_null() { m } else { (v.1)(m, model, 3) }
        };
        let z = xi(i32::from(rd::<i16>(this, 8))).fmul_m(0.125);
        ground = z.to_f32();
        if !z.fcom(x(-100.0)).test_parity(0x41) {
            let gy = xi(i32::from(rd::<i16>(this, 6))).fmul_m(0.125).to_f32();
            let gx = xi(i32::from(rd::<i16>(this, 4))).fmul_m(0.125).to_f32();
            ground = X87::from_f64_exact(find_ground_z_for_coord(gx, gy)).to_f32();
        }
        wr::<u16>(veh, 0x4A6, 0xFF);
    } else {
        let z = xi(i32::from(rd::<i16>(this, 8))).fmul_m(0.125);
        let px = xi(i32::from(rd::<i16>(this, 4))).fmul_m(0.125).to_f32();
        let py = xi(i32::from(rd::<i16>(this, 6))).fmul_m(0.125).to_f32();
        let top = if z.fcom(x(-100.0)).test_parity(0x41) {
            z.fadd_m(1.0).to_f32()
        } else {
            1000.0
        };
        let mut pos = [px, py, top];
        let mut cp = [0u32; 12];
        let mut ent = [0u32; 1];
        let cpp = GPtr::<u8>::from_raw(cp.as_mut_ptr().cast());
        if process_vertical_line(
            GPtr::<f32>::cast::<u8>(GPtr::from_raw(pos.as_mut_ptr())),
            -1000.0,
            cpp,
            GPtr::<u32>::cast::<u8>(GPtr::from_raw(ent.as_mut_ptr())),
            1,
            0,
            0,
            0,
            0,
            0,
            GPtr::NULL,
        ) == 0
        {
            return;
        }
        ground = rd::<f32>(cpp, 8);
        let mut plate = [0u8; 16];
        let pp = GPtr::<u8>::from_raw(plate.as_mut_ptr());
        let index = (this.addr() as u32).wrapping_sub(0xC27AD0) >> 5;
        special_plate_find(g(0xC279D8), index, pp);
        if plate[0] != 0 && rd::<u32>(model_info(model), 0x24) != 0 {
            set_custom_car_plate_text(model_info(model), pp);
        }
        type Ctor = fn(GPtr<u8>, i32, u32) -> GPtr<u8>;
        let (size, ctor, bike): (u32, Option<Ctor>, bool) = match rd::<u32>(model_info(model), 0x3C) {
            1 => (0x99C, Some(monster_truck_ctor), false),
            2 => (0x9BC, Some(quad_bike_ctor), false),
            3 => (0xA18, Some(heli_ctor), false),
            4 => (0xA04, Some(plane_ctor), false),
            6 => (0x6AC, Some(|t, m, c| train_ctor(t, m as u32, c)), false),
            9 => (0x814, Some(bike_ctor), true),
            10 => (0x838, Some(bmx_ctor), true),
            11 => (0x9F4, Some(trailer_ctor), false),
            _ => (0x988, None, false),
        };
        let m = vehicle_new(size);
        veh = if m.is_null() {
            if bike {
                let p = GPtr::<u8>::from_addr(0x614);
                p.write(p.read() | 0x10);
            }
            m
        } else {
            match ctor {
                Some(c) => {
                    let v = c(m, model, 3);
                    if bike {
                        wr::<u8>(v, 0x614, rd::<u8>(v, 0x614) | 0x10);
                    }
                    v
                }
                None => automobile_ctor(m, model as u32, 3, 1),
            }
        };
        get_distance_from_centre_of_mass_to_base_of_model(veh);
        wr::<u8>(veh, 0x4A8, rd::<u8>(veh, 0x4A8) & 0xE7);
        wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0xBF);
        vehicle_set_collision_lighting(veh, rd::<u32>(cpp, 0x25));
    }
    // 0x6F3ADA
    entity_vslot_10(veh, 0);
    wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0xEF);
    let z = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(veh))
        .fadd_m(ground)
        .to_f32();
    let py = xi(i32::from(rd::<i16>(this, 6))).fmul_m(0.125).to_f32();
    let px = xi(i32::from(rd::<i16>(this, 4))).fmul_m(0.125).to_f32();
    placeable_set_posn(veh, px, py, z);
    let heading = xi(i32::from(rd::<u8>(this, 0xA)))
        .fmul_m(f32::from_bits(0x3CC9_0FDB))
        .to_f32();
    placeable_set_orientation(veh, 0.0, 0.0, heading);
    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x20);
    wr::<u32>(veh, 0x4F8, 1);
    let old = rd::<u8>(veh, 0x42A);
    wr::<u8>(veh, 0x42A, old ^ (((rd::<u8>(this, 0xD) >> 2) ^ old) & 2));
    let mut driver = -1i32;
    let vm = rd::<u16>(veh, 0x22);
    if !night && (vm == 0x213 || vm == 0x214) {
        let p = entity_pos(veh);
        let mut node = [0u32; 1];
        let np = GPtr::from_raw(node.as_mut_ptr());
        find_node_closest_to_coors(
            g(0x96F050),
            np,
            p.read(),
            p.wrapping_index(1).read(),
            p.wrapping_index(2).read(),
            0,
            20.0,
            0,
            0,
            0,
            0,
            1,
        );
        let a = node[0];
        if a & 0xFFFF != 0xFFFF {
            let nodes = g::<GPtr<u8>>((0x96F854 + (a & 0xFFFF) * 4) as usize).read();
            let n = nodes.byte_add(((a >> 16) * 0x1C) as isize);
            if rd::<u8>(n, 0x18) & 0xF != 0 {
                let links = g::<GPtr<u32>>((0x96FA94 + (a & 0xFFFF) * 4) as usize).read();
                let l0 = links.wrapping_index(i32::from(rd::<i16>(n, 0x10))).read();
                if !g::<GPtr<u8>>((0x96F854 + (l0 & 0xFFFF) * 4) as usize)
                    .read()
                    .is_null()
                {
                    vehicle_set_created_by(veh, 1);
                    wr::<u32>(veh, 0x390, node[0]);
                    wr::<u8>(veh, 0x3D0, 7);
                    wr::<u8>(veh, 0x3BA, 1);
                    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x18);
                    wr::<u32>(veh, 0x394, l0);
                    wr::<u8>(veh, 0x42C, rd::<u8>(veh, 0x42C) | 0x40);
                    wr::<u16>(veh, 0x4A6, 0xFF);
                    wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 1);
                    driver = 4;
                }
            }
        }
    }
    world_add(veh);
    if driver != -1 {
        set_up_driver_and_passengers_for_vehicle(veh, driver, 0, 0, 0, 0x63);
    }
    if random_int_in_range(0, 100) < i32::from(rd::<u8>(this, 0xB)) {
        wr::<u16>(veh, 0x45C, 0xFFFF);
    }
    if random_int_in_range(0, 100) < i32::from(rd::<u8>(this, 0xC)) {
        wr::<u32>(veh, 0x4F8, 2);
    }
    let c1 = rd::<u8>(this, 2);
    if c1 != 0xFF && rd::<u8>(this, 3) != 0xFF {
        wr::<u8>(veh, 0x434, c1);
        wr::<u8>(veh, 0x435, rd::<u8>(this, 3));
    } else if rd::<i16>(this, 0) < -1 {
        wr::<u8>(this, 2, rd::<u8>(veh, 0x434));
        wr::<u8>(this, 3, rd::<u8>(veh, 0x435));
    }
    set_clump_alpha(rd::<GPtr<u8>>(veh, 0x18), 0);
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let index = ((veh.addr() as i32).wrapping_sub(rd::<i32>(pool, 0))) / 0xA18;
    let flag = rd::<GPtr<u8>>(pool, 4).wrapping_index(index).read();
    wr::<u16>(this, 0x18, ((index << 8) as u16).wrapping_add(u16::from(flag)));
    // `movzx ecx, m_nGenerateCount; cmp ecx, -1; jge`: never decremented
    wr::<u32>(this, 0x14, g::<u32>(0xB7CB84).read().wrapping_add(4));
}

/// `CGangWars::StrengthenPlayerInfluenceInZone(int32 amount)` (stub 0x445F50 -> 0x1565090): on the zone being
/// fought over (`*(CZoneInfo**)0x96AB8C`), the Grove strength byte (+1) grows by `amount` up to 0x37 (only when
/// below 0x37); when Grove was not already strongest (non-zero and >= +0 + +2) but now is, stat 0xEA += 1.0. The
/// float is pushed through an obfuscated pair: `[0x15604FC] - [0x665271]` = 0x1950B2E9 - 0xD9D0B2E9 = 0x3F800000
/// (decoded from the static image).
#[hook(addr = 0x445F50, conv = cdecl, name = "CGangWars::StrengthenPlayerInfluenceInZone", expect = "E9 3B F1 11 01")]
pub fn strengthen_player_influence_in_zone(amount: i32) {
    let zi = g::<GPtr<u8>>(0x96AB8C).read();
    let strongest = |zi: GPtr<u8>| {
        let v = i32::from(rd::<u8>(zi, 1));
        v != 0 && v >= i32::from(rd::<u8>(zi, 2)) + i32::from(rd::<u8>(zi, 0))
    };
    let was = strongest(zi);
    let cur = rd::<u8>(zi, 1);
    let zi = if cur < 0x37 {
        let v = (i32::from(cur) + amount).min(0x37);
        wr::<u8>(zi, 1, v as u8);
        g::<GPtr<u8>>(0x96AB8C).read()
    } else {
        zi
    };
    if !was && strongest(zi) {
        increment_stat(0xEA, f32::from_bits(0x3F80_0000));
    }
}

/// `CGangWars::AttackWaveOvercome()` (0x445B30, AL): walks the ped pool downwards; peds of the attack wave
/// (+0x474 bit 12) that are dead or dying (+0x530 == 0x37 / 0x36) leave the wave (bit cleared,
/// `SetCharCreatedBy(1)`); the others count, and those within 45.0 (2D) of the player count as near. Overcome
/// when at most one is left and none is near (also when the pool is empty). The pool pointer is read through an
/// obfuscated pointer: `rol([0x15606E8], [0x4B5588] & 31)` = `rol(0x48005BA2, 9)` = 0xB74490 (CPools::ms_pPedPool,
/// decoded from the static image; the key byte 0x4B5588 is code).
#[hook(addr = 0x445B30, conv = cdecl, name = "CGangWars::AttackWaveOvercome", expect = "83 EC 14 53 90")]
pub fn attack_wave_overcome() -> u8 {
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let n = rd::<i32>(pool, 8);
    if n == 0 {
        return 1;
    }
    let (mut count, mut near) = (0i32, 0i32);
    for i in (0..n).rev() {
        if rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() & 0x80 != 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() {
            continue;
        }
        let flags = rd::<u32>(ped, 0x474);
        if flags & 0x1000 == 0 {
            continue;
        }
        let state = rd::<u32>(ped, 0x530);
        if state == 0x37 || state == 0x36 {
            wr::<u32>(ped, 0x474, flags & 0xFFFF_EFFF);
            ped_set_char_created_by(ped, 1);
            continue;
        }
        count += 1;
        let p = entity_pos(ped);
        let mut pc = [0f32; 3];
        let c = find_player_coors_out(GPtr::from_raw(pc.as_mut_ptr()), -1);
        let dx = x(p.read()).fsub_m(c.read());
        let dy = x(p.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read());
        let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
        if !jp5(d, x(45.0)) {
            near += 1;
        }
    }
    u8::from(count <= 1 && near == 0)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GANG_WARS_UPDATE,
    HOOK_GENERATE_ROAD_BLOCKS,
    HOOK_CHECK_FOR_BLOCKAGE,
    HOOK_DARKEL_DRAW_MESSAGES,
    HOOK_REGISTER_KILL_BY_PLAYER,
    HOOK_DO_INTERNAL_PROCESSING,
    HOOK_STRENGTHEN_PLAYER_INFLUENCE_IN_ZONE,
    HOOK_ATTACK_WAVE_OVERCOME,
];
