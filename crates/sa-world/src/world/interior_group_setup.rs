//! WRITER-WORLD-2 (batch `world6-2`, SA:MP-executed): `InteriorGroup_c::SetupShopPeds` and
//! `InteriorGroup_c::SetupPaths`, which builds the
//! ped path graph of a group of interiors (furniture-free walk nodes, the door nodes of every open
//! side, their links to the exterior path network and the pairing of unconnected doors). Ported from
//! the 1.0 US bytes; gta-reversed has it unreversed.
//!
//! Byte-level details kept: the four sides are not identical (side A, info +5, skips a link index
//! when it is negative while B / C / D skip only -1; only side A passes the group's "+0xE" flag to
//! its outer node; side D does not mark the first door; side A sums the squared distance to the
//! exterior node as `(dz² + dy²) + dx²`, the others as `(dx² + dz²) + dy²`); the exterior node
//! position stored at +0x3F4 is `int16 * 0.125` through f32; a door pair is linked when the best
//! squared distance, kept as f32 after each improvement but compared unrounded, is below 3 (start
//! 1e9). The original's door arrays hold 12 (positions) / 16 (node ids) entries in a 0x250-byte
//! frame; more doors overflow its stack (not modelled: 32 entries here).

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, x, xi};

use crate::bindings::add_ped;
use crate::bindings::get_tile_centre;
use crate::bindings::set_char_created_by;
use crate::bindings::set_ped_decision_maker_type;
use crate::bindings::tm_set_task as task_manager_set_task;
use sa_stream::bindings::load_all_requested_models;
use sa_core::bindings::task_new;
use sa_stream::bindings::stream_peds_for_interior;
orig! {
    /// `InteriorGroup_c::ArePathsLoaded()` (0x595380)
    fn are_paths_loaded(this: GPtr<u8>) -> u8 = thiscall 0x595380 => unported;
    /// `CPathFind::StartNewInterior(int32)` (0x44DE80) on `ThePaths` (0x96F050)
    fn start_new_interior(pf: GPtr<u8>, id: i32) = thiscall 0x44DE80 => unported;
    /// `CPathFind::AddNodeToNewInterior(float x, float y, float z, bool, int8 x6)` (0x450E90): the node
    /// address through the hidden pointer
    fn add_node_to_new_interior(pf: GPtr<u8>, out: GPtr<u8>, px: f32, py: f32, pz: f32, b: u32, l0: i32, l1: i32, l2: i32, l3: i32, l4: i32, l5: i32) -> GPtr<u8> = thiscall 0x450E90 => unported;
    /// `CPathFind::AddInteriorLink(int32, int32)` (0x44DED0)
    fn add_interior_link(pf: GPtr<u8>, a: i32, b: i32) = thiscall 0x44DED0 => unported;
    /// `CPathFind::AddInteriorLinkToExternalNode(int32, CNodeAddress)` (0x44DF30)
    fn add_interior_link_to_external_node(pf: GPtr<u8>, node: i32, ext: u32) = thiscall 0x44DF30 => unported;
    /// `CPathFind::FindNearestExteriorNodeToInteriorNode(int32)` (0x450F30): the address through the hidden pointer
    fn find_nearest_exterior_node(pf: GPtr<u8>, out: GPtr<u8>, node: i32) -> GPtr<u8> = thiscall 0x450F30 => unported;
    /// 0x44DF60 (a `.HOODLUM` thunk, catalogued as `CPathFind::RemoveInteriorLinks`), one node id
    fn path_44df60(pf: GPtr<u8>, node: i32) = thiscall 0x44DF60 => unported;
    /// `CPathFind::CompleteNewInterior(CNodeAddress*)` (0x452270)
    fn complete_new_interior(pf: GPtr<u8>, out: u32) = thiscall 0x452270 => unported;
    /// `CStreaming::FindMIPedSlotForInterior(int32)` (0x407FB0)
    fn find_mi_ped_slot_for_interior(rand_val: i32) -> i32 = cdecl 0x407FB0 => unported;
    /// `InteriorGroup_c::GetRandomInterior()` (0x5948C0)
    fn get_random_interior(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5948C0 => unported;
    /// `Interior_c::GetRandomTile(int32 status, int32* x, int32* y)` (0x591B20)
    fn get_random_tile(it: GPtr<u8>, status: i32, tx: GPtr<u8>, ty: GPtr<u8>) -> u8 = thiscall 0x591B20 => unported;
    /// `CTaskInteriorShopKeeper::CTaskInteriorShopKeeper(InteriorGroup_c*, bool)` (0x675660)
    fn shop_keeper_ctor(t: GPtr<u8>, group: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x675660 => unported;
    /// `CTaskInteriorBeInShop::CTaskInteriorBeInShop(InteriorGroup_c*)` (0x6753E0)
    fn be_in_shop_ctor(t: GPtr<u8>, group: GPtr<u8>) -> GPtr<u8> = thiscall 0x6753E0 => unported;
}

/// `1 / 32768` (0x858B14), `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8).
const INV_32768: f32 = f32::from_bits(0x3800_0000);
const MODEL_INFOS: usize = 0xA9_B0C8;

/// `ftol((rand() & 0xFFFF) * (1/32768) * k)`.
fn rand_scaled(site: u32, k: f32) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(INV_32768).fmul_m(k))
}

