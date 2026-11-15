//! `CRoadBlocks::CreateRoadBlockBetween2Points` (0x4619C0), FRONTIER-2 turn 4 lane A (batch `frontier2-a7`). Ported
//! from the 1.0 US bytes (gta-reversed does not reverse it). The body is split into helpers that follow the
//! original's blocks: the line set-up (0x4619EF..0x461B72), the vehicle model (0x461B72..0x461C2D), the slot count
//! (0x461C2D..0x461D4D), one slot of the line (0x461D72..0x4624C6; a car, or for a gang block sometimes a burning
//! object) and the cones of a police block (0x4624EE..0x4629A9).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::fpu::{fbits, fcom_c0, fld_fst, ge_or_nan, le_or_nan, st0};
use sa_math::x87::X87;

use crate::bindings::join_car_with_road_system;
use crate::common::{find_player_coors, ftol32, g, model_info, rand_at, rand_frac, rd, wr, x, xi};
use sa_core::bindings::{
    automobile_ctor, matrix_assign, matrix_rotate_y, matrix_rotate_z, matrix_set_translate, matrix_set_translate_only,
    matrix_set_unity, object_ctor_5a1d70, object_new, vector_normalise,
};
use sa_stream::bindings::{are_army_required, are_fbi_required, are_swat_required, get_default_cop_car_model};
use sa_world::bindings::{
    automobile_place_on_road_properly, find_ground_z_for_3d_coord, find_player_wanted, matrix_dtor, multiply3x3_vm,
    object_delete_all_temp_objects_in_area, set_clump_alpha, start_fire_on, uses_siren, vehicle_new, world_add,
};
use crate::bindings::get_car_node_index_from_door;
use sa_world::world::common::entity_pos;
use sa_world::world::mvpb_world::entity_update_rw_matrix;

orig! {
    /// `CRoadBlocks::ClearSpaceForRoadBlockObject(CVector, CVector)` (0x461020, cdecl, both vectors by value, AL)
    fn clear_space(ax: u32, ay: u32, az: u32, bx: u32, by: u32, bz: u32) -> u8 = cdecl 0x461020 => crate::misc5::clear_space_for_road_block_object;
    /// `CPopulation::PickRiotRoadBlockCar()` (0x6144B0)
    fn pick_riot_road_block_car() -> i32 = cdecl 0x6144B0 => crate::population2::pick_riot_road_block_car;
    /// `CAutomobile::SetTotalDamage(bool)` (0x6A27F0, `ret 4`)
    fn automobile_set_total_damage(v: GPtr<u8>, random: u32) = thiscall 0x6A27F0 => unported;
    /// entity virtual +0x0 (scalar deleting destructor)
    fn v_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// entity virtual +0x10 (`SetIsStatic(bool)`)
    fn v_set_is_static(e: GPtr<u8>, s: u32) = thiscall vslot 0x10 => unported;
    /// vehicle virtual +0x6C (`OpenDoor(CPed*, int32 node, eDoors, float ratio, bool sound)`)
    fn v_open_door(v: GPtr<u8>, ped: u32, node: i32, door: u32, ratio: f32, sound: u32) = thiscall vslot 0x6C => unported;
    /// vehicle virtual +0xA8 (`BlowUpCarCutSceneNoExtras(bool, bool, bool, bool)`)
    fn v_blow_up_cut_scene(v: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall vslot 0xA8 => unported;
    /// vehicle virtual +0xD4 (`GetHeightAboveRoad()`, ST0)
    fn v_get_height_above_road(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
}

type V3 = [f32; 3];

/// `gFireManager` (0xB71F80)
const FIRE_MANAGER: usize = 0xB71F80;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7CB84;
/// `CObject::nNoTempObjects` (0xBB4A70, int16)
const NO_TEMP_OBJECTS: usize = 0xBB4A70;
/// `ModelIndices::MI_ROADBLOCKFUCKEDCAR1`-style burning object of a gang block (0x8CD580, int16)
const MI_GANG_BLOCK_OBJECT: usize = 0x8CD580;
/// The cone model of a police block (0x8CD57C, int16)
const MI_CONE: usize = 0x8CD57C;
/// `M_PI` as the original pushes it (0x40490FDB)
const PI: f32 = fbits(0x40490FDB);

/// A stack `CMatrix` local of the original: its offsets (right +0, forward +0x10, up +0x20, pos +0x30).
#[derive(Clone, Copy)]
struct Mat(GPtr<u8>);

impl Mat {
    fn set(self, off: isize, v: [u32; 3]) {
        for (i, w) in v.iter().enumerate() {
            wr::<u32>(self.0, off + 4 * i as isize, *w);
        }
    }
    fn f(self, off: isize) -> f32 {
        rd::<f32>(self.0, off)
    }
    fn setf(self, off: isize, v: f32) {
        wr::<f32>(self.0, off, v);
    }
}

fn bits(v: V3) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

/// `fld; fchs; fstp`
fn neg(v: f32) -> u32 {
    x(v).fchs().to_f32().to_bits()
}

const ONE: u32 = 0x3F80_0000;

/// `CRoadBlocks::CreateRoadBlockBetween2Points(CVector a, CVector b, uint32 gang)` (0x4619C0, cdecl; SEH frame for one
/// stack `CMatrix` local: its inlined constructor clears `m_pAttachMatrix` / `m_bOwnsAttachedMatrix`, every exit runs
/// its destructor 0x59ACD0). A line of vehicles across `a`..`b` (see [`line_slots`]), then for a police block (gang
/// 0) the cones of [`cones`]. The flag is tested as the whole dword.
#[hook(addr = 0x4619C0, conv = cdecl, name = "CRoadBlocks::CreateRoadBlockBetween2Points", expect = "6A FF E9 9E 5A FA FF",
       unverified = "frontier2-a7", route = "split", hazards = "heap-writes,uninit-locals", recs = "police-gang",
       note = "the first slot's RotateZ reads the matrix local's uninitialised position (overwritten right after)")]
