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

orig! {
    /// `InteriorGroup_c::ArePathsLoaded()` (0x595380)
    fn are_paths_loaded(this: GPtr<u8>) -> u8 = thiscall 0x595380 => super::interior_paths_loaded::are_paths_loaded;
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
    /// `CStreaming::StreamPedsForInterior(int32)` (0x40BBB0)
    fn stream_peds_for_interior(kind: i32) = cdecl 0x40BBB0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn load_all_requested_models(only_priority: u32) = cdecl 0x40EA10 => unported;
    /// `CStreaming::FindMIPedSlotForInterior(int32)` (0x407FB0)
    fn find_mi_ped_slot_for_interior(rand_val: i32) -> i32 = cdecl 0x407FB0 => unported;
    /// `InteriorGroup_c::GetRandomInterior()` (0x5948C0)
    fn get_random_interior(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5948C0 => unported;
    /// `Interior_c::GetRandomTile(int32 status, int32* x, int32* y)` (0x591B20)
    fn get_random_tile(it: GPtr<u8>, status: i32, tx: GPtr<u8>, ty: GPtr<u8>) -> u8 = thiscall 0x591B20 => unported;
    /// `Interior_c::GetTileCentre(float x, float y, CVector* out)` (0x591BD0)
    fn get_tile_centre(it: GPtr<u8>, tx: f32, ty: f32, out: GPtr<u8>) = thiscall 0x591BD0 => unported;
    /// `CPopulation::AddPed(ePedType, uint32 model, const CVector&, bool)` (0x612710)
    fn add_ped(ty: u32, model: i32, pos: GPtr<u8>, b: u32) -> GPtr<u8> = cdecl 0x612710 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(ped: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    fn set_ped_decision_maker_type(intel: GPtr<u8>, ty: i32) = thiscall 0x600B50 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskInteriorShopKeeper::CTaskInteriorShopKeeper(InteriorGroup_c*, bool)` (0x675660)
    fn shop_keeper_ctor(t: GPtr<u8>, group: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x675660 => unported;
    /// `CTaskInteriorBeInShop::CTaskInteriorBeInShop(InteriorGroup_c*)` (0x6753E0)
    fn be_in_shop_ctor(t: GPtr<u8>, group: GPtr<u8>) -> GPtr<u8> = thiscall 0x6753E0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0)
    fn task_manager_set_task(tm: GPtr<u8>, task: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
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

/// A Rust local passed by address to an original callee.
fn p<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

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
        let ped = add_ped(ped_type, model, p(&mut pos), 0);
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

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SETUP_SHOP_PEDS, HOOK_SETUP_PATHS];