/// `ThePaths` (0x96F050), `ThePaths.m_pPathNodes[area]` (0x96F854).
const THE_PATHS: usize = 0x96_F050;
const PATH_NODES: usize = 0x96_F854;
/// `0.125` (0x858C48), `9.0` (0x85EED4), `3.0` (0x858B3C), `1e9` (0x858FEC).
const F_0_125: f32 = 0.125;
const F_9: f32 = 9.0;
const F_3: f32 = 3.0;
const F_1E9: f32 = f32::from_bits(0x4E6E_6B28);

use sa_core::field::ptr_of as p;

fn paths() -> GPtr<u8> {
    g(THE_PATHS)
}

/// `AddNodeToNewInterior(&out, pos, flag, -1 x6)`, the node address dword.
fn add_node(pos: GPtr<u8>, flag: u32) -> u32 {
    let mut out: u32 = 0;
    let r = add_node_to_new_interior(paths(), p(&mut out), rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8), flag,
                                     -1, -1, -1, -1, -1, -1);
    rd::<u32>(r, 0)
}

/// How a side sums the squared distance from the exterior node to its outer door point.
#[derive(Clone, Copy, PartialEq)]
enum SumOrder {
    /// `(dz² + dy²) + dx²` (side A)
    Zyx,
    /// `(dx² + dz²) + dy²` (sides B, C, D)
    Xzy,
}

/// The per-frame door records of the pairing pass.
struct Doors {
    count: usize,
    pos: [[u32; 3]; 32],
    node: [i32; 32],
    used: [u8; 32],
}