pub fn create_road_block_between_2_points(ax: u32, ay: u32, az: u32, bx: u32, by: u32, bz: u32, gang: u32) {
    let mut mat = [0u32; 0x12];
    let m = Mat(GPtr::from_raw(mat.as_mut_ptr()).cast());
    let a = [f32::from_bits(ax), f32::from_bits(ay), f32::from_bits(az)];
    let b = [f32::from_bits(bx), f32::from_bits(by), f32::from_bits(bz)];
    body(m, &a, &b, gang);
    matrix_dtor(m.0);
}

/// The line set-up and its geometry, shared by the slots and the cones.
struct Line {
    a: V3,
    /// `(b - a)` normalised (+0x2C)
    dir: V3,
    /// `|b - a|` (+0xBC)
    len: f32,
    /// `(a + b) * 0.5` (+0xD4)
    mid: V3,
    /// `(dir.y, -dir.x, 0)` normalised, flipped to the player's side (+0x5C)
    perp: V3,
}

fn body(m: Mat, a: &V3, b: &V3, gang: u32) {
    let line = line_setup(a, b);
    let Some(model) = block_model(gang) else {
        return;
    };
    let col = rd::<GPtr<u8>>(model_info(model), 0x14);
    let min = [rd::<f32>(col, 0), rd::<f32>(col, 4), rd::<f32>(col, 8)];
    let max = [rd::<f32>(col, 0xC), rd::<f32>(col, 0x10), rd::<f32>(col, 0x14)];
    let width = x(max[0]).fsub_m(min[0]).fadd_m(2.0).to_f32();
    let mut length = x(max[1]).fsub_m(min[1]).fadd_m(fbits(0x3E4CCCCD)).to_f32();
    if gang != 0 {
        length = x(length).fadd_m(0.5).to_f32();
    }
    line_slots(m, &line, gang, model, &min, &max, width, length);
    if gang == 0 {
        cones(m, &line);
    }
}

