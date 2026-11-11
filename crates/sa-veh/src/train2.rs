//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CTrain` and `CTrainNode`. Ported from the 1.0 US
//! bytes; gta-reversed and docs/re/specs/VEH as reading aids.
//!
//! `CTrainNode` (0xA): +0/+2/+4 position x/y/z (i16, units of 1/8 m), +6 distance along the track (u16,
//! units of 1/3 m), +8 lighting byte, +9 "lighting valid" bool.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, jp5, rd, wr, x, xi};

/// `CTrain*` / `CTrainNode*`.
type Ptr = GPtr<u8>;

sa_core::globals! {
    list GLOBALS;
    /// `arrTotalTrackLength[4]`
    pub static TOTAL_TRACK_LENGTH @ 0xC37FEC: [f32; 4] = [0.0; 4];
    /// `NumTrackNodes[4]`
    pub static NUM_TRACK_NODES @ 0xC38014: [i32; 4] = [0; 4];
    /// `pTrackNodes[4]` (`CTrainNode*`)
    pub static TRACK_NODES @ 0xC38024: [GPtr<u8>; 4] = [GPtr::NULL; 4];
    /// `StationDist[6]`
    pub static STATION_DIST @ 0xC38034: [f32; 6] = [0.0; 6];
}

orig! {
    /// `CAEVehicleAudioEntity::Terminate()` (0x4FB8C0)
    pub fn audio_terminate(a: GPtr<u8>) = thiscall 0x4FB8C0 => unported;
    /// `CVehicle::~CVehicle` body (0x6E2B40)
    pub fn vehicle_destructor(v: GPtr<u8>) = thiscall 0x6E2B40 => unported;
    /// `CVehicle::operator delete(void*)` (0x6E2D90)
    pub fn vehicle_delete(p: GPtr<u8>) = cdecl 0x6E2D90 => crate::vehicle2::operator_delete;
    /// `CVehicle::PreRender()` (0x6D6480)
    pub fn vehicle_pre_render(v: GPtr<u8>) = thiscall 0x6D6480 => unported;
    /// `CVehicle::DoVehicleLights(CMatrix&, uint32)` (0x6E1A60)
    pub fn do_vehicle_lights(v: GPtr<u8>, m: GPtr<u8>, flags: u32) = thiscall 0x6E1A60 => unported;
    /// `CEntity` virtual slot 0x10 (`SetIsStatic(bool)`)
    pub fn entity_set_is_static(e: GPtr<u8>, v: u32) = thiscall vslot 0x10 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    pub fn add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x7)` (0x5674E0)
    pub fn world_process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, l: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CTrain::ReadAndInterpretTrackFile(const char*, CTrainNode**, int32*, float*, int32)` (0x6F55D0)
    pub fn read_and_interpret_track_file(name: u32, nodes: GPtr<u8>, num: GPtr<u8>, len: GPtr<u8>, track: i32) = cdecl 0x6F55D0 => unported;
    /// `CVehicle::CVehicle(eVehicleCreatedBy)` (0x6D5F10)
    pub fn vehicle_ctor(v: GPtr<u8>, created_by: u32) -> GPtr<u8> = thiscall 0x6D5F10 => unported;
    /// `CVehicle::SetModelIndex(uint32)` (0x6D6A40)
    pub fn vehicle_set_model_index(v: GPtr<u8>, mi: u32) = thiscall 0x6D6A40 => unported;
    /// `CClumpModelInfo::FillFrameArray(RpClump*, RwFrame**)` (0x4C5440)
    pub fn fill_frame_array(clump: GPtr<u8>, frames: GPtr<u8>) = cdecl 0x4C5440 => unported;
    /// `CVehicleModelInfo::ChooseVehicleColour(uint8&, uint8&, uint8&, uint8&, int32)` (0x4C8500)
    pub fn choose_vehicle_colour(mi: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, inc: i32) = thiscall 0x4C8500 => unported;
    /// `CAEVehicleAudioEntity::Initialise(CEntity*)` (0x4F7670)
    pub fn audio_initialise(a: GPtr<u8>, e: GPtr<u8>) = thiscall 0x4F7670 => unported;
    /// CRT `floor(double)` (0x8219F0)
    pub fn floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    pub fn clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CTrain::TrainHitStuff(CPtrListSingleLink<CPhysical*>&, CEntity*)` (0x6F5CF0)
    pub fn train_hit_stuff_o(list: GPtr<GPtr<u8>>, train: GPtr<u8>) = cdecl 0x6F5CF0 => crate::train2::train_hit_stuff;
    /// `CTrain::FindClosestTrackNode(CVector, int32*)` (0x6F6BD0)
    pub fn find_closest_track_node_o(x: f32, y: f32, z: f32, track: GPtr<i32>) -> i32 = cdecl 0x6F6BD0 => crate::train2::find_closest_track_node;
}

/// `0.125`: track node coordinates to metres.
fn node_scale() -> f32 {
    g::<f32>(0x858C48).read()
}

/// `CTrainNode::CTrainNode()` (0x6F5370): clears the "lighting valid" byte; returns `this`.
#[hook(addr = 0x6F5370, conv = thiscall, name = "CTrainNode", expect = "8B C1 C6 40 09 00", this = "CTrainNode")]
pub fn train_node_ctor(n: Ptr) -> Ptr {
    wr::<u8>(n, 9, 0);
    n
}

/// `CTrain::~CTrain()` (0x6F54D0): vtable 0x872370, `CAEVehicleAudioEntity::Terminate(+0x138)`, then
/// `CVehicle::~CVehicle`. EAX is the base destructor's (void here).
#[hook(addr = 0x6F54D0, conv = thiscall, name = "CTrain::~CTrain", expect = "6A FF 68 88 8A 84 00", this = "CTrain")]
pub fn train_dtor(t: Ptr) {
    wr::<u32>(t, 0, 0x0087_2370);
    audio_terminate(t.byte_add(0x138));
    vehicle_destructor(t);
}

orig! {
    /// `CTrain::~CTrain()` (0x6F54D0)
    pub fn train_dtor_o(t: GPtr<u8>) = thiscall 0x6F54D0 => crate::train2::train_dtor;
}

/// `CTrain::scalar deleting destructor(uint8)` (0x6F6300): `~CTrain`, `operator delete` when bit 0 of
/// the flags is set; returns `this`.
#[hook(addr = 0x6F6300, conv = thiscall, name = "CTrain::`scalar deleting destructor'", expect = "56 8B F1 E8 C8 F1 FF FF", this = "CTrain")]
pub fn train_scalar_deleting_dtor(t: Ptr, flags: u32) -> Ptr {
    train_dtor_o(t);
    if flags & 1 != 0 {
        vehicle_delete(t);
    }
    t
}