/// One side of an interior: `inner` / `outer` the two door points (+0x514.. / +0x524.. for side A,
/// 0x20 apart per side), `links` the two interior node offsets (+0x512.. relative to the interior's
/// first walk node), `mark` whether the side may set the first-door fields (+0x3F0 / +0x400) and the
/// flag that only the first such side sets them, `skip_neg` side A's `jle` test of the link offsets.
/// Returns the inner node's index (the side's entry node for the no-walk-node links).
#[allow(clippy::too_many_arguments)]
fn side(group: GPtr<u8>, it: GPtr<u8>, inner: isize, links: isize, first: i32, skip_neg: bool, outer_flag: u32,
        mark: Option<&mut bool>, sum: SumOrder, edi: &mut i32, ebx: &mut i32, doors: &mut Doors) -> i32 {
    let addr = add_node(it.byte_add(inner), 0);
    for k in 0..2 {
        let l = rd::<i8>(it, links + k);
        let take = if skip_neg { l > -1 } else { l != -1 };
        if take {
            add_interior_link(paths(), *edi, first + i32::from(l));
        }
    }
    if let Some(flag) = mark.filter(|f| !**f) {
        for k in 0..3 {
            wr::<u32>(it, 0x400 + 4 * k, rd::<u32>(it, inner + 4 * k));
        }
        wr::<u32>(it, 0x3F0, addr);
        *flag = true;
    }
    let entry = *edi;
    *edi += 1;
    *ebx += 1;
    let outer = inner + 0x10;
    add_node(it.byte_add(outer), outer_flag);
    add_interior_link(paths(), *edi, *ebx);
    let mut ext: u32 = 0;
    find_nearest_exterior_node(paths(), p(&mut ext), *edi);
    if ext as u16 == 0xFFFF {
        let n = doors.count;
        for k in 0..3 {
            doors.pos[n][k] = rd::<u32>(it, outer + 4 * k as isize);
        }
        doors.node[n] = *edi;
        doors.used[n] = 0;
        doors.count += 1;
    } else {
        wr::<u32>(it, 0x3EC, ext);
        let area = (ext & 0xFFFF) as usize;
        let id = (ext >> 16) as isize;
        let node = g::<GPtr<u8>>(PATH_NODES + area * 4).read().byte_add(id * 0x1C);
        let c = |o: isize| xi(i32::from(rd::<i16>(node, o))).fmul_m(F_0_125);
        for k in 0..3 {
            wr::<f32>(it, 0x3F4 + 4 * k as isize, c(8 + 2 * k as isize).to_f32());
        }
        if rd::<u8>(group, 0xE) == 0 {
            let t = [rd::<f32>(it, outer), rd::<f32>(it, outer + 4), rd::<f32>(it, outer + 8)];
            let z = c(0xC).to_f32();
            let dx = c(8).fsub_m(t[0]).to_f32();
            let dy = c(0xA).fsub_m(t[1]);
            let dz = x(z).fsub_m(t[2]);
            let d2 = match sum {
                SumOrder::Zyx => dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)),
                SumOrder::Xzy => x(dx).fmul_m(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy)),
            };
            if !d2.fcom(x(F_9)).test_parity(0x41) {
                add_interior_link_to_external_node(paths(), *edi, ext);
            }
        }
    }
    *edi += 1;
    *ebx += 1;
    entry
}

/// `InteriorGroup_c::SetupShopPeds()` (0x594C10): streams the interior's peds
/// (`StreamPedsForInterior(1)`, `LoadAllRequestedModels(false)`), clears the ped count (+0x36) and
/// adds `(2 - ftol(rand() * -3 / 32768)) * (+0xF) + 1` peds: the first from ped slot 0 as the shop
/// keeper, the others from slot `1 - ftol(rand() * -7 / 32768)` as shoppers, each on a random
/// status-3 tile of a random interior of the group (tile centre, z + 1), created by 2, decision
/// maker 7, stored at +0x38 + 4 * n with the count incremented, and given
/// `CTaskInteriorShopKeeper(group, false)` / `CTaskInteriorBeInShop(group)` in task slot 4. A failed
/// `AddPed` still overwrites the slot with null.
#[hook(addr = 0x594C10, conv = thiscall, name = "InteriorGroup_c::SetupShopPeds", this = "InteriorGroup_c",
       expect = "6A FF 68 96 D0 83 00",
       unverified = "world6-2", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "rand at_site 0x594C45, 0x594CA1; the tile x / y are uninitialised stack words (kept across peds) when GetRandomTile does not write them (0 here)")]