/// 0x4619EF..0x461B72: direction, length, midpoint and the perpendicular facing the player (`dot <= 0`, ordered:
/// multiplied by -1).
fn line_setup(a: &V3, b: &V3) -> Line {
    let dz = x(b[2]).fsub_m(a[2]);
    let dy = x(b[1]).fsub_m(a[1]);
    let dx = x(b[0]).fsub_m(a[0]).to_f32();
    let mut dir = [dx, dy.to_f32(), dz.to_f32()];
    let len = x(dx).fmul_m(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt().to_f32();
    let sz = x(a[2]).fadd_m(b[2]);
    let sy = x(a[1]).fadd_m(b[1]);
    let sx = x(a[0]).fadd_m(b[0]).to_f32();
    let cz = sz.fmul_m(0.5).to_f32();
    let cy = sy.fmul_m(0.5).to_f32();
    let cx = x(sx).fmul_m(0.5).to_f32();
    vector_normalise(GPtr::from_raw(&mut dir));
    let mut perp = [dir[1], f32::from_bits(neg(dir[0])), 0.0];
    vector_normalise(GPtr::from_raw(&mut perp));
    let mut t = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(t.as_mut_ptr()), -1);
    let ez = x(p.wrapping_index(2).read()).fsub_m(cz);
    let ey = x(p.wrapping_index(1).read()).fsub_m(cy);
    let ex = x(p.read()).fsub_m(cx);
    let dot = ez.fmul_m(perp[2]).fadd(ex.fmul_m(perp[0])).fadd(ey.fmul_m(perp[1]));
    if !ge_or_nan(dot, x(0.0)) {
        for v in &mut perp {
            *v = x(*v).fmul_m(-1.0).to_f32();
        }
    }
    Line { a: *a, dir, len, mid: [cx, cy, cz], perp }
}

/// 0x461B72..0x461C2D: a gang block's riot car (`PickRiotRoadBlockCar`, -1: none); else barracks (army), FBI rancher,
/// enforcer (SWAT) or the default cop car, the default cop car when that model is not loaded (load state != 1), and
/// none for the cop bike (523).
fn block_model(gang: u32) -> Option<i32> {
    if gang != 0 {
        let m = pick_riot_road_block_car();
        return if m == -1 { None } else { Some(m) };
    }
    let mut m = if are_army_required(find_player_wanted(-1)) != 0 {
        0x1B1
    } else if are_fbi_required(find_player_wanted(-1)) != 0 {
        0x1EA
    } else if are_swat_required(find_player_wanted(-1)) != 0 {
        0x1AB
    } else {
        get_default_cop_car_model(0)
    };
    if g::<u8>(0x8E4CD0 + m as usize * 0x14).read() != 1 {
        m = get_default_cop_car_model(0);
    }
    if m == 0x20B {
        None
    } else {
        Some(m)
    }
}

/// 0x461C2D..0x4624DB: up to five slots fit in the length; each is sideways (`rand() & 1`, only for a police block
/// without the barracks) taking the model's width + 2, or lengthways taking its length + 0.2 (+ 0.5 for a gang),
/// while the rest is not below it (`test ah, 1`). The gap is the rest over `count + 1`; the slots follow along the
/// line from `a`.
#[allow(clippy::too_many_arguments)]
fn line_slots(m: Mat, line: &Line, gang: u32, model: i32, min: &V3, max: &V3, width: f32, length: f32) {
    let mut side = [0u8; 5];
    let mut rest = line.len;
    let mut count = 0i32;
    for s in side.iter_mut() {
        *s = if model != 0x1B1 && gang == 0 { (rand_at(0x461CD6) & 1) as u8 } else { 0 };
        let step = if *s != 0 { width } else { length };
        if fcom_c0(x(rest), x(step)) {
            break;
        }
        rest = x(rest).fsub_m(step).to_f32();
        count += 1;
        if count == 5 {
            break;
        }
    }
    let gap = x(rest).fdiv(xi(count + 1));
    if count <= 0 {
        return;
    }
    let mut along = gap.to_f32();
    let half = x(length).fmul_m(0.5);
    let half_len = half.to_f32();
    let after = half.fadd(gap).to_f32();
    for j in 0..count {
        let s = Slot { side: side[j as usize] != 0, count, half_len, after };
        slot(m, line, gang, model, min, max, width, &s, &mut along);
    }
}

