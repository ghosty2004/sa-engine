//! Builder for L1 specs of functions that work on structs in memory (MATH-2): the inputs are laid out
//! as regions in scratch memory (`this` and pointer arguments point at them), every float word of a
//! region is a generated float input, the other words are generated raw words, and the Rust side
//! computes the expected scratch memory from the same initial bytes, field by field.
//!
//! Input vector layout (`Inputs`): all float slots first (scalar float arguments, then the float words
//! of the regions in order), then all raw slots (scalar integer arguments, raw region words, the alias
//! decision, extra raw inputs). The runner shrinks and SNaN-routes the leading float slots.

use crate::call::*;
use crate::generate::{self, Rng};
use crate::specs::*;
use sa_math::matrix::{CMatrix, RwMatrix};
use sa_math::vector::{CVector, CVector2D};
use sa_math::x87::X87;
use std::sync::Arc;

/// One dword of a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W {
    /// generated float
    F,
    /// generated raw word
    R,
    /// constant
    C(u32),
    /// address of region `r`
    P(usize),
    /// address of region `r` when the region's raw slot is odd, else 0 (`m_pAttachMatrix`)
    PN(usize),
    /// the owns dword: 0 whenever the preceding `PN` word is non-zero (no RwMatrixDestroy path), else
    /// a generated raw word
    OwnNoDestroy,
}

/// Generators for the float words of a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Any,
    /// a CMatrix/RwMatrix: sometimes a rotation with a realistic position
    Matrix,
    /// a quaternion: sometimes unit length
    Quat,
}

#[derive(Clone, Debug)]
pub struct Region {
    pub words: Vec<W>,
    pub kind: Kind,
}

/// A scalar argument.
#[derive(Clone, Copy, Debug)]
pub enum A {
    /// address of a region
    P(usize),
    /// float input
    F,
    /// raw input with a generator
    I(fn(&mut Rng) -> u32),
    /// constant
    K(u32),
}

pub fn vec3() -> Region {
    Region {
        words: vec![W::F; 3],
        kind: Kind::Any,
    }
}
pub fn vec2() -> Region {
    Region {
        words: vec![W::F; 2],
        kind: Kind::Any,
    }
}
pub fn floats(n: usize) -> Region {
    Region {
        words: vec![W::F; n],
        kind: Kind::Any,
    }
}
/// One output float (initial value random bits).
pub fn out1() -> Region {
    Region {
        words: vec![W::R],
        kind: Kind::Any,
    }
}
pub fn raw(n: usize) -> Region {
    Region {
        words: vec![W::R; n],
        kind: Kind::Any,
    }
}
pub fn quat() -> Region {
    Region {
        words: vec![W::F; 4],
        kind: Kind::Quat,
    }
}
fn mat_words() -> Vec<W> {
    let mut w = Vec::new();
    for _ in 0..4 {
        w.extend([W::F, W::F, W::F, W::R]);
    }
    w
}
pub fn rwmat() -> Region {
    Region {
        words: mat_words(),
        kind: Kind::Matrix,
    }
}
/// A CMatrix not attached (attach 0, owns random).
pub fn mat() -> Region {
    let mut w = mat_words();
    w.extend([W::C(0), W::R]);
    Region {
        words: w,
        kind: Kind::Matrix,
    }
}
/// A CMatrix attached (sometimes) to region `rw`, never owning it when attached.
pub fn mat_att(rw: usize) -> Region {
    let mut w = mat_words();
    w.extend([W::PN(rw), W::OwnNoDestroy]);
    Region {
        words: w,
        kind: Kind::Matrix,
    }
}
/// A CMatrix always attached to `rw` (owns random: not followed).
pub fn mat_att_always(rw: usize) -> Region {
    let mut w = mat_words();
    w.extend([W::P(rw), W::R]);
    Region {
        words: w,
        kind: Kind::Matrix,
    }
}