pub fn setup_shop_peds(this: GPtr<u8>) {
    stream_peds_for_interior(1);
    load_all_requested_models(0);
    wr::<u8>(this, 0x36, 0);
    let r = rand_scaled(0x59_4C45, f32::from_bits(0xC040_0000));
    let count = (2 - r).wrapping_mul(i32::from(rd::<i8>(this, 0xF))).wrapping_add(1);
    let (mut tx, mut ty) = (0i32, 0i32); // [esp+0x1C], [esp+0x18]
    let mut i = 0;
    while i < count {
        let slot = if i == 0 { 0 } else { 1 - rand_scaled(0x59_4CA1, f32::from_bits(0xC0E0_0000)) };
        let model = find_mi_ped_slot_for_interior(slot);
        let ped_type = rd::<u32>(g::<GPtr<u8>>((MODEL_INFOS as isize + model as isize * 4) as usize).read(), 0x28);
        let mut pos = [0f32; 3];
        let it = get_random_interior(this);
        get_random_tile(it, 3, p(&mut tx), p(&mut ty));
        get_tile_centre(it, xi(tx).to_f32(), xi(ty).to_f32(), p(&mut pos));
        pos[2] = x(pos[2]).fadd_m(1.0).to_f32();
        let n = isize::from(rd::<i8>(this, 0x36));
        let ped = add_ped(ped_type, model as u32, GPtr::<u8>::cast::<f32>(p(&mut pos)), 0);
        wr::<GPtr<u8>>(this, 0x38 + n * 4, ped);
        if !ped.is_null() {
            wr::<u8>(this, 0x36, rd::<u8>(this, 0x36).wrapping_add(1));
            let slot_ped = || rd::<GPtr<u8>>(this, 0x38 + n * 4);
            set_char_created_by(slot_ped(), 2);
            set_ped_decision_maker_type(rd::<GPtr<u8>>(slot_ped(), 0x47C), 7);
            let task = if i == 0 {
                let m = task_new(0x20);
                if m.is_null() { GPtr::NULL } else { shop_keeper_ctor(m, this, 0) }
            } else {
                let m = task_new(0x34);
                if m.is_null() { GPtr::NULL } else { be_in_shop_ctor(m, this) }
            };
            task_manager_set_task(rd::<GPtr<u8>>(slot_ped(), 0x47C).byte_add(4), task, 4, 0);
        }
        i += 1;
    }
}

/// `InteriorGroup_c::SetupPaths()` (0x595590). Nothing until `ArePathsLoaded`. Then a new interior
/// of `ThePaths` (id: the group's +0xC) gets, per interior of the group (+0x10, 8 slots), its walk
/// nodes (+0x40C count; the first two at +0x414 / +0x424 linked together, every further node at
/// +0x434 + 16 * (n - 2) linked to node n - 2 and, for odd n, also to n - 1; an interior with at most
/// two nodes adds none but still advances the running base index). Then, per interior and per open
/// side (interior info +5 / +6 / +0xA / +8 > -1), an inner door node (linked to up to two walk
/// nodes) and an outer one (linked to the inner node and, when an exterior node is near it, to that
/// node within 3 units unless the group's +0xE is set; its address and position are kept at +0x3EC /
/// +0x3F4); the first door sets +0x3F0 / +0x400. An interior without walk nodes links its doors'
/// inner nodes (A to B, C, D; C to B, D). Outer doors without an exterior node are paired with the
/// nearest unpaired one closer than sqrt(3) (`AddInteriorLink`), or passed to 0x44DF60. Finally
/// `CompleteNewInterior(null)` and +0xB8 = 1.
#[hook(addr = 0x595590, conv = thiscall, name = "InteriorGroup_c::SetupPaths", this = "InteriorGroup_c",
       expect = "81 EC 50 02 00 00",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       recs = "grove-walk",
       note = "builds ThePaths' new interior (heap); the original's door arrays overflow its frame above 12 unconnected doors; first-node slots of empty interior slots stay unwritten (never read)")]