/// One slot's fixed inputs.
struct Slot {
    side: bool,
    count: i32,
    half_len: f32,
    /// `half_len + gap` (+0x14): added after the slot's position
    after: f32,
}

/// One slot (0x461D72..0x4624C6): the matrix (sideways: forward = perp; else forward = dir, a gang block tilting it
/// on its side one time in four, and `rand() & 1` turning it round by pi), a small random yaw, the position on the
/// line, the space cleared ([`clear_box`]); then for a gang block a burning object (tilted, or `rand() & 255 < 64`)
/// or else the car of [`slot_car`].
#[allow(clippy::too_many_arguments)]
fn slot(m: Mat, line: &Line, gang: u32, model: i32, min: &V3, max: &V3, width: f32, s: &Slot, along: &mut f32) {
    let mut tilted = false;
    let angle: X87 = if s.side {
        *along = x(width).fmul_m(0.5).fadd_m(*along).to_f32();
        m.set(0x10, bits(line.perp));
        m.set(0, [line.perp[1].to_bits(), neg(line.perp[0]), 0]);
        m.set(0x20, [0, 0, ONE]);
        let r = rand_at(0x461DF8) & 0xFF;
        let k = if gang != 0 { fbits(0x3B83126F) } else { fbits(0x3B03126F) };
        xi(r).fsub_m(128.0).fmul_m(k)
    } else {
        *along = x(s.half_len).fadd_m(*along).to_f32();
        if gang != 0 {
            tilted = rand_at(0x461E48) & 0xFF < 0x40;
        }
        m.set(0x10, bits(line.dir));
        if tilted {
            m.set(0, [0, 0, ONE]);
            m.set(0x20, [neg(line.dir[1]), line.dir[0].to_bits(), 0]);
        } else {
            m.set(0, [line.dir[1].to_bits(), neg(line.dir[0]), 0]);
            m.set(0x20, [0, 0, ONE]);
        }
        if rand_at(0x461EF1) & 1 != 0 {
            if tilted {
                matrix_rotate_y(m.0, PI);
            } else {
                matrix_rotate_z(m.0, PI);
            }
        }
        let r = rand_at(0x461F13) & 0xFF;
        let k = if gang != 0 { fbits(0x3BC49BA6) } else { fbits(0x3B449BA6) };
        xi(r).fsub_m(128.0).fmul_m(k)
    };
    matrix_rotate_z(m.0, angle.to_f32());
    let (a, d) = (&line.a, &line.dir);
    let pz = x(d[2]).fmul_m(*along);
    let py = x(*along).fmul_m(d[1]);
    let px = x(*along).fmul_m(d[0]).to_f32();
    let tz = x(a[2]).fadd(pz).to_f32();
    let ty = x(a[1]).fadd(py).to_f32();
    let tx = x(px).fadd_m(a[0]).to_f32();
    matrix_set_translate_only(m.0, tx, ty, tz);
    let low = if tilted { min[0] } else { min[2] };
    m.setf(0x38, x(m.f(0x38)).fsub_m(low).fadd_m(fbits(0x3E99999A)).to_f32());
    *along = x(s.after).fadd_m(*along).to_f32();
    if !clear_box(m, min, max) {
        return;
    }
    if gang != 0 && (tilted || rand_at(0x462102) & 0xFF < 0x40) {
        let o = new_object(i32::from(g::<u16>(MI_GANG_BLOCK_OBJECT).read()), m);
        v_set_is_static(o, 0);
        add_temp_object(o);
        set_on_fire(o, 0x164);
        return;
    }
    slot_car(m, gang, model, s);
}