/// `CTrain::PreRender()` (0x6F5570): `CVehicle::PreRender`, then `DoVehicleLights(matrix, f)` with f =
/// 5 (model 0x1C1) or 0x25 when train flag bit 3 is set, else 0x15 when bit 4 is set.
#[hook(addr = 0x6F5570, conv = thiscall, name = "CTrain::PreRender", expect = "56 8B F1 E8 08 0F FE FF", this = "CTrain")]
pub fn pre_render(t: Ptr) {
    vehicle_pre_render(t);
    let f = rd::<u8>(t, 0x5B8);
    let m = rd::<GPtr<u8>>(t, 0x14);
    if f & 8 != 0 {
        let fl = if rd::<u16>(t, 0x22) == 0x1C1 { 5 } else { 0x25 };
        do_vehicle_lights(t, m, fl);
    } else if f & 0x10 != 0 {
        do_vehicle_lights(t, m, 0x15);
    }
}

/// `CTrain::TrainHitStuff(CPtrListSingleLink<CPhysical*>&, CEntity*)` (0x6F5CF0, the `or 0x800` in the
/// .HOODLUM fragment 0x406E85): every entity of the list except `train` gets bit 11 of `+0x40`; objects
/// (type 4) with flag bit 2 or bit 18 of `+0x1C` whose model is one of the three words at 0x8CD6C8 /
/// 0x8CD6CC / 0x8CD6D0 are made non-static (vtable +0x10 with false), added to the moving list, and get
/// `+0xB8 = 0`.
#[hook(addr = 0x6F5CF0, conv = cdecl, name = "CTrain::TrainHitStuff<CPtrListSingleLink<CPhysical*>>", expect = "8B 44 24 04 57")]
pub fn train_hit_stuff(list: GPtr<GPtr<u8>>, train: Ptr) {
    let mut node = list.read();
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if e == train {
            continue;
        }
        wr::<u32>(e, 0x40, rd::<u32>(e, 0x40) | 0x800);
        if rd::<u8>(e, 0x36) & 7 != 4 {
            continue;
        }
        let fl = rd::<u32>(e, 0x1C);
        if fl & 4 == 0 && fl & 0x40000 == 0 {
            continue;
        }
        let mi = i32::from(rd::<i16>(e, 0x22));
        if mi != i32::from(g::<u16>(0x8CD6C8).read())
            && mi != i32::from(g::<u16>(0x8CD6CC).read())
            && mi != i32::from(g::<u16>(0x8CD6D0).read())
        {
            continue;
        }
        entity_set_is_static(e, 0);
        add_to_moving_list(e);
        wr::<u8>(e, 0xB8, 0);
    }
}