#[allow(clippy::needless_range_loop)]
pub fn setup_paths(this: GPtr<u8>) {
    if are_paths_loaded(this) == 0 {
        return;
    }
    start_new_interior(paths(), i32::from(rd::<i8>(this, 0xC)));
    let mut first = [0i32; 8];
    let mut base: i32 = 0;
    let mut edi: i32 = 0;
    for k in 0..8 {
        let it = rd::<GPtr<u8>>(this, 0x10 + 4 * k as isize);
        if it.is_null() {
            continue;
        }
        first[k] = base;
        if rd::<i8>(it, 0x40C) > 2 {
            add_node(it.byte_add(0x414), 0);
            add_node(it.byte_add(0x424), 0);
            edi += 1;
            add_interior_link(paths(), edi, base);
            edi += 1;
            let mut n: i32 = 2;
            if rd::<i8>(it, 0x40C) > 2 {
                loop {
                    add_node(it.byte_add(0x434 + 0x10 * (n as isize - 2)), 0);
                    add_interior_link(paths(), edi, base + n - 2);
                    if n % 2 != 0 {
                        add_interior_link(paths(), edi, base + n - 1);
                    }
                    edi += 1;
                    n += 1;
                    if n >= i32::from(rd::<i8>(it, 0x40C)) {
                        break;
                    }
                }
            }
        }
        base += i32::from(rd::<i8>(it, 0x40C));
    }
    let mut doors = Doors { count: 0, pos: [[0; 3]; 32], node: [0; 32], used: [0; 32] };
    let mut edi = base;
    let mut ebx = base - 1;
    for k in 0..8 {
        let it = rd::<GPtr<u8>>(this, 0x10 + 4 * k as isize);
        if it.is_null() {
            continue;
        }
        let info = || rd::<GPtr<u8>>(it, 0x14);
        let mut marked = false;
        let mut entry = [0i32; 4]; // A [esp+0x30], B [esp+0x4C], C [esp+0x28], D [esp+0x48]
        let outer_flag = u32::from(rd::<u8>(this, 0xE) != 0);
        if rd::<i8>(info(), 5) > -1 {
            entry[0] = side(this, it, 0x514, 0x512, first[k], true, outer_flag, Some(&mut marked), SumOrder::Zyx,
                            &mut edi, &mut ebx, &mut doors);
            marked = true;
        }
        if rd::<i8>(info(), 6) > -1 {
            entry[1] = side(this, it, 0x534, 0x532, first[k], false, 0, Some(&mut marked), SumOrder::Xzy,
                            &mut edi, &mut ebx, &mut doors);
        }
        if rd::<i8>(info(), 0xA) > -1 {
            entry[2] = side(this, it, 0x554, 0x552, first[k], false, 0, Some(&mut marked), SumOrder::Xzy,
                            &mut edi, &mut ebx, &mut doors);
        }
        if rd::<i8>(info(), 8) > -1 {
            entry[3] = side(this, it, 0x574, 0x572, first[k], false, 0, Some(&mut marked), SumOrder::Xzy,
                            &mut edi, &mut ebx, &mut doors);
        }
        if rd::<u8>(it, 0x40C) == 0 {
            if rd::<i8>(info(), 5) > -1 {
                if rd::<i8>(info(), 6) > -1 {
                    add_interior_link(paths(), entry[0], entry[1]);
                }
                if rd::<i8>(info(), 0xA) > -1 {
                    add_interior_link(paths(), entry[0], entry[2]);
                }
                if rd::<i8>(info(), 8) > -1 {
                    add_interior_link(paths(), entry[0], entry[3]);
                }
            }
            if rd::<i8>(info(), 0xA) > -1 {
                if rd::<i8>(info(), 6) > -1 {
                    add_interior_link(paths(), entry[2], entry[1]);
                }
                if rd::<i8>(info(), 8) > -1 {
                    add_interior_link(paths(), entry[2], entry[3]);
                }
            }
        }
    }
    for i in 0..doors.count {
        if doors.used[i] != 0 {
            continue;
        }
        let mut best = x(F_1E9);
        let mut pick: i32 = -1;
        let pi = doors.pos[i].map(f32::from_bits);
        for j in i + 1..doors.count {
            if doors.used[j] != 0 {
                continue;
            }
            let pj = doors.pos[j].map(f32::from_bits);
            let dx = x(pj[0]).fsubr_m(pi[0]);
            let dy = x(pi[1]).fsub_m(pj[1]);
            let dz = x(pi[2]).fsub_m(pj[2]);
            let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
            if !d2.fcom(best).test_parity(0x05) {
                best = x(d2.to_f32());
                pick = j as i32;
            }
        }
        if best.fcom(x(F_3)).test_parity(0x05) {
            path_44df60(paths(), doors.node[i]);
        } else {
            doors.used[i] = 1;
            doors.used[pick as usize] = 1;
            add_interior_link(paths(), doors.node[i], doors.node[pick as usize]);
        }
    }
    complete_new_interior(paths(), 0);
    wr::<u8>(this, 0xB8, 1);
}