/// The car of a slot (0x4621EF..0x4624C6): a random-vehicle `CAutomobile` of the model, abandoned, lifted to its
/// height above road (- 0.6), placed, joined to the road system; a gang car is wrecked (`_ftol(frac * 4) == 0`:
/// `SetTotalDamage(true)`) or blown up and set on fire, a police car may run its siren. A car whose up.z is not above
/// 0.94 is deleted; else it is added invisible (alpha 0), with road block peds (count > 3: one row, else two) on the
/// side facing the player (sideways: doors 2 and 3 opened).
fn slot_car(m: Mat, gang: u32, model: i32, s: &Slot) {
    let mut v = vehicle_new(0x988);
    if !v.is_null() {
        v = automobile_ctor(v, model as u32, 1, 1);
    }
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x20);
    let h = st0(v_get_height_above_road(v));
    m.setf(0x38, h.fsub_m(fbits(0x3F19999A)).fadd_m(m.f(0x38)).to_f32());
    matrix_assign(rd::<GPtr<u8>>(v, 0x14), m.0);
    automobile_place_on_road_properly(v);
    v_set_is_static(v, 0);
    entity_update_rw_matrix(v);
    wr::<u32>(v, 0x4F8, 1);
    join_car_with_road_system(v);
    let f42c = rd::<u8>(v, 0x42C) | 0x40;
    let f428 = rd::<u8>(v, 0x428) & 0xE7;
    for o in [0x3BA, 0x3BB, 0x3B7, 0x3B8] {
        wr::<u8>(v, o, 0);
    }
    wr::<u32>(v, 0x3C8, 0);
    wr::<u8>(v, 0x3D0, 0);
    wr::<u8>(v, 0x42C, f42c);
    wr::<u8>(v, 0x428, f428);
    let mut burn = false;
    if gang != 0 {
        let r = rand_at(0x4622F9);
        if ftol32(rand_frac(r).fmul_m(4.0)) == 0 {
            automobile_set_total_damage(v, 1);
        } else {
            v_blow_up_cut_scene(v, 1, 1, 1, 1);
            wr::<u32>(v, 0x4D8, rd::<u32>(v, 0x4D8).wrapping_add(1_000_000));
            burn = true;
        }
    } else if uses_siren(v) != 0 && rand_at(0x462352) & 1 != 0 {
        wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) | 0x80);
    }
    let up_z = rd::<f32>(rd::<GPtr<u8>>(v, 0x14), 0x28);
    if le_or_nan(x(up_z), x(fbits(0x3F70A3D7))) {
        v_delete(v, 1);
        return;
    }
    set_clump_alpha(rd::<GPtr<u8>>(v, 0x18), 0);
    world_add(v);
    wr::<u8>(v, 0x42A, rd::<u8>(v, 0x42A) | 0x10);
    wr::<u32>(v, 0x4D0, g::<u32>(TIME_MS).read().wrapping_add(7000));
    wr::<u8>(v, 0x4AE, if s.count > 3 { 1 } else { 2 });
    if s.side {
        wr::<u8>(v, 0x4AD, 2);
        open_door(v, 2);
        if rd::<u8>(v, 0x4AE) > 1 {
            open_door(v, 3);
        }
    } else {
        let pp = entity_pos(v);
        let mut t = [0f32; 3];
        let p = find_player_coors(GPtr::from_raw(t.as_mut_ptr()), -1);
        let ez = x(rd::<f32>(pp, 8)).fsub_m(p.wrapping_index(2).read());
        let ey = x(rd::<f32>(pp, 4)).fsub_m(p.wrapping_index(1).read());
        let ex = x(rd::<f32>(pp, 0)).fsub_m(p.read());
        let r = rd::<GPtr<u8>>(v, 0x14);
        let dot = ey.fmul_m(rd::<f32>(r, 4)).fadd(ez.fmul_m(rd::<f32>(r, 8))).fadd(ex.fmul_m(rd::<f32>(r, 0)));
        wr::<u8>(v, 0x4AD, u8::from(ge_or_nan(dot, x(0.0))));
    }
    if burn {
        set_on_fire(v, 0x490);
    }
}

