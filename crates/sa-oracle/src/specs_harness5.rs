//! L1 specs for HARNESS-5's register-convention ports (`sa_core::regconv`): the call's entry
//! registers come from the hook's own spec (`Call::regconv`), with random garbage in the register
//! bytes an AL/DI-style location does not cover and in the upper half of word-sized stack arguments.
//!
//! - `GetCharacterPositionInCharSet` 0x6FD7C0: AL + two `uint32&` (scratch +0, +4);
//! - `ToggleEdge` 0x70FA70 (`FUN_0070fa70`): ECX = edge array (scratch +0x10), EDX = `u16*` count
//!   (scratch +0), DI = vertex a, stack word b; ECX/EDX returned unchanged (`keep = ecx edx`, checked
//!   by the runner's preserve rule against the hook registration).

use crate::call::*;
use crate::generate::Rng;
use crate::specs::{ExpRet, Expect, FILL, Inputs, RetKind, Route, Spec};
use sa_core::mem::GPtr;
use std::sync::Arc;

fn regconv(addr: u32) -> sa_core::regconv::RegConv {
    crate::hook_regconv(addr).expect("a register-convention hook")
}

fn host_ptr<T>(m: &mut [u8; SCRATCH_LEN], off: usize) -> GPtr<T> {
    GPtr::from_raw(m.as_mut_ptr().wrapping_add(off).cast())
}

fn base(key: &'static str, name: &'static str, addr: u32) -> Spec {
    Spec {
        preserve: 0,
        key,
        name,
        addr,
        ret: RetKind::Void,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 200_000,
        floats: 0,
        generate: Arc::new(|_| vec![]),
        edges: Arc::new(Vec::new),
        call: Arc::new(|_| Call::new(0, Conv::Cdecl, &[])),
        rust: Arc::new(|_, s| Expect::Returns { ret: ExpRet::Void, scratch: *s }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

// ------------------------------------------------------------ GetCharacterPositionInCharSet ------

const CHARSET: u32 = 0x6FD7C0;

/// Inputs: [character byte, register garbage, scratch fill word].
fn char_call(i: &Inputs) -> Call {
    let mut c = Call::regconv(CHARSET, &regconv(CHARSET), &[i[0], SCRATCH, SCRATCH + 4], i[1]);
    for w in c.scratch.chunks_mut(4) {
        w.copy_from_slice(&i[2].to_le_bytes());
    }
    c
}

pub fn char_position() -> Spec {
    Spec {
        generate: Arc::new(|r: &mut Rng| vec![r.below(256), r.next_u32(), r.next_u32()]),
        edges: Arc::new(|| (0..256).map(|c| vec![c, 0xDEAD_BE00, 0xA5A5_A5A5]).collect()),
        call: Arc::new(char_call),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            let (col, row) = (host_ptr(&mut m, 0), host_ptr(&mut m, 4));
            sa_render::carplates::get_character_position_in_char_set(i[0] as u8, col, row);
            Expect::Returns { ret: ExpRet::Void, scratch: m }
        }),
        // CPU-decided: the static scan decodes the jump-table bytes after the code as SSE
        // (`movups` at 0x6FDBBD), which the route rule sends to the native backend
        route: Route::L2 { reason: "static scan: jump-table bytes decode as SSE (0x6FDBBD)" },
        ..base("CharPosition", "CCustomCarPlateMgr::GetCharacterPositionInCharSet", CHARSET)
    }
}

// ------------------------------------------------------------------------ ToggleEdge ------------

const TOGGLE: u32 = 0x70FA70;
const EDGES: usize = 0x10;
/// Most pairs before the call (the append writes one more).
const MAX_PAIRS: u32 = 64;

/// Inputs: [a, b, garbage for EDI's upper half / b's upper half, count, pairs...]: vertex values
/// from a small range so that matches (removal) are frequent.
fn gen_toggle(r: &mut Rng) -> Inputs {
    let k = if r.chance(30) { 3 } else { 8 };
    let n = match r.below(6) {
        0 => 0,
        1 => 1,
        _ => r.below(MAX_PAIRS + 1),
    };
    let mut v = vec![r.below(k), r.below(k), r.next_u32(), n];
    for _ in 0..n {
        v.push(r.below(k) | (r.below(k) << 16));
    }
    v
}

fn toggle_call(i: &Inputs) -> Call {
    let g = i[2];
    let mut c = Call::regconv(TOGGLE, &regconv(TOGGLE), &[SCRATCH + EDGES as u32, SCRATCH, i[0], i[1]], g);
    // the caller pushes b as a whole register: garbage in the upper half
    c.stack[0] = (g << 16) | (i[1] & 0xFFFF);
    c.scratch.fill(FILL);
    c.scratch[..2].copy_from_slice(&(i[3] as u16).to_le_bytes());
    for (k, w) in i[4..].iter().enumerate() {
        c.scratch[EDGES + 4 * k..EDGES + 4 * k + 4].copy_from_slice(&w.to_le_bytes());
    }
    c
}

pub fn toggle_edge() -> Spec {
    Spec {
        preserve: sa_core::abi::PRESERVE_ECX | sa_core::abi::PRESERVE_EDX,
        generate: Arc::new(gen_toggle),
        edges: Arc::new(|| {
            vec![
                vec![1, 2, 0, 0],
                vec![1, 2, 0xFFFF_FFFF, 1, 1 | (2 << 16)],
                vec![1, 2, 0x1234_5678, 1, 2 | (1 << 16)],
                vec![1, 2, 0, 2, 2 | (1 << 16), 3 | (4 << 16)],
                vec![3, 4, 0, 2, 2 | (1 << 16), 3 | (4 << 16)],
                vec![5, 5, 0, 1, 5 | (5 << 16)],
            ]
        }),
        call: Arc::new(toggle_call),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            let (edges, count) = (host_ptr(&mut m, EDGES), host_ptr(&mut m, 0));
            sa_render::harness5::toggle_edge(edges, count, i[0] as u16, i[1] as u16);
            Expect::Returns { ret: ExpRet::Void, scratch: m }
        }),
        ..base("ToggleEdge", "FUN_0070fa70", TOGGLE)
    }
}

pub fn all() -> Vec<Spec> {
    vec![char_position(), toggle_edge()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_follow_the_hook_specs() {
        let c = char_call(&vec![0x41, 0xDEAD_BEEF, 0]);
        assert_eq!((c.eax, c.stack.clone()), (Some(0xDEAD_BE41), vec![SCRATCH, SCRATCH + 4]));
        let c = toggle_call(&vec![3, 4, 0xABCD_0000, 0]);
        assert_eq!((c.ecx, c.edx, c.edi, c.stack.clone()), (SCRATCH + 0x10, SCRATCH, 0xABCD_0003, vec![4]));
        assert_eq!(c.conv, Conv::Cdecl);
        // every generated case stays inside scratch
        for k in 0..2_000 {
            let i = gen_toggle(&mut Rng::for_case(5, k));
            let c = toggle_call(&i);
            let _ = (toggle_edge().rust)(&i, &c.scratch);
        }
    }
}