orig! {
    /// `InteriorGroup_c::GetNumInteriorInfos(int32 type)` (0x594920)
    fn get_num_interior_infos(this: GPtr<u8>, ty: i32) -> i32
        = thiscall 0x594920 => crate::world::misc2::interior_group_get_num_interior_infos;
    /// `CTaskInteriorBeInOffice::CTaskInteriorBeInOffice(InteriorGroup_c*)` (0x675220)
    fn be_in_office_ctor(t: GPtr<u8>, group: GPtr<u8>) -> GPtr<u8> = thiscall 0x675220 => unported;
}
use crate::bindings::{ped_is_pointer_valid, population_remove_ped};
use sa_core::bindings::{general_random_int_in_range, matrix_mul_vec};
use sa_stream::bindings::clock_get_is_time_in_range;

/// `InteriorGroup_c::UpdateOfficePeds()` (0x594E90, thiscall). With +0x34 set and +0x35 clear: the group entity's
/// (+8) position as `GetMatrix() * (0, 0, 0)` (the inlined GetMatrix), `n = GetNumInteriorInfos(6)`; from 9 to 18
/// `GetRandomNumberInRange(n / 2, n)` peds, from 18 to 22 or 6 to 9 `GetRandomNumberInRange(0, n / 2)` (both capped
/// at 16), otherwise none (`n / 2` rounds towards zero). Each one: a model from `FindMIPedSlotForInterior(ftol((rand()
/// & 0xFFFF) / 32768 * 8))` (rand site 0x594F9C) added at that position with its model info's ped type (+0x28);
/// a ped goes to slot +0x38 + 4 * count (+0x36, the slot written even with no ped), is created by 2 (mission-free),
/// gets decision maker 7 and the primary task `CTaskInteriorBeInOffice(this)` (slot 4). With +0x34 clear and +0x35
/// set: every valid ped of the 16 slots is removed from the population, the slots and the count cleared.
#[hook(addr = 0x594E90, conv = thiscall, name = "InteriorGroup_c::UpdateOfficePeds", this = "InteriorGroup_c",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a25", route = "L2", hazards = "heap-writes",
       note = "creates / removes peds (pools); rand site 0x594F9C")]