/// `GetCarNodeIndexFromDoor(door)`; when the car has that frame, the door opened fully (with sound).
fn open_door(v: GPtr<u8>, door: u32) {
    let node = get_car_node_index_from_door(door);
    if rd::<u32>(v, 0x648 + node as isize * 4) != 0 {
        v_open_door(v, 0, node, door, 1.0, 1);
    }
}

/// `gFireManager.StartFire(e, NULL, 2.8, 1, 60000, 2)`; the entity's fire (`fire_off`) gets +0x21 = 0x5C.
fn set_on_fire(e: GPtr<u8>, fire_off: isize) {
    start_fire_on(g::<u8>(FIRE_MANAGER), e, GPtr::NULL, fbits(0x40333333), 1, 60000, 2);
    let f = rd::<GPtr<u8>>(e, fire_off);
    if !f.is_null() {
        wr::<u8>(f, 0x21, 0x5C);
    }
}

/// `new CObject(model, true)` (pool `new` 0x5A1EE0, size 0x17C; null stays null), its matrix assigned from the local
/// and its position written as the inlined `SetPosn` does (x by integer move, y / z through the x87).
fn new_object(model: i32, m: Mat) -> GPtr<u8> {
    let mut o = object_new(0x17C);
    if !o.is_null() {
        o = object_ctor_5a1d70(o, model, 1);
    }
    matrix_assign(rd::<GPtr<u8>>(o, 0x14), m.0);
    let (z, y, px) = (fld_fst(m.f(0x38)), fld_fst(m.f(0x34)), rd::<u32>(m.0, 0x30));
    let p = entity_pos(o);
    wr::<u32>(p, 0, px);
    wr::<f32>(p, 4, y);
    wr::<f32>(p, 8, z);
    o
}

/// `++nNoTempObjects`, the object temporary (+0x13C = 3) for 600 s, `CWorld::Add`.
fn add_temp_object(o: GPtr<u8>) {
    let n = g::<u16>(NO_TEMP_OBJECTS);
    n.write(n.read().wrapping_add(1));
    wr::<u8>(o, 0x13C, 3);
    wr::<u32>(o, 0x150, g::<u32>(TIME_MS).read().wrapping_add(600_000));
    world_add(o);
}

/// The model's box through the matrix (`FUN_0059C810` of min and max, plus the matrix position) cleared by
/// `ClearSpaceForRoadBlockObject`.
fn clear_box(m: Mat, min: &V3, max: &V3) -> bool {
    let corner = |v: &V3| -> [u32; 3] {
        let mut out = [0f32; 3];
        let mut v = *v;
        let r = multiply3x3_vm(GPtr::from_raw(out.as_mut_ptr()).cast(), GPtr::from_raw(v.as_mut_ptr()).cast(), m.0);
        let o = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
        [
            x(o[0]).fadd_m(m.f(0x30)).to_f32().to_bits(),
            x(o[1]).fadd_m(m.f(0x34)).to_f32().to_bits(),
            x(o[2]).fadd_m(m.f(0x38)).to_f32().to_bits(),
        ]
    };
    let c1 = corner(min);
    let c2 = corner(max);
    clear_space(c1[0], c1[1], c1[2], c2[0], c2[1], c2[2]) != 0
}