/// Where each input of a spec lives.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    /// region byte offsets in scratch
    pub off: Vec<usize>,
    pub len: usize,
    /// float slot of (scalar arg k) / raw slot
    pub arg_slot: Vec<Option<usize>>,
    /// per region, per word: input slot
    pub word_slot: Vec<Vec<Option<usize>>>,
    /// per region: raw slot of its PN decision
    pub pn_slot: Vec<Option<usize>>,
    pub alias_slot: Option<usize>,
    pub extra_slot: Vec<usize>,
    pub n_float: usize,
    pub n: usize,
}

/// The expected-state builder handed to the port closures.
pub struct Ctx<'a> {
    pub mem: [u8; SCRATCH_LEN],
    pub inp: &'a Inputs,
    pub lay: &'a Layout,
    pub alias: Option<(usize, usize)>,
    pub aliased: bool,
    pub args: &'a [A],
}

impl Ctx<'_> {
    pub fn addr(&self, r: usize) -> u32 {
        SCRATCH + self.lay.off[r] as u32
    }
    /// The region argument `k` points at (after aliasing).
    pub fn ar(&self, k: usize) -> usize {
        if self.aliased
            && let Some((ak, r)) = self.alias
            && ak == k
        {
            return r;
        }
        match self.args[k] {
            A::P(r) => r,
            _ => panic!("argument {k} is not a pointer"),
        }
    }
    pub fn f(&self, k: usize) -> f32 {
        f32::from_bits(self.inp[self.lay.arg_slot[k].unwrap()])
    }
    pub fn i(&self, k: usize) -> u32 {
        self.inp[self.lay.arg_slot[k].unwrap()]
    }
    pub fn extra(&self, k: usize) -> u32 {
        self.inp[self.lay.extra_slot[k]]
    }
    pub fn u(&self, r: usize, w: usize) -> u32 {
        let o = self.lay.off[r] + 4 * w;
        u32::from_le_bytes(self.mem[o..o + 4].try_into().unwrap())
    }
    pub fn set_u(&mut self, r: usize, w: usize, v: u32) {
        let o = self.lay.off[r] + 4 * w;
        self.mem[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    pub fn fl(&self, r: usize, w: usize) -> f32 {
        f32::from_bits(self.u(r, w))
    }
    pub fn set_fl(&mut self, r: usize, w: usize, v: f32) {
        self.set_u(r, w, v.to_bits())
    }
    pub fn byte(&self, r: usize, o: usize) -> u8 {
        self.mem[self.lay.off[r] + o]
    }
    pub fn set_byte(&mut self, r: usize, o: usize, v: u8) {
        let b = self.lay.off[r] + o;
        self.mem[b] = v;
    }
    pub fn words(&self, r: usize, n: usize) -> Vec<u32> {
        (0..n).map(|w| self.u(r, w)).collect()
    }
    pub fn set_words(&mut self, r: usize, ws: &[u32]) {
        for (w, v) in ws.iter().enumerate() {
            self.set_u(r, w, *v);
        }
    }
    pub fn vec(&self, r: usize) -> CVector {
        CVector::new(self.fl(r, 0), self.fl(r, 1), self.fl(r, 2))
    }
    pub fn set_vec(&mut self, r: usize, v: CVector) {
        self.set_fl(r, 0, v.x);
        self.set_fl(r, 1, v.y);
        self.set_fl(r, 2, v.z);
    }
    pub fn v2(&self, r: usize) -> CVector2D {
        CVector2D {
            x: self.fl(r, 0),
            y: self.fl(r, 1),
        }
    }
    pub fn set_v2(&mut self, r: usize, v: CVector2D) {
        self.set_fl(r, 0, v.x);
        self.set_fl(r, 1, v.y);
    }
    pub fn mat(&self, r: usize) -> CMatrix {
        CMatrix::from_words(&self.words(r, 18))
    }
    pub fn set_mat(&mut self, r: usize, m: &CMatrix) {
        self.set_words(r, &m.to_words());
    }
    pub fn rw(&self, r: usize) -> RwMatrix {
        RwMatrix::from_words(&self.words(r, 16))
    }
    pub fn set_rw(&mut self, r: usize, m: &RwMatrix) {
        self.set_words(r, &m.to_words());
    }
    /// The region whose address is stored in word `w` of region `r` (None for 0).
    pub fn ptr_region(&self, r: usize, w: usize) -> Option<usize> {
        let a = self.u(r, w);
        if a == 0 {
            return None;
        }
        self.lay.off.iter().position(|&o| SCRATCH + o as u32 == a)
    }
}

/// Repairs generated inputs (see [`B::fix`]).
pub type FixFn = Arc<dyn Fn(&mut Inputs, &Layout) + Send + Sync>;

pub type Port = Arc<dyn Fn(&mut Ctx) -> ExpRet + Send + Sync>;

pub struct B {
    pub key: &'static str,
    pub name: &'static str,
    pub addr: u32,
    pub conv: Conv,
    pub ret: RetKind,
    pub st0_exact: bool,
    pub route: Route,
    pub callees: &'static [u32],
    pub budget: u64,
    pub regions: Vec<Region>,
    pub args: Vec<A>,
    /// (argument index, region it points at instead, percent)
    pub alias: Option<(usize, usize, u32)>,
    pub extra: Vec<fn(&mut Rng) -> u32>,
    pub ignore: Vec<(usize, usize)>,
    pub patches: Vec<(u32, Vec<u8>)>,
    pub reads: Vec<(u32, u32)>,
    /// reject/repair generated inputs (e.g. away from paths that need state beyond L1)
    pub fix: Option<FixFn>,
    pub edge_cap: usize,
    /// see [`Spec::preserve`]
    pub preserve: u8,
}

pub fn b(key: &'static str, name: &'static str, addr: u32, conv: Conv, ret: RetKind) -> B {
    B {
        key,
        name,
        addr,
        conv,
        ret,
        st0_exact: true,
        route: Route::L1,
        callees: &[],
        budget: 20_000,
        regions: vec![],
        args: vec![],
        alias: None,
        extra: vec![],
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        fix: None,
        edge_cap: 60_000,
        preserve: 0,
    }
}

impl B {
    pub fn regions(mut self, r: Vec<Region>) -> B {
        self.regions = r;
        self
    }
    pub fn args(mut self, a: Vec<A>) -> B {
        self.args = a;
        self
    }
    pub fn route(mut self, r: Route) -> B {
        self.route = r;
        self
    }
    pub fn callees(mut self, c: &'static [u32]) -> B {
        self.callees = c;
        self
    }
    pub fn alias(mut self, arg: usize, region: usize, pct: u32) -> B {
        self.alias = Some((arg, region, pct));
        self
    }
    pub fn extra(mut self, g: fn(&mut Rng) -> u32) -> B {
        self.extra.push(g);
        self
    }
    /// Ignore the flags/padding words of a matrix region (uninitialised stack copied by the original).
    pub fn ignore_mat_pads(mut self, r: usize) -> B {
        for w in [3, 7, 11, 15] {
            self.ignore.push((r, 4 * w));
        }
        self
    }
    pub fn fix(mut self, f: impl Fn(&mut Inputs, &Layout) + Send + Sync + 'static) -> B {
        self.fix = Some(Arc::new(f));
        self
    }
    pub fn patch(mut self, p: Vec<(u32, Vec<u8>)>) -> B {
        self.patches.extend(p);
        self
    }
    pub fn reads(mut self, r: Vec<(u32, u32)>) -> B {
        self.reads.extend(r);
        self
    }
    pub fn budget(mut self, n: u64) -> B {
        self.budget = n;
        self
    }
    pub fn st0_exact(mut self, e: bool) -> B {
        self.st0_exact = e;
        self
    }
    /// Registers the original preserves for its callers (see [`Spec::preserve`]).
    pub fn preserve(mut self, regs: u8) -> B {
        self.preserve = regs;
        self
    }

    fn layout(&self) -> Layout {
        let mut l = Layout::default();
        let mut off = 0x10usize;
        for r in &self.regions {
            l.off.push(off);
            off += (4 * r.words.len() + 0x10 + 0xF) & !0xF;
        }
        l.len = off;
        let mut nf = 0;
        for a in &self.args {
            l.arg_slot.push(if matches!(a, A::F) {
                nf += 1;
                Some(nf - 1)
            } else {
                None
            });
        }
        for r in &self.regions {
            let mut ws = Vec::new();
            for w in &r.words {
                ws.push(if *w == W::F {
                    nf += 1;
                    Some(nf - 1)
                } else {
                    None
                });
            }
            l.word_slot.push(ws);
        }
        l.n_float = nf;
        let mut n = nf;
        for (k, a) in self.args.iter().enumerate() {
            if matches!(a, A::I(_)) {
                l.arg_slot[k] = Some(n);
                n += 1;
            }
        }
        for (ri, r) in self.regions.iter().enumerate() {
            let mut pn = None;
            for (wi, w) in r.words.iter().enumerate() {
                match w {
                    W::R | W::OwnNoDestroy => {
                        l.word_slot[ri][wi] = Some(n);
                        n += 1;
                    }
                    W::PN(_) => {
                        pn = Some(n);
                        n += 1;
                    }
                    _ => {}
                }
            }
            l.pn_slot.push(pn);
        }
        if self.alias.is_some() {
            l.alias_slot = Some(n);
            n += 1;
        }
        for _ in &self.extra {
            l.extra_slot.push(n);
            n += 1;
        }
        l.n = n;
        l
    }

    pub fn rust(self, port: impl Fn(&mut Ctx) -> ExpRet + Send + Sync + 'static) -> Spec {
        let lay = Arc::new(self.layout());
        let port: Port = Arc::new(port);
        let regions = Arc::new(self.regions.clone());
        let args = Arc::new(self.args.clone());
        let extra = self.extra.clone();
        let alias = self.alias;
        let fix = self.fix.clone();

        // raw slot generators, in slot order
        let mut raw_gen: Vec<fn(&mut Rng) -> u32> = vec![gen_raw; lay.n - lay.n_float];
        for (k, a) in self.args.iter().enumerate() {
            if let A::I(g) = a {
                raw_gen[lay.arg_slot[k].unwrap() - lay.n_float] = *g;
            }
        }
        for (ri, r) in self.regions.iter().enumerate() {
            for (wi, w) in r.words.iter().enumerate() {
                if *w == W::OwnNoDestroy {
                    raw_gen[lay.word_slot[ri][wi].unwrap() - lay.n_float] = gen_raw;
                }
            }
        }
        if let Some((_, _, pct)) = alias {
            let s = lay.alias_slot.unwrap() - lay.n_float;
            raw_gen[s] = match pct {
                10 => (|r| r.chance(10) as u32) as fn(&mut Rng) -> u32,
                20 => |r| r.chance(20) as u32,
                25 => |r| r.chance(25) as u32,
                _ => |r| r.chance(30) as u32,
            };
        }
        for (k, g) in extra.iter().enumerate() {
            raw_gen[lay.extra_slot[k] - lay.n_float] = *g;
        }
        let raw_gen = Arc::new(raw_gen);

        let finish = {
            let lay = lay.clone();
            let regions = regions.clone();
            let fix = fix.clone();
            move |v: &mut Inputs| {
                // the owns word is 0 whenever the attach pointer is set
                for (ri, r) in regions.iter().enumerate() {
                    for (wi, w) in r.words.iter().enumerate() {
                        if *w == W::OwnNoDestroy && lay.pn_slot[ri].is_some_and(|s| v[s] & 1 == 1) {
                            v[lay.word_slot[ri][wi].unwrap()] = 0;
                        }
                    }
                }
                if let Some(f) = &fix {
                    f(v, &lay);
                }
            }
        };

        let generate: GenFn = {
            let lay = lay.clone();
            let regions = regions.clone();
            let raw_gen = raw_gen.clone();
            let finish = finish.clone();
            Arc::new(move |r: &mut Rng| {
                let mut v: Inputs = generate::f32_tuple(r, lay.n_float)
                    .into_iter()
                    .map(f32::to_bits)
                    .collect();
                for (ri, reg) in regions.iter().enumerate() {
                    let slots: Vec<usize> = lay.word_slot[ri]
                        .iter()
                        .zip(&reg.words)
                        .filter(|(_, w)| **w == W::F)
                        .map(|(s, _)| s.unwrap())
                        .collect();
                    shape(r, reg.kind, &slots, &mut v);
                }
                for g in raw_gen.iter() {
                    v.push(g(r));
                }
                finish(&mut v);
                v
            })
        };

        let edges: EdgesFn = {
            let lay = lay.clone();
            let raw_gen = raw_gen.clone();
            let cap = self.edge_cap;
            let finish = finish.clone();
            Arc::new(move || {
                let mut r = Rng::new(0xED6E_0002);
                let tuples = if lay.n_float == 0 {
                    vec![vec![]; 2000]
                } else {
                    generate::edge_tuples(lay.n_float, cap)
                };
                tuples
                    .into_iter()
                    .map(|t| {
                        let mut v: Inputs = t.into_iter().map(f32::to_bits).collect();
                        for g in raw_gen.iter() {
                            v.push(g(&mut r));
                        }
                        finish(&mut v);
                        v
                    })
                    .collect()
            })
        };

        let call: CallFn = {
            let lay = lay.clone();
            let regions = regions.clone();
            let args = args.clone();
            let (addr, conv) = (self.addr, self.conv);
            Arc::new(move |v: &Inputs| {
                assert!(lay.len <= SCRATCH_LEN, "layout larger than scratch");
                let mut scratch = [FILL; SCRATCH_LEN];
                for (ri, reg) in regions.iter().enumerate() {
                    for (wi, w) in reg.words.iter().enumerate() {
                        let val = match *w {
                            W::F | W::R | W::OwnNoDestroy => v[lay.word_slot[ri][wi].unwrap()],
                            W::C(c) => c,
                            W::P(t) => SCRATCH + lay.off[t] as u32,
                            W::PN(t) => {
                                if v[lay.pn_slot[ri].unwrap()] & 1 == 1 {
                                    SCRATCH + lay.off[t] as u32
                                } else {
                                    0
                                }
                            }
                        };
                        let o = lay.off[ri] + 4 * wi;
                        scratch[o..o + 4].copy_from_slice(&val.to_le_bytes());
                    }
                }
                let aliased = lay.alias_slot.is_some_and(|s| v[s] == 1);
                let a: Vec<Arg> = args
                    .iter()
                    .enumerate()
                    .map(|(k, a)| match *a {
                        A::P(r) => {
                            let r = match alias {
                                Some((ak, t, _)) if aliased && ak == k => t,
                                _ => r,
                            };
                            Arg::Ptr(SCRATCH + lay.off[r] as u32)
                        }
                        A::F => Arg::F32(f32::from_bits(v[lay.arg_slot[k].unwrap()])),
                        A::I(_) => Arg::I32(v[lay.arg_slot[k].unwrap()]),
                        A::K(c) => Arg::I32(c),
                    })
                    .collect();
                let mut c = Call::new(addr, conv, &a);
                c.scratch = scratch;
                c
            })
        };

        let rust: RustFn = {
            let lay = lay.clone();
            let args = args.clone();
            Arc::new(move |v: &Inputs, s: &[u8; SCRATCH_LEN]| {
                let mut ctx = Ctx {
                    mem: *s,
                    inp: v,
                    lay: &lay,
                    alias: alias.map(|(a, r, _)| (a, r)),
                    aliased: lay.alias_slot.is_some_and(|sl| v[sl] == 1),
                    args: &args,
                };
                let ret = port(&mut ctx);
                Expect::Returns {
                    ret,
                    scratch: ctx.mem,
                }
            })
        };

        let ignore = self.ignore.iter().map(|&(r, o)| (lay.off[r] + o, 4)).collect();
        Spec {
            key: self.key,
            name: self.name,
            addr: self.addr,
            ret: self.ret,
            st0_exact: self.st0_exact,
            route: self.route,
            callees: self.callees,
            budget: self.budget,
            floats: lay.n_float,
            generate,
            edges,
            call,
            rust,
            ignore,
            patches: self.patches,
            reads: self.reads,
            preserve: self.preserve,
        }
    }
}

fn gen_raw(r: &mut Rng) -> u32 {
    match r.below(10) {
        0..=3 => 0,
        4..=5 => r.below(16),
        6 => 1,
        _ => r.next_u32(),
    }
}

/// Region-specific shapes for the generated floats (realistic values next to the generic mix).
fn shape(r: &mut Rng, kind: Kind, slots: &[usize], v: &mut Inputs) {
    match kind {
        Kind::Any => {}
        Kind::Matrix if slots.len() >= 9 && r.chance(35) => {
            // a rotation (f64, rounded) and a position
            let (a, b, c) = (
                r.uniform(3.2) as f64,
                r.uniform(3.2) as f64,
                r.uniform(3.2) as f64,
            );
            let (sa, ca, sb, cb, sc, cc) = (a.sin(), a.cos(), b.sin(), b.cos(), c.sin(), c.cos());
            let m = [
                cb * cc,
                cb * sc,
                -sb,
                sa * sb * cc - ca * sc,
                sa * sb * sc + ca * cc,
                sa * cb,
                ca * sb * cc + sa * sc,
                ca * sb * sc - sa * cc,
                ca * cb,
            ];
            let noise = r.chance(30);
            for (k, s) in slots.iter().take(9).enumerate() {
                let mut x = m[k] as f32;
                if noise {
                    x += r.uniform(0.01);
                }
                v[*s] = x.to_bits();
            }
            if slots.len() >= 12 && r.chance(70) {
                for s in &slots[9..12] {
                    v[*s] = r.uniform(3000.0).to_bits();
                }
            }
        }
        Kind::Quat if slots.len() == 4 && r.chance(35) => {
            let q: Vec<f64> = (0..4).map(|_| r.uniform(1.0) as f64).collect();
            let n = q.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-9);
            for (k, s) in slots.iter().enumerate() {
                v[*s] = ((q[k] / n) as f32).to_bits();
            }
        }
        _ => {}
    }
}

pub fn st0(v: X87) -> ExpRet {
    ExpRet::St0 {
        f32: v.to_f32().to_bits(),
        ext: v.to_ext(),
    }
}

// ------------------------------------------------------------------ generators for raw arguments

pub fn g_small_int(r: &mut Rng) -> u32 {
    match r.below(10) {
        0..=5 => (r.below(2001) as i32 - 1000) as u32,
        6..=7 => r.below(32) as i32 as u32,
        _ => r.next_u32(),
    }
}

pub fn g_rand(r: &mut Rng) -> u32 {
    match r.below(10) {
        0..=7 => r.below(0x8000),
        8 => r.pick(&[0, 1, 0x7FFF, 0x4000, 0x3FFF, 0xFFFF, 0x8000]),
        _ => r.next_u32(),
    }
}

pub fn g_byte(r: &mut Rng) -> u32 {
    r.next_u32()
}

pub fn g_bool(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => 0,
        1 => 1,
        2 => r.below(256),
        _ => r.next_u32(),
    }
}

pub fn g_euler_flags(r: &mut Rng) -> u32 {
    if r.chance(90) { r.below(32) } else { r.next_u32() }
}