pub fn interior_group_update_office_peds(this: GPtr<u8>) {
    if rd::<u8>(this, 0x34) == 0 {
        if rd::<u8>(this, 0x35) == 0 {
            return;
        }
        for k in 0..16isize {
            let o = 0x38 + 4 * k;
            let ped = rd::<GPtr<u8>>(this, o);
            if !ped.is_null() {
                if ped_is_pointer_valid(ped) != 0 {
                    population_remove_ped(rd::<GPtr<u8>>(this, o));
                }
                wr::<u32>(this, o, 0);
            }
        }
        wr::<u8>(this, 0x36, 0);
        return;
    }
    if rd::<u8>(this, 0x35) != 0 {
        return;
    }
    let ent = rd::<GPtr<u8>>(this, 8);
    let mut origin = [0u32; 3];
    let mut out = [0u32; 3];
    let m = crate::world::entity::get_matrix(ent);
    let r = matrix_mul_vec(p(&mut out).cast(), m, p(&mut origin).cast());
    let mut pos = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
    let n = get_num_interior_infos(this, 6);
    let half = n / 2;
    let count = if clock_get_is_time_in_range(9, 18) != 0 {
        general_random_int_in_range(half, n).min(16)
    } else if clock_get_is_time_in_range(18, 22) != 0 || clock_get_is_time_in_range(6, 9) != 0 {
        general_random_int_in_range(0, half).min(16)
    } else {
        0
    };
    for _ in 0..count.max(0) {
        let model = find_mi_ped_slot_for_interior(rand_scaled(0x59_4F9C, 8.0));
        let mi = g::<GPtr<u8>>(MODEL_INFOS + 4 * model as usize).read();
        let slot = i32::from(rd::<i8>(this, 0x36));
        let ped = add_ped(rd::<u32>(mi, 0x28), model as u32, p(&mut pos).cast(), 0);
        let o = 0x38 + 4 * slot as isize;
        wr::<GPtr<u8>>(this, o, ped);
        if ped.is_null() {
            continue;
        }
        wr::<u8>(this, 0x36, rd::<u8>(this, 0x36).wrapping_add(1));
        set_char_created_by(rd::<GPtr<u8>>(this, o), 2);
        set_ped_decision_maker_type(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, o), 0x47C), 7);
        let t = task_new(0x18);
        let task = if t.is_null() { GPtr::NULL } else { be_in_office_ctor(t, this) };
        task_manager_set_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, o), 0x47C).byte_add(4), task, 4, 0);
    }
}

orig! {
    /// `CScriptsForBrains::StartOrRequestNewStreamedScriptBrainWithThisName(const char*, CEntity*, uint8)` (0x46CED0)
    fn start_brain_with_name(brains: GPtr<u8>, name: GPtr<u8>, ent: GPtr<u8>, ty: u32) = thiscall 0x46CED0 => unported;
    /// `CTaskInteriorBeInHouse::CTaskInteriorBeInHouse(InteriorGroup_c*)` (0x674FC0)
    fn be_in_house_ctor(t: GPtr<u8>, group: GPtr<u8>) -> GPtr<u8> = thiscall 0x674FC0 => unported;
}
use super::interior_group_update::ig_remove_ped;
use crate::bindings::{choose_gang_occupation, pick_gang_to_create_members_of};

/// `InteriorGroup_c::SetupHousePeds()` (0x5965E0, thiscall). `StreamPedsForInterior(0)`,
/// `LoadAllRequestedModels(false)`, the ped count (+0x36) cleared, the position `GetMatrix(+8) * (0, 0, 0)` (the
/// inlined GetMatrix). `ftol((rand() & 0xFFFF) / 32768 * 100) <= 50` gives 2 peds, else 1 (rand site 0x59666A).
/// With the gang byte 0xC0BC68 set: `PickGangToCreateMembersOf()`; a gang with `ChooseGangOccupation(gang - 7) >
/// -1` makes a gang house when a second roll (0x5966CC) is at most 15, then with 3 peds, 4 when `rand()` (0x5966FA)
/// >= 0x3FFF. Per ped k: the model is `ChooseGangOccupation(gang - 7)` in a gang house, else
/// `FindMIPedSlotForInterior(rand() >= 0x3FFF)` (0x596742) for a single ped or `FindMIPedSlotForInterior(k)`; added at
/// the position with its model info's ped type (+0x28), stored at slot +0x38 + 4 * count (also when null); a ped
/// increments the count, gets the "house" brain (0x866D4C, type 3, `ScriptsForBrains` 0xA90CF0), created by 2,
/// decision maker 7 and the primary task `CTaskInteriorBeInHouse(this)` (slot 4). When the slot flag 0xBB3D9C + k is
/// clear, `RemovePed(slot)` (the slot's ped, null included).
#[hook(addr = 0x5965E0, conv = thiscall, name = "InteriorGroup_c::SetupHousePeds", this = "InteriorGroup_c",
       expect = "6A FF 68 CB D0 83 00",
       unverified = "frontier2-a25", route = "L2", hazards = "heap-writes",
       note = "creates peds (pools); rand sites 0x59666A, 0x5966CC, 0x5966FA, 0x596742")]