/// The cones of a police block (0x4624EE..0x4629A9): `n = min(_ftol(len / (cone width + 0.5)), 8)` cones, the temp
/// objects within `len / 2` of the midpoint deleted first; each cone at `a + dir * along + perp * 5`, jittered by
/// `(rand() & 15) * 0.05` in x and y, dropped onto the ground (`FindGroundZFor3DCoord` from 2 above; none found: no
/// cone), its space cleared, then added as a temporary object.
fn cones(m: Mat, line: &Line) {
    let model = i32::from(g::<u16>(MI_CONE).read());
    let col = rd::<GPtr<u8>>(model_info(model), 0x14);
    let min = [rd::<f32>(col, 0), rd::<f32>(col, 4), rd::<f32>(col, 8)];
    let max = [rd::<f32>(col, 0xC), rd::<f32>(col, 0x10), rd::<f32>(col, 0x14)];
    let w = x(max[0]).fsub_m(min[0]).fadd_m(0.5).to_f32();
    let mut n = ftol32(x(line.len).fdiv_m(w));
    if n > 8 {
        n = 8;
    }
    let gap = x(line.len).fsub(xi(n).fmul_m(w)).fdiv(xi(n + 1)).to_f32();
    let radius = x(line.len).fmul_m(0.5).to_f32();
    object_delete_all_temp_objects_in_area(line.mid[0], line.mid[1], line.mid[2], radius);
    for k in 0..n {
        cone(m, line, model, &min, &max, w, gap, k);
    }
}

/// One cone of [`cones`] (0x4625E0..0x46299B).
#[allow(clippy::too_many_arguments)]
fn cone(m: Mat, line: &Line, model: i32, min: &V3, max: &V3, w: f32, gap: f32, k: i32) {
    let (a, d, p) = (&line.a, &line.dir, &line.perp);
    let along = xi(k).fadd_m(0.5).fmul_m(w).fadd(xi(k + 1).fmul_m(gap)).to_f32();
    matrix_set_unity(m.0);
    matrix_set_translate(m.0, 0.0, 0.0, 0.0);
    m.set(0x10, [d[1].to_bits(), neg(d[0]), 0]);
    m.set(0, bits(*d));
    m.set(0x20, [0, 0, ONE]);
    let r = rand_at(0x46267D) & 0xFF;
    matrix_rotate_z(m.0, xi(r).fsub_m(128.0).fmul_m(fbits(0x3B449BA6)).to_f32());
    let oz = x(p[2]).fmul_m(5.0);
    let oy = x(p[1]).fmul_m(5.0);
    let ox = x(p[0]).fmul_m(5.0).to_f32();
    let dz = x(d[2]).fmul_m(along);
    let dy = x(along).fmul_m(d[1]);
    let dx = x(along).fmul_m(d[0]).to_f32();
    let tz = x(a[2]).fadd(dz).to_f32();
    let ty = x(a[1]).fadd(dy);
    let tx = x(dx).fadd_m(a[0]).to_f32();
    let tz = x(tz).fadd(oz).to_f32();
    let ty = ty.fadd(oy).to_f32();
    let tx = x(tx).fadd_m(ox).to_f32();
    matrix_set_translate_only(m.0, tx, ty, tz);
    let r = rand_at(0x462767) & 0xF;
    m.setf(0x30, xi(r).fmul_m(fbits(0x3D4CCCCD)).fadd_m(m.f(0x30)).to_f32());
    let r = rand_at(0x46278B) & 0xF;
    m.setf(0x34, xi(r).fmul_m(fbits(0x3D4CCCCD)).fadd_m(m.f(0x34)).to_f32());
    let z = x(m.f(0x38)).fadd_m(2.0).to_f32();
    let mut found = 0u8;
    let gz = find_ground_z_for_3d_coord(m.f(0x30), m.f(0x34), z, GPtr::from_raw(&mut found), GPtr::NULL);
    m.setf(0x38, st0(gz).to_f32());
    if found == 0 {
        return;
    }
    m.setf(0x38, x(m.f(0x38)).fsub_m(min[2]).to_f32());
    if !clear_box(m, min, max) {
        return;
    }
    let o = new_object(model, m);
    add_temp_object(o);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CREATE_ROAD_BLOCK_BETWEEN_2_POINTS];
