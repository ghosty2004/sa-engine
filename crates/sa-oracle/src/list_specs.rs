//! L1 specs of `CMatrixLink` / `CMatrixLinkList` (pointer manipulation only): a `CMatrixLinkList` and
//! eight `CMatrixLink`s in scratch memory, distributed over the three lists by the inputs.
//!
//! Inputs (raw words, no floats): `[assign, order, pick_a, pick_b]`: `assign` gives each link a list (2
//! bits each: 0 = list 1, 1 = list 2, 2/3 = free), `order` permutes them, the picks choose the `this`
//! and argument links.

use crate::call::*;
use crate::generate::Rng;
use crate::specs::*;
use sa_math::matrix_link::{self as ml, Mem32};
use std::sync::Arc;

const LIST: u32 = 0x10;
const LINKS: u32 = 0x210;
const N: u32 = 8;

struct M([u8; SCRATCH_LEN]);

impl Mem32 for M {
    fn rd(&self, a: u32) -> u32 {
        let o = (a - SCRATCH) as usize;
        u32::from_le_bytes(self.0[o..o + 4].try_into().unwrap())
    }
    fn wr(&mut self, a: u32, v: u32) {
        let o = (a - SCRATCH) as usize;
        self.0[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
}

fn list() -> u32 {
    SCRATCH + LIST
}

fn link(i: u32) -> u32 {
    SCRATCH + LINKS + i * ml::LINK_SIZE
}

/// The links in list order per list (0, 1, 2 = free).
fn lists(inp: &Inputs) -> [Vec<u32>; 3] {
    let mut order: Vec<u32> = (0..N).collect();
    let mut r = Rng::new(inp[1] as u64);
    for i in (1..order.len()).rev() {
        let j = r.below(i as u32 + 1) as usize;
        order.swap(i, j);
    }
    let mut l: [Vec<u32>; 3] = Default::default();
    for i in order {
        let a = ((inp[0] >> (2 * i)) & 3).min(2) as usize;
        l[a].push(i);
    }
    l
}

/// Every link that is in a list (not a sentinel).
fn members(inp: &Inputs) -> Vec<u32> {
    lists(inp).iter().flatten().copied().collect()
}

fn scratch(inp: &Inputs) -> [u8; SCRATCH_LEN] {
    let mut m = M([FILL; SCRATCH_LEN]);
    let heads = [ml::LIST_HEAD, ml::LIST_ALLOC_HEAD, ml::LIST_FREE_HEAD];
    let tails = [ml::LIST_TAIL, ml::LIST_ALLOC_TAIL, ml::LIST_FREE_TAIL];
    for (k, l) in lists(inp).iter().enumerate() {
        let mut chain = vec![list() + heads[k]];
        chain.extend(l.iter().map(|&i| link(i)));
        chain.push(list() + tails[k]);
        for w in chain.windows(2) {
            m.wr(w[0] + ml::LINK_NEXT, w[1]);
            m.wr(w[1] + ml::LINK_PREV, w[0]);
        }
    }
    m.wr(list() + ml::LIST_OBJECTS, link(0));
    m.0
}

fn gen_case(r: &mut Rng) -> Inputs {
    let assign = match r.below(4) {
        0 => 0xAAAA,                // all free
        1 => 0x0000,                // all in list 1
        2 => r.next_u32() & 0x5555, // lists 1 and 2, none free
        _ => r.next_u32(),
    };
    vec![assign, r.next_u32(), r.next_u32(), r.next_u32()]
}

fn edges() -> Vec<Inputs> {
    let mut v = Vec::new();
    for assign in [0u32, 0xFFFF, 0xAAAA, 0x5555, 0x0003, 0xC000, 0x1234, 0xFEDC] {
        for order in 0..8 {
            for pick in 0..4 {
                v.push(vec![assign, order, pick, pick * 7 + 1]);
            }
        }
    }
    v
}

#[derive(Clone, Copy)]
enum Op {
    Insert,
    Remove,
    AddToList1,
    AddToList2,
    MoveToList1,
    MoveToList2,
    MoveToFreeList,
    GetNumFree,
    GetNumUsed1,
    GetNumUsed2,
}

/// `this` and the argument of a case.
fn picks(op: Op, inp: &Inputs) -> (u32, Option<u32>) {
    let mem = members(inp);
    let pick_member = |p: u32| mem.get(p as usize % mem.len().max(1)).map(|&i| link(i));
    match op {
        Op::Insert => {
            // `this`: a head sentinel or a member (its `next` is valid); `link`: any link
            let heads = [ml::LIST_HEAD, ml::LIST_ALLOC_HEAD, ml::LIST_FREE_HEAD];
            let this = match pick_member(inp[2]) {
                Some(l) if !inp[2].is_multiple_of(4) => l,
                _ => list() + heads[(inp[2] / 4 % 3) as usize],
            };
            (this, Some(link(inp[3] % N)))
        }
        Op::Remove => (pick_member(inp[2]).unwrap_or(link(0)), None),
        Op::MoveToList1 | Op::MoveToList2 | Op::MoveToFreeList => (list(), pick_member(inp[2])),
        _ => (list(), None),
    }
}

/// Inputs whose case is valid for `op` (Remove/MoveTo need a link that is in a list).
fn valid(op: Op, inp: &Inputs) -> bool {
    match op {
        Op::Remove | Op::MoveToList1 | Op::MoveToList2 | Op::MoveToFreeList => !members(inp).is_empty(),
        _ => true,
    }
}

fn spec(key: &'static str, name: &'static str, addr: u32, op: Op) -> Spec {
    let ret = match op {
        Op::AddToList1 | Op::AddToList2 | Op::GetNumFree | Op::GetNumUsed1 | Op::GetNumUsed2 => RetKind::Eax,
        _ => RetKind::Void,
    };
    let repair = move |mut v: Inputs| {
        if !valid(op, &v) {
            v[0] = 0; // everything in list 1
        }
        v
    };
    Spec {
        key,
        name,
        addr,
        ret,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 10_000,
        floats: 0,
        generate: Arc::new(move |r| repair(gen_case(r))),
        edges: Arc::new(move || edges().into_iter().map(repair).collect()),
        call: Arc::new(move |inp| {
            let (this, arg) = picks(op, inp);
            let args: Vec<Arg> = std::iter::once(Arg::Ptr(this)).chain(arg.map(Arg::Ptr)).collect();
            let mut c = Call::new(addr, Conv::Thiscall, &args);
            c.scratch = scratch(inp);
            c
        }),
        rust: Arc::new(move |inp, s| {
            let (this, arg) = picks(op, inp);
            let mut m = M(*s);
            let ret = match op {
                Op::Insert => {
                    ml::insert(&mut m, this, arg.unwrap());
                    ExpRet::Void
                }
                Op::Remove => {
                    ml::remove(&mut m, this);
                    ExpRet::Void
                }
                Op::AddToList1 => ExpRet::Eax(ml::add_to_list1(&mut m, this)),
                Op::AddToList2 => ExpRet::Eax(ml::add_to_list2(&mut m, this)),
                Op::MoveToList1 => {
                    ml::move_to_list1(&mut m, this, arg.unwrap());
                    ExpRet::Void
                }
                Op::MoveToList2 => {
                    ml::move_to_list2(&mut m, this, arg.unwrap());
                    ExpRet::Void
                }
                Op::MoveToFreeList => {
                    ml::move_to_free_list(&mut m, this, arg.unwrap());
                    ExpRet::Void
                }
                Op::GetNumFree => ExpRet::Eax(ml::get_num_free(&m, this)),
                Op::GetNumUsed1 => ExpRet::Eax(ml::get_num_used1(&m, this)),
                Op::GetNumUsed2 => ExpRet::Eax(ml::get_num_used2(&m, this)),
            };
            Expect::Returns { ret, scratch: m.0 }
        }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

pub fn all() -> Vec<Spec> {
    vec![
        spec("CMatrixLinkInsert", "CMatrixLink::Insert", 0x54E8F0, Op::Insert),
        spec("CMatrixLinkRemove", "CMatrixLink::Remove", 0x54E910, Op::Remove),
        spec(
            "CMatrixLinkListAddToList1",
            "CMatrixLinkList::AddToList1",
            0x54E9D0,
            Op::AddToList1,
        ),
        spec(
            "CMatrixLinkListAddToList2",
            "CMatrixLinkList::AddToList2",
            0x54EA10,
            Op::AddToList2,
        ),
        spec(
            "CMatrixLinkListMoveToList1",
            "CMatrixLinkList::MoveToList1",
            0x54EA60,
            Op::MoveToList1,
        ),
        spec(
            "CMatrixLinkListMoveToList2",
            "CMatrixLinkList::MoveToList2",
            0x54EA90,
            Op::MoveToList2,
        ),
        spec(
            "CMatrixLinkListMoveToFreeList",
            "CMatrixLinkList::MoveToFreeList",
            0x54EAD0,
            Op::MoveToFreeList,
        ),
        spec(
            "CMatrixLinkListGetNumFree",
            "CMatrixLinkList::GetNumFree",
            0x54EB10,
            Op::GetNumFree,
        ),
        spec(
            "CMatrixLinkListGetNumUsed1",
            "CMatrixLinkList::GetNumUsed1",
            0x54EB30,
            Op::GetNumUsed1,
        ),
        spec(
            "CMatrixLinkListGetNumUsed2",
            "CMatrixLinkList::GetNumUsed2",
            0x54EB50,
            Op::GetNumUsed2,
        ),
    ]
}