pub fn interior_group_setup_house_peds(this: GPtr<u8>) {
    stream_peds_for_interior(0);
    load_all_requested_models(0);
    wr::<u8>(this, 0x36, 0);
    let ent = rd::<GPtr<u8>>(this, 8);
    let mut origin = [0u32; 3];
    let mut out = [0u32; 3];
    let m = crate::world::entity::get_matrix(ent);
    let r = matrix_mul_vec(p(&mut out).cast(), m, p(&mut origin).cast());
    let mut pos = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
    // [esp+0x18]: the first roll's masked rand, overwritten by the picked gang
    let mut gang = (at_site(0x59_666A, rand) & 0xFFFF) as i32;
    let roll = ftol32(xi(gang).fmul_m(INV_32768).fmul_m(100.0));
    let mut count = i32::from(roll <= 0x32) + 1;
    let mut gang_house = false;
    if g::<u32>(0xC0_BC68).read() != 0 {
        gang = pick_gang_to_create_members_of();
        if gang != 0 && choose_gang_occupation(gang - 7) > -1 {
            gang_house = true;
            if rand_scaled(0x59_66CC, 100.0) > 0xF {
                gang_house = false;
            } else {
                count = i32::from(at_site(0x59_66FA, rand) >= 0x3FFF) + 3;
            }
        }
    }
    let mut k = 0i32;
    while k < count {
        let model = if gang_house {
            choose_gang_occupation(gang - 7)
        } else if count == 1 {
            find_mi_ped_slot_for_interior(i32::from(at_site(0x59_6742, rand) >= 0x3FFF))
        } else {
            find_mi_ped_slot_for_interior(k)
        };
        let mi = g::<GPtr<u8>>(MODEL_INFOS + 4 * model as usize).read();
        let slot = isize::from(rd::<i8>(this, 0x36));
        let o = 0x38 + 4 * slot;
        let ped = add_ped(rd::<u32>(mi, 0x28), model as u32, p(&mut pos).cast(), 0);
        wr::<GPtr<u8>>(this, o, ped);
        if !ped.is_null() {
            wr::<u8>(this, 0x36, rd::<u8>(this, 0x36).wrapping_add(1));
            start_brain_with_name(GPtr::from_addr(0xA9_0CF0), GPtr::from_addr(0x86_6D4C), rd::<GPtr<u8>>(this, o), 3);
            set_char_created_by(rd::<GPtr<u8>>(this, o), 2);
            set_ped_decision_maker_type(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, o), 0x47C), 7);
            let t = task_new(0x18);
            let task = if t.is_null() { GPtr::NULL } else { be_in_house_ctor(t, this) };
            task_manager_set_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, o), 0x47C).byte_add(4), task, 4, 0);
        }
        if g::<u8>(0xBB_3D9C + k as usize).read() == 0 {
            ig_remove_ped(this, rd::<GPtr<u8>>(this, o));
        }
        k += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SETUP_SHOP_PEDS,
    HOOK_SETUP_PATHS,
    HOOK_INTERIOR_GROUP_UPDATE_OFFICE_PEDS,
    HOOK_INTERIOR_GROUP_SETUP_HOUSE_PEDS,
];