/// `CTrainNode::GetLightingFromCollision()` (0x6F5F80): the cached `+8` when `+9` is set; otherwise
/// `CWorld::ProcessVerticalLine` from (x/8, y/8, z/8 + 1.0) down 1000 m against buildings: on a hit the
/// colpoint's `+0x25` lighting byte is cached (and `+9` set) and returned, else 0x48. Only AL is defined.
#[hook(addr = 0x6F5F80, conv = thiscall, name = "CTrainNode::GetLightingFromCollision", expect = "83 EC 3C 56 8B F1", this = "CTrainNode")]
pub fn get_lighting_from_collision(n: Ptr) -> u8 {
    if rd::<u8>(n, 9) != 0 {
        return rd::<u8>(n, 8);
    }
    let k = node_scale();
    let mut origin = [0f32; 3];
    origin[0] = xi(i32::from(rd::<i16>(n, 0))).fmul_m(k).to_f32();
    origin[1] = xi(i32::from(rd::<i16>(n, 2))).fmul_m(k).to_f32();
    origin[2] = xi(i32::from(rd::<i16>(n, 4)))
        .fmul_m(k)
        .fadd_m(g::<f32>(0x858624).read())
        .to_f32();
    let mut cp = [0u8; 0x2C];
    let mut ent = 0u32;
    let hit = world_process_vertical_line(
        GPtr::from_raw(&raw mut origin).cast(),
        0xC47A_0000,
        GPtr::from_raw(&raw mut cp).cast(),
        GPtr::from_raw(&raw mut ent).cast(),
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    if hit == 0 {
        return 0x48;
    }
    let l = cp[0x25];
    wr::<u8>(n, 8, l);
    wr::<u8>(n, 9, 1);
    l
}

/// `CTrain::FindClosestTrackNode(CVector pos, int32* track)` (0x6F6BD0): over the four tracks, the node
/// with the smallest 3D distance (`sqrt((dz² + dy²) + dx²)`, dx stored as f32, dy at register
/// precision, the node z stored as f32) below 99999.9 (strict, the minimum is stored as f32); writes its
/// track to `*track` and returns its index. Deviation: when no node is closer than 99999.9 the original
/// returns an uninitialised stack slot; this returns 0.
#[hook(addr = 0x6F6BD0, conv = cdecl, name = "CTrain::FindClosestTrackNode", expect = "83 EC 20 53 8B 5C 24 34")]
pub fn find_closest_track_node(px: f32, py: f32, pz: f32, track: GPtr<i32>) -> i32 {
    let k = node_scale();
    let mut best_d = f32::from_bits(0x47C3_4FF3);
    let mut best = 0i32;
    let nums = NUM_TRACK_NODES.ptr().cast::<i32>();
    let nodes = TRACK_NODES.ptr().cast::<GPtr<u8>>();
    for t in 0..4i32 {
        let mut j = 0i32;
        while j < nums.wrapping_index(t).read() {
            let nd = nodes.wrapping_index(t).read().byte_add(j as isize * 0xA);
            let nx = xi(i32::from(rd::<i16>(nd, 0))).fmul_m(k);
            let ny = xi(i32::from(rd::<i16>(nd, 2))).fmul_m(k);
            let nz = xi(i32::from(rd::<i16>(nd, 4))).fmul_m(k).to_f32();
            let dx = x(px).fsub(nx).to_f32();
            let dy = x(py).fsub(ny);
            let dz = x(pz).fsub_m(nz);
            let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
            if !jp5(d, x(best_d)) {
                best_d = d.to_f32();
                best = j;
                track.write(t);
            }
            j += 1;
        }
    }
    best
}

/// `CTrain::InitTrains()` (0x6F7440): clears `bDisableRandomTrains` and the dword 0xC3800C; reads the
/// four track files whose node pointer is still null (names 0x8724DC / 0x8724C4 / 0x8724AC / 0x872494);
/// then for the six station positions (0x8D48F8, 12 bytes each) `StationDist[i]` = distance byte of the
/// closest node, taken from track 0's node array whatever track it was found on, times 1/3 (0x859040).
#[hook(addr = 0x6F7440, conv = cdecl, name = "CTrain::InitTrains", expect = "51 A1 24 80 C3 00")]
pub fn init_trains() {
    crate::train::DISABLE_RANDOM_TRAINS.set(0);
    g::<u32>(0xC3800C).write(0);
    let nodes = TRACK_NODES.ptr().cast::<GPtr<u8>>();
    let nums = NUM_TRACK_NODES.ptr().cast::<i32>();
    let lens = TOTAL_TRACK_LENGTH.ptr().cast::<f32>();
    for (t, name) in [0x0087_24DCu32, 0x0087_24C4, 0x0087_24AC, 0x0087_2494]
        .into_iter()
        .enumerate()
    {
        let t = t as i32;
        if nodes.wrapping_index(t).read().is_null() {
            read_and_interpret_track_file(
                name,
                nodes.wrapping_index(t).cast(),
                nums.wrapping_index(t).cast(),
                lens.wrapping_index(t).cast(),
                t,
            );
        }
    }
    let n0 = nodes.wrapping_index(0).read();
    let dist = STATION_DIST.ptr().cast::<f32>();
    for i in 0..6i32 {
        let p = GPtr::<u8>::from_addr(0x8D48F8 + 12 * i as usize);
        let mut tr = 0i32;
        let node = find_closest_track_node_o(
            rd::<f32>(p, 0),
            rd::<f32>(p, 4),
            rd::<f32>(p, 8),
            GPtr::from_raw(&raw mut tr),
        );
        let d = rd::<u16>(n0, node as isize * 0xA + 6);
        dist.wrapping_index(i)
            .write(xi(i32::from(d)).fmul_m(g::<f32>(0x859040).read()).to_f32());
    }
}

/// `_ftol(floor(v))` as the original computes a sector index (CRT floor of the register value).
fn floor_ftol(v: sa_math::x87::X87) -> i32 {
    crate::common::ftol32(sa_math::x87::X87::from_f64_exact(floor(v.to_f64())))
}

/// `CTrain::MarkSurroundingEntitiesForCollisionWithTrain(CVector pos, float radius, CEntity* train,
/// bool onlyFirstList)` (0x6F6640): the sector rectangle of `pos ± radius` (`v * [0x858B38] +
/// [0x858B34]`; the min x from the register `x - r`, the others from f32 stores; each index is
/// `_ftol(floor(...))`, computed again from the stored f32 when in range: min clamped to 0, max to
/// 0x77); a new scan code (`ClearScanCodes` and 1 on wrap); then for every sector (y outer, x inner,
/// `&0xF` repeat sectors at 0xB992B8, 12 bytes) `TrainHitStuff` on list 0, and unless the bool is set
/// also on lists 1 and 2.
#[hook(addr = 0x6F6640, conv = cdecl, name = "CTrain::MarkSurroundingEntitiesForCollisionWithTrain", expect = "83 EC 0C D9 44 24 10")]
pub fn mark_surrounding_entities_for_collision_with_train(
    px: f32,
    py: f32,
    _pz: f32,
    radius: f32,
    train: Ptr,
    only_first: u32,
) {
    let m = g::<f32>(0x858B38).read();
    let a = g::<f32>(0x858B34).read();
    let xmin = x(px).fsub_m(radius);
    let xmax = x(px).fadd_m(radius).to_f32();
    let ymin = x(py).fsub_m(radius).to_f32();
    let ymax = x(py).fadd_m(radius).to_f32();
    let lo = |v: sa_math::x87::X87| {
        let s = v.fmul_m(m).fadd_m(a);
        if floor_ftol(s) > 0 {
            floor_ftol(x(s.to_f32()))
        } else {
            0
        }
    };
    let hi = |v: f32| {
        let s = x(v).fmul_m(m).fadd_m(a);
        if floor_ftol(s) < 0x77 {
            floor_ftol(x(s.to_f32()))
        } else {
            0x77
        }
    };
    let min_x = lo(xmin);
    let min_y = lo(x(ymin));
    let max_x = hi(xmax);
    let max_y = hi(ymax);
    let sc = g::<u16>(0xB7CD78);
    if sc.read() < 0xFFFF {
        sc.write(sc.read() + 1);
    } else {
        clear_scan_codes();
        sc.write(1);
    }
    let mut yy = min_y;
    while yy <= max_y {
        let mut xx = min_x;
        while xx <= max_x {
            let idx = ((xx & 0xF) + ((yy & 0xF) << 4)) as usize;
            let rs = 0xB992B8 + idx * 12;
            train_hit_stuff_o(GPtr::from_addr(rs), train);
            if only_first as u8 == 0 {
                train_hit_stuff_o(GPtr::from_addr(rs + 4), train);
                train_hit_stuff_o(GPtr::from_addr(rs + 8), train);
            }
            xx += 1;
        }
        yy += 1;
    }
}

/// `CTrain::CTrain(int32 modelIndex, eVehicleCreatedBy)` (0x6F6030): `CVehicle(createdBy)`, vtable
/// 0x872370, `+0x5D8..+0x668` zeroed, type/subtype 6, handling `0xC2B9DC + 0xE0 * mi->+0x4A`
/// (`+0x38C` = its flags `+0xD0`), `SetModelIndex`, the 17 frames `+0x668` filled from the clump, the
/// two doors (model 0x23A: open 1.25 / 0.25, else -1.2566 / 1.2566, axis 2, dirn 0x14), train flags,
/// two `rand()` nibbles of `+0x5CB` (`& 3`, `(& 3) + 1 << 4`), 5 passengers, colours
/// (`ChooseVehicleColour(..., 1)`), mass / turn mass / centre of mass / elasticity 0.05 / buoyancy from
/// the handling (`+0x98 = drag / [0x872364] * [0x858B8C]` when drag `>` [0x858C58], else drag), entity
/// and physical flags, audio `Initialise`. Returns `this`.
#[hook(addr = 0x6F6030, conv = thiscall, name = "CTrain::Constructor", expect = "6A FF 68 C8 8A 84 00", this = "CTrain")]
pub fn train_ctor(t: Ptr, mi: u32, created_by: u32) -> Ptr {
    vehicle_ctor(t, created_by);
    wr::<u32>(t, 0, 0x0087_2370);
    for k in 0..0x24 {
        wr::<u32>(t, 0x5D8 + 4 * k, 0);
    }
    let mip = g::<GPtr<u8>>(0xA9B0C8 + 4 * mi as usize).read();
    wr::<u32>(t, 0x594, 6);
    wr::<u32>(t, 0x590, 6);
    let h = GPtr::<u8>::from_addr(0xC2B9DC + 0xE0 * usize::from(rd::<u8>(mip, 0x4A)));
    wr::<GPtr<u8>>(t, 0x384, h);
    wr::<u32>(t, 0x38C, rd::<u32>(h, 0xD0));
    vehicle_set_model_index(t, mi);
    for k in 0..0x11 {
        wr::<u32>(t, 0x668 + 4 * k, 0);
    }
    fill_frame_array(rd::<GPtr<u8>>(t, 0x18), t.byte_add(0x668));
    wr::<u16>(t, 0x610, 0x14);
    wr::<u8>(t, 0x612, 2);
    if rd::<u16>(t, 0x22) == 0x23A {
        wr::<u32>(t, 0x608, 0x3FA0_0000);
        wr::<u32>(t, 0x60C, 0x3E80_0000);
        wr::<u32>(t, 0x620, 0x3FA0_0000);
        wr::<u32>(t, 0x624, 0x3E80_0000);
    } else {
        wr::<u32>(t, 0x608, 0xBFA0_D97C);
        wr::<u32>(t, 0x60C, 0);
        wr::<u32>(t, 0x620, 0x3FA0_D97C);
        wr::<u32>(t, 0x624, 0);
    }
    wr::<u8>(t, 0x62A, 2);
    wr::<u16>(t, 0x628, 0x14);
    wr::<u8>(t, 0x5B8, (rd::<u8>(t, 0x5B8) & 0xD9) | 0x81);
    wr::<u8>(t, 0x5B9, (rd::<u8>(t, 0x5B9) & 0xFA) | 2);
    wr::<u8>(t, 0x5CA, 0);
    let r1 = crate::common::rand_at(0x6F6174) as u8;
    wr::<u8>(t, 0x5CB, (r1 & 3) | (rd::<u8>(t, 0x5CB) & 0xF0));
    let r2 = crate::common::rand_at(0x6F618C) as u8;
    let hi = (((r2 & 3).wrapping_add(1)) << 4) & 0xF0;
    let f40 = rd::<u32>(t, 0x40) | 0x2_0000;
    let f1c = rd::<u32>(t, 0x1C) | 1;
    wr::<u8>(t, 0x5CB, hi | (rd::<u8>(t, 0x5CB) & 0xF));
    wr::<u32>(t, 0x5CC, 0);
    wr::<u8>(t, 0x488, 5);
    wr::<u32>(t, 0x40, f40);
    wr::<u32>(t, 0x1C, f1c);
    wr::<u32>(t, 0x5C4, g::<u32>(0xB7CB84).read());
    wr::<u16>(t, 0x5C8, 0);
    wr::<u8>(t, 0x5C0, 0);
    wr::<u32>(t, 0x5A8, 0);
    wr::<u32>(t, 0x5A4, 0);
    wr::<u32>(t, 0x5BC, 0);
    choose_vehicle_colour(
        mip,
        t.byte_add(0x434),
        t.byte_add(0x435),
        t.byte_add(0x436),
        t.byte_add(0x437),
        1,
    );
    let h = rd::<GPtr<u8>>(t, 0x384);
    wr::<u32>(t, 0x8C, rd::<u32>(h, 4));
    wr::<u32>(t, 0x90, rd::<u32>(h, 0xC));
    for k in 0..3 {
        wr::<u32>(t, 0xA4 + 4 * k, rd::<u32>(h, 0x14 + 4 * k));
    }
    let h = rd::<GPtr<u8>>(t, 0x384);
    wr::<u32>(t, 0x9C, 0x3D4C_CCCD);
    wr::<u32>(t, 0xA0, rd::<u32>(h, 0x24));
    if !crate::common::jne_mask(x(rd::<f32>(h, 0x10)), x(g::<f32>(0x858C58).read()), 0x41) {
        let v = x(rd::<f32>(h, 0x10))
            .fdiv_m(g::<f32>(0x872364).read())
            .fmul_m(g::<f32>(0x858B8C).read());
        wr::<f32>(t, 0x98, v.to_f32());
    } else {
        wr::<u32>(t, 0x98, rd::<u32>(h, 0x10));
    }
    wr::<u32>(t, 0x40, (rd::<u32>(t, 0x40) & 0xFFFF_FFF7) | 4);
    let b36 = (rd::<u8>(t, 0x36) & 7) | 0x30;
    wr::<u32>(t, 0x1C, rd::<u32>(t, 0x1C) | 0x8000_0000);
    wr::<u32>(t, 0x5D0, 0);
    wr::<u32>(t, 0x5D4, 0);
    wr::<u8>(t, 0x36, b36);
    wr::<u32>(t, 0x3C8, 0);
    wr::<u8>(t, 0x3D0, 0);
    audio_initialise(t.byte_add(0x138), t);
    t
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TRAIN_CTOR,
    HOOK_MARK_SURROUNDING_ENTITIES_FOR_COLLISION_WITH_TRAIN,
    HOOK_TRAIN_NODE_CTOR,
    HOOK_TRAIN_DTOR,
    HOOK_TRAIN_SCALAR_DELETING_DTOR,
    HOOK_PRE_RENDER,
    HOOK_TRAIN_HIT_STUFF,
    HOOK_GET_LIGHTING_FROM_COLLISION,
    HOOK_FIND_CLOSEST_TRACK_NODE,
    HOOK_INIT_TRAINS,
];
