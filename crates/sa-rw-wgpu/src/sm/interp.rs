//! Reference CPU interpreter of decoded shaders (vs_1_1..vs_2_x, ps_1_1..ps_2_x).
//!
//! This is the executable specification the WGSL translation ([`super::wgsl`]) is tested against,
//! so it follows the D3D9 reference semantics literally, with a few fixed decisions:
//!
//! - scalar-source instructions (`rcp`, `rsq`, `exp`, `log`, `expp`, `logp`, `pow`, `sincos`) and
//!   the `ifc`/`breakc` comparisons read lane w of the swizzled, modified source (the assembler
//!   encodes `r0.x` as the replicate swizzle `xxxx`);
//! - vs_1_1 `mov a0, x` floors, `mova` rounds half away from zero;
//! - ps_1_x constant reads are clamped to [-1, 1];
//! - `oFog` defaults to 1.0, `oPts` to 0.0 (both report lane x);
//! - ps_1_4 `texcrd` does not clamp, `phase` is a no-op, `texldb` ignores the bias.
//!
//! Not implemented (they return `Err` naming the instruction): the ps_1_x texture-addressing
//! instructions (`texbem`, `texbeml`, `texreg2*`, `texm3x*`, `texdp3*`, `texdepth`), `bem`,
//! `dsx`/`dsy`, `texldd`, `texldl` and SM3 relative output addressing.

use std::collections::HashMap;

use super::parse::mnemonic;
use super::{Dst, Instr, Reg, Shader, Src, Version, op, reg, srcmod};

/// Constant registers as the device holds them (`SetVertexShaderConstantF/I/B` or the pixel
/// equivalents); `def`/`defi`/`defb` in the shader override them.
#[derive(Debug, Clone, Copy)]
pub struct Consts<'a> {
    pub f: &'a [[f32; 4]],
    pub i: &'a [[i32; 4]],
    pub b: &'a [bool],
}

/// Vertex shader outputs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VsOut {
    pub pos: [f32; 4],
    /// `oD0`, `oD1` (not saturated here).
    pub d: [[f32; 4]; 2],
    pub t: [[f32; 4]; 8],
    pub fog: f32,
    pub psize: f32,
}

/// Pixel shader inputs (colours and texture coordinates as interpolated).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PsIn {
    pub d: [[f32; 4]; 2],
    pub t: [[f32; 4]; 8],
}

/// Pixel shader outputs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PsOut {
    pub color: [f32; 4],
    pub depth: Option<f32>,
}

type V4 = [f32; 4];

/// Number of temporary registers (`r0..r31`).
pub const TEMP_COUNT: usize = 32;
/// Instructions executed before the interpreter gives up (runaway loops).
pub const MAX_STEPS: u64 = 1 << 22;
/// Nested `call` depth limit.
pub const MAX_CALL_DEPTH: usize = 16;

/// Complete register state of one invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub r: [V4; TEMP_COUNT],
    /// Vertex inputs `v0..v15`; in a pixel shader `v0`/`v1` are the colours.
    pub v: [V4; 16],
    /// Pixel shader texture registers `t0..t7`.
    pub t: [V4; 8],
    pub a0: [i32; 4],
    pub al: i32,
    pub p: [bool; 4],
    pub pos: V4,
    pub fog: V4,
    pub pts: V4,
    pub od: [V4; 2],
    pub ot: [V4; 8],
    pub oc: [V4; 4],
    pub depth: V4,
    pub depth_written: bool,
}

impl Default for State {
    fn default() -> State {
        State {
            r: [[0.0; 4]; TEMP_COUNT],
            v: [[0.0; 4]; 16],
            t: [[0.0; 4]; 8],
            a0: [0; 4],
            al: 0,
            p: [false; 4],
            pos: [0.0; 4],
            fog: [1.0; 4],
            pts: [0.0; 4],
            od: [[0.0; 4]; 2],
            ot: [[0.0; 4]; 8],
            oc: [[0.0; 4]; 4],
            depth: [0.0; 4],
            depth_written: false,
        }
    }
}

/// Runs a vertex shader on one vertex (`v` = input registers v0..v15).
pub fn run_vs(sh: &Shader, c: &Consts<'_>, v: &[[f32; 4]; 16]) -> Result<VsOut, String> {
    let st = vs_state(sh, c, v)?;
    Ok(VsOut {
        pos: st.pos,
        d: st.od,
        t: st.ot,
        fog: st.fog[0],
        psize: st.pts[0],
    })
}

/// Runs a vertex shader and returns its final register state.
pub fn vs_state(sh: &Shader, c: &Consts<'_>, v: &[[f32; 4]; 16]) -> Result<State, String> {
    if sh.version.pixel {
        return Err(format!("run_vs on a pixel shader ({})", sh.version));
    }
    let m = Machine::new(sh, c, None);
    let mut st = State {
        v: *v,
        ..State::default()
    };
    if m.run(sh, &mut st)? {
        return Err("texkill in a vertex shader".into());
    }
    Ok(st)
}

/// Runs a pixel shader on one pixel; `sample(stage, coord)` samples texture stage/sampler
/// `stage` at `coord` (already projected). `Ok(None)` = the pixel was killed (`texkill`).
pub fn run_ps(
    sh: &Shader,
    c: &Consts<'_>,
    inp: &PsIn,
    sample: &dyn Fn(u32, [f32; 4]) -> [f32; 4],
) -> Result<Option<PsOut>, String> {
    let Some(st) = ps_state(sh, c, inp, sample)? else {
        return Ok(None);
    };
    let color = if sh.version.major < 2 { st.r[0] } else { st.oc[0] };
    Ok(Some(PsOut {
        color,
        depth: st.depth_written.then_some(st.depth[0]),
    }))
}

/// Runs a pixel shader and returns its final register state (`None` = killed).
pub fn ps_state(
    sh: &Shader,
    c: &Consts<'_>,
    inp: &PsIn,
    sample: &dyn Fn(u32, [f32; 4]) -> [f32; 4],
) -> Result<Option<State>, String> {
    if !sh.version.pixel {
        return Err(format!("run_ps on a vertex shader ({})", sh.version));
    }
    let m = Machine::new(sh, c, Some(sample));
    let mut st = State {
        t: inp.t,
        ..State::default()
    };
    st.v[0] = inp.d[0];
    st.v[1] = inp.d[1];
    if m.run(sh, &mut st)? {
        return Ok(None);
    }
    Ok(Some(st))
}

// ------------------------------------------------------------------------- helpers ---------

fn map1(a: V4, f: impl Fn(f32) -> f32) -> V4 {
    a.map(f)
}

fn map2(a: V4, b: V4, f: impl Fn(f32, f32) -> f32) -> V4 {
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]
}

fn map3(a: V4, b: V4, c: V4, f: impl Fn(f32, f32, f32) -> f32) -> V4 {
    [
        f(a[0], b[0], c[0]),
        f(a[1], b[1], c[1]),
        f(a[2], b[2], c[2]),
        f(a[3], b[3], c[3]),
    ]
}

fn dot(a: V4, b: V4, n: usize) -> f32 {
    (0..n).map(|k| a[k] * b[k]).sum()
}

/// Comparison of `ifc`/`breakc`/`setp` (instruction control 1 gt, 2 eq, 3 ge, 4 lt, 5 ne, 6 le).
fn compare(control: u8, a: f32, b: f32) -> Result<bool, String> {
    Ok(match control {
        1 => a > b,
        2 => a == b,
        3 => a >= b,
        4 => a < b,
        5 => a != b,
        6 => a <= b,
        c => return Err(format!("bad comparison {c}")),
    })
}

fn rcp(x: f32) -> f32 {
    if x == 1.0 {
        1.0
    } else if x == 0.0 {
        f32::INFINITY
    } else {
        1.0 / x
    }
}

fn rsq(x: f32) -> f32 {
    if x == 0.0 {
        f32::INFINITY
    } else if x == 1.0 {
        1.0
    } else {
        1.0 / x.abs().sqrt()
    }
}

fn log2_abs(x: f32) -> f32 {
    if x == 0.0 {
        f32::NEG_INFINITY
    } else {
        x.abs().log2()
    }
}

/// What one instruction does to the state (computed against a read-only state so that coissued
/// pairs both see the state from before the pair).
enum Eff {
    Nothing,
    Write {
        dst: Dst,
        val: V4,
        lanes: u8,
        round: bool,
    },
    Kill,
}

struct LoopFrame {
    /// First instruction of the body.
    start: usize,
    /// Index of the matching `endrep`/`endloop`.
    end: usize,
    remaining: u32,
    is_loop: bool,
    step: i32,
    prev_al: i32,
}

/// Matching structure: `jump[i]` for `if`/`ifc` = its `else` or `endif`, for `else` = its
/// `endif`, for `rep`/`loop` = its end, for `endrep`/`endloop` = its start; label positions.
fn structure(ins: &[Instr]) -> Result<(Vec<usize>, HashMap<u32, usize>), String> {
    let mut jump = vec![usize::MAX; ins.len()];
    let mut labels = HashMap::new();
    let mut stack: Vec<usize> = Vec::new();
    for (i, x) in ins.iter().enumerate() {
        match x.opcode {
            op::IF | op::IFC | op::REP | op::LOOP => stack.push(i),
            op::ELSE => {
                let Some(&top) = stack.last() else {
                    return Err(format!("else at {i} without if"));
                };
                if !matches!(ins[top].opcode, op::IF | op::IFC) {
                    return Err(format!("else at {i} without if"));
                }
                jump[top] = i;
                *stack.last_mut().expect("non-empty") = i;
            }
            op::ENDIF => {
                let top = stack.pop().ok_or_else(|| format!("endif at {i} without if"))?;
                if !matches!(ins[top].opcode, op::IF | op::IFC | op::ELSE) {
                    return Err(format!("endif at {i} closes a loop"));
                }
                jump[top] = i;
            }
            op::ENDREP | op::ENDLOOP => {
                let top = stack
                    .pop()
                    .ok_or_else(|| format!("loop end at {i} without start"))?;
                let want = if x.opcode == op::ENDREP { op::REP } else { op::LOOP };
                if ins[top].opcode != want {
                    return Err(format!("loop end at {i} does not match instruction {top}"));
                }
                jump[top] = i;
                jump[i] = top;
            }
            op::LABEL => {
                let l = x
                    .src
                    .first()
                    .ok_or_else(|| format!("label at {i} without register"))?;
                labels.insert(l.reg.index, i);
            }
            _ => {}
        }
    }
    if let Some(&open) = stack.last() {
        return Err(format!("unterminated block starting at {open}"));
    }
    Ok((jump, labels))
}

// ------------------------------------------------------------------------- machine ---------

struct Machine<'m> {
    ver: Version,
    c: &'m Consts<'m>,
    fdef: HashMap<u32, V4>,
    idef: HashMap<u32, [i32; 4]>,
    bdef: HashMap<u32, bool>,
    sample: Option<&'m dyn Fn(u32, V4) -> V4>,
}

impl<'m> Machine<'m> {
    fn new(sh: &Shader, c: &'m Consts<'m>, sample: Option<&'m dyn Fn(u32, V4) -> V4>) -> Machine<'m> {
        let mut m = Machine {
            ver: sh.version,
            c,
            fdef: HashMap::new(),
            idef: HashMap::new(),
            bdef: HashMap::new(),
            sample,
        };
        for i in &sh.instrs {
            let Some(d) = i.dst else { continue };
            match i.opcode {
                op::DEF => {
                    m.fdef.insert(d.reg.index, i.imm.map(f32::from_bits));
                }
                op::DEFI => {
                    m.idef.insert(d.reg.index, i.imm.map(|b| b as i32));
                }
                op::DEFB => {
                    m.bdef.insert(d.reg.index, i.imm[0] != 0);
                }
                _ => {}
            }
        }
        m
    }

    fn name(&self, i: &Instr) -> &'static str {
        mnemonic(i.opcode, self.ver, i.control)
    }

    fn ps1(&self) -> bool {
        self.ver.pixel && self.ver.major < 2
    }

    fn fconst(&self, idx: i64) -> V4 {
        let Ok(u) = u32::try_from(idx) else {
            return [0.0; 4];
        };
        let v = self
            .fdef
            .get(&u)
            .copied()
            .or_else(|| self.c.f.get(u as usize).copied())
            .unwrap_or([0.0; 4]);
        if self.ps1() {
            v.map(|x| x.clamp(-1.0, 1.0))
        } else {
            v
        }
    }

    fn iconst(&self, idx: u32) -> [i32; 4] {
        self.idef
            .get(&idx)
            .copied()
            .or_else(|| self.c.i.get(idx as usize).copied())
            .unwrap_or([0; 4])
    }

    fn bconst(&self, idx: u32) -> bool {
        self.bdef
            .get(&idx)
            .copied()
            .or_else(|| self.c.b.get(idx as usize).copied())
            .unwrap_or(false)
    }

    /// Raw value of register `r` at (possibly relative-adjusted) index `idx`.
    fn reg_value(&self, st: &State, r: Reg, idx: i64) -> Result<V4, String> {
        let pick = |arr: &[V4]| {
            usize::try_from(idx)
                .ok()
                .and_then(|i| arr.get(i))
                .copied()
                .ok_or_else(|| format!("register {} index {idx} out of range", r.ty))
        };
        let pixel = self.ver.pixel;
        match r.ty {
            reg::TEMP => pick(&st.r),
            reg::INPUT if pixel => pick(&st.v[..2]),
            reg::INPUT => pick(&st.v),
            reg::CONST => Ok(self.fconst(idx)),
            reg::CONST2 => Ok(self.fconst(idx + 2048)),
            reg::CONST3 => Ok(self.fconst(idx + 4096)),
            reg::CONST4 => Ok(self.fconst(idx + 6144)),
            reg::TEXTURE if pixel => pick(&st.t),
            reg::ADDR => Ok(st.a0.map(|x| x as f32)),
            reg::CONSTINT => Ok(u32::try_from(idx)
                .map(|i| self.iconst(i).map(|x| x as f32))
                .unwrap_or([0.0; 4])),
            reg::CONSTBOOL => {
                let b = u32::try_from(idx).map(|i| self.bconst(i)).unwrap_or(false);
                Ok([if b { 1.0 } else { 0.0 }; 4])
            }
            reg::LOOP => Ok([st.al as f32; 4]),
            reg::PREDICATE => Ok(st.p.map(|b| if b { 1.0 } else { 0.0 })),
            reg::RASTOUT if !pixel => pick(&[st.pos, st.fog, st.pts]),
            reg::ATTROUT if !pixel => pick(&st.od),
            reg::TEXCRDOUT if !pixel => pick(&st.ot),
            reg::COLOROUT if pixel => pick(&st.oc),
            t => Err(format!("cannot read register type {t} in {}", self.ver)),
        }
    }

    fn rel_offset(&self, st: &State, s: &Src) -> Result<i64, String> {
        let Some(rel) = s.rel else { return Ok(0) };
        let c = (rel.comp & 3) as usize;
        match rel.reg.ty {
            reg::ADDR if !self.ver.pixel => Ok(st.a0[c] as i64),
            reg::LOOP => Ok(st.al as i64),
            t => Err(format!("relative addressing through register type {t}")),
        }
    }

    /// Source value: register (index + `row` + relative offset) → swizzle → modifier.
    fn read(&self, st: &State, s: &Src, row: u32) -> Result<V4, String> {
        let idx = s.reg.index as i64 + row as i64 + self.rel_offset(st, s)?;
        let raw = self.reg_value(st, s.reg, idx)?;
        let x = s.swizzle.map(|c| raw[(c & 3) as usize]);
        Ok(match s.modifier {
            srcmod::NONE => x,
            srcmod::NEG => map1(x, |a| -a),
            srcmod::BIAS => map1(x, |a| a - 0.5),
            srcmod::BIASNEG => map1(x, |a| -(a - 0.5)),
            srcmod::SIGN => map1(x, |a| 2.0 * a - 1.0),
            srcmod::SIGNNEG => map1(x, |a| -(2.0 * a - 1.0)),
            srcmod::COMP => map1(x, |a| 1.0 - a),
            srcmod::X2 => map1(x, |a| 2.0 * a),
            srcmod::X2NEG => map1(x, |a| -2.0 * a),
            srcmod::DZ => map1(x, |a| a / x[2]),
            srcmod::DW => map1(x, |a| a / x[3]),
            srcmod::ABS => map1(x, f32::abs),
            srcmod::ABSNEG => map1(x, |a| -a.abs()),
            srcmod::NOT => map1(x, |a| if a == 0.0 { 1.0 } else { 0.0 }),
            m => return Err(format!("bad source modifier {m}")),
        })
    }

    /// Boolean condition of `if`/`callnz`/`breakp`: a `b` constant or a predicate lane, `!` inverts.
    fn cond(&self, st: &State, s: &Src) -> Result<bool, String> {
        let b = match s.reg.ty {
            reg::CONSTBOOL => self.bconst(s.reg.index),
            reg::PREDICATE => st.p[(s.swizzle[3] & 3) as usize],
            t => return Err(format!("register type {t} is not a boolean condition")),
        };
        Ok(b != (s.modifier == srcmod::NOT))
    }

    /// Lanes enabled by an instruction predicate (`(p0) …` / `(!p0) …`).
    fn pred_lanes(&self, st: &State, p: &Src) -> u8 {
        let not = p.modifier == srcmod::NOT;
        (0..4)
            .filter(|&k| st.p[(p.swizzle[k] & 3) as usize] != not)
            .fold(0, |m, k| m | 1 << k)
    }

    fn compute(&self, st: &State, ins: &Instr) -> Result<Eff, String> {
        let name = self.name(ins);
        let src = |k: usize| -> Result<V4, String> {
            let s = ins
                .src
                .get(k)
                .ok_or_else(|| format!("{name}: missing source {k}"))?;
            self.read(st, s, 0)
        };
        let dst = || ins.dst.ok_or_else(|| format!("{name}: missing destination"));
        let pixel = self.ver.pixel;
        let sm1 = self.ver.major < 2;
        let ps14 = pixel && sm1 && self.ver.minor >= 4;
        let splat = |x: f32| [x; 4];
        let mut round = false;
        let (val, lanes): (V4, u8) = match ins.opcode {
            op::NOP | op::DCL | op::DEF | op::DEFI | op::DEFB | op::PHASE => return Ok(Eff::Nothing),
            op::MOV => (src(0)?, 0xF),
            op::MOVA => {
                round = true;
                (src(0)?, 0xF)
            }
            op::ADD => (map2(src(0)?, src(1)?, |a, b| a + b), 0xF),
            op::SUB => (map2(src(0)?, src(1)?, |a, b| a - b), 0xF),
            op::MUL => (map2(src(0)?, src(1)?, |a, b| a * b), 0xF),
            op::MAD => (map3(src(0)?, src(1)?, src(2)?, |a, b, c| a * b + c), 0xF),
            op::MIN => (map2(src(0)?, src(1)?, f32::min), 0xF),
            op::MAX => (map2(src(0)?, src(1)?, f32::max), 0xF),
            op::SLT => (map2(src(0)?, src(1)?, |a, b| if a < b { 1.0 } else { 0.0 }), 0xF),
            op::SGE => (map2(src(0)?, src(1)?, |a, b| if a >= b { 1.0 } else { 0.0 }), 0xF),
            op::FRC => (map1(src(0)?, |a| a - a.floor()), 0xF),
            op::ABS => (map1(src(0)?, f32::abs), 0xF),
            op::SGN => (
                map1(src(0)?, |a| {
                    if a > 0.0 {
                        1.0
                    } else if a < 0.0 {
                        -1.0
                    } else {
                        0.0
                    }
                }),
                0xF,
            ),
            op::LRP => (map3(src(0)?, src(1)?, src(2)?, |a, b, c| a * (b - c) + c), 0xF),
            op::CND => (
                map3(src(0)?, src(1)?, src(2)?, |a, b, c| if a > 0.5 { b } else { c }),
                0xF,
            ),
            op::CMP => (
                map3(src(0)?, src(1)?, src(2)?, |a, b, c| if a >= 0.0 { b } else { c }),
                0xF,
            ),
            op::DP3 => (splat(dot(src(0)?, src(1)?, 3)), 0xF),
            op::DP4 => (splat(dot(src(0)?, src(1)?, 4)), 0xF),
            op::DP2ADD => {
                let (a, b, c) = (src(0)?, src(1)?, src(2)?);
                (splat(a[0] * b[0] + a[1] * b[1] + c[3]), 0xF)
            }
            op::RCP => (splat(rcp(src(0)?[3])), 0xF),
            op::RSQ => (splat(rsq(src(0)?[3])), 0xF),
            op::EXP => (splat(src(0)?[3].exp2()), 0xF),
            op::LOG => (splat(log2_abs(src(0)?[3])), 0xF),
            op::EXPP => {
                let x = src(0)?[3];
                if sm1 {
                    ([x.floor().exp2(), x - x.floor(), x.exp2(), 1.0], 0xF)
                } else {
                    (splat(x.exp2()), 0xF)
                }
            }
            op::LOGP => {
                let x = src(0)?[3];
                if sm1 {
                    let v = x.abs();
                    if v == 0.0 {
                        ([f32::NEG_INFINITY, 1.0, f32::NEG_INFINITY, 1.0], 0xF)
                    } else {
                        let bits = v.to_bits();
                        let e = ((bits >> 23) & 0xFF) as f32 - 127.0;
                        let m = f32::from_bits((bits & 0x7F_FFFF) | 0x3F80_0000);
                        ([e, m, v.log2(), 1.0], 0xF)
                    }
                } else {
                    (splat(log2_abs(x)), 0xF)
                }
            }
            op::POW => (splat(src(0)?[3].abs().powf(src(1)?[3])), 0xF),
            op::SINCOS => {
                let x = src(0)?[3];
                ([x.cos(), x.sin(), 0.0, 0.0], 0b0011)
            }
            op::LIT => {
                let s = src(0)?;
                let power = s[3].clamp(-127.9961, 127.9961);
                let mut d = [1.0, 0.0, 0.0, 1.0];
                if s[0] > 0.0 {
                    d[1] = s[0];
                    if s[1] > 0.0 {
                        d[2] = s[1].powf(power);
                    }
                }
                (d, 0xF)
            }
            op::DST => {
                let (a, b) = (src(0)?, src(1)?);
                ([1.0, a[1] * b[1], a[2], b[3]], 0xF)
            }
            op::NRM => {
                let s = src(0)?;
                let f = dot(s, s, 3);
                let f = if f == 0.0 { f32::INFINITY } else { 1.0 / f.sqrt() };
                (map1(s, |a| a * f), 0xF)
            }
            op::CRS => {
                let (a, b) = (src(0)?, src(1)?);
                (
                    [
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                        0.0,
                    ],
                    0b0111,
                )
            }
            op::M4X4 | op::M4X3 | op::M3X4 | op::M3X3 | op::M3X2 => {
                let (cols, rows) = match ins.opcode {
                    op::M4X4 => (4, 4),
                    op::M4X3 => (4, 3),
                    op::M3X4 => (3, 4),
                    op::M3X3 => (3, 3),
                    _ => (3, 2),
                };
                let a = src(0)?;
                let m = ins
                    .src
                    .get(1)
                    .ok_or_else(|| format!("{name}: missing source 1"))?;
                let mut d = [0.0; 4];
                for (k, out) in d.iter_mut().enumerate().take(rows) {
                    *out = dot(a, self.read(st, m, k as u32)?, cols);
                }
                (d, (1u8 << rows) - 1)
            }
            op::SETP => {
                let (a, b) = (src(0)?, src(1)?);
                let mut d = [0.0; 4];
                for k in 0..4 {
                    d[k] = if compare(ins.control, a[k], b[k])? {
                        1.0
                    } else {
                        0.0
                    };
                }
                (d, 0xF)
            }
            op::TEXKILL if pixel => {
                let d = dst()?;
                let v = self.reg_value(st, d.reg, d.reg.index as i64)?;
                let lanes = if sm1 { 0b0111 } else { d.mask };
                let kill = (0..4).any(|k| lanes >> k & 1 != 0 && v[k] < 0.0);
                return Ok(if kill { Eff::Kill } else { Eff::Nothing });
            }
            op::TEXCOORD if pixel && sm1 => {
                if ps14 {
                    // texcrd rD, src
                    (src(0)?, 0xF)
                } else {
                    // texcoord tN
                    let d = dst()?;
                    let t = self.reg_value(st, d.reg, d.reg.index as i64)?;
                    let s = |x: f32| x.clamp(0.0, 1.0);
                    ([s(t[0]), s(t[1]), s(t[2]), 1.0], 0xF)
                }
            }
            op::TEX if pixel => {
                let sample = self.sample.ok_or("texture sampling without a sampler callback")?;
                let d = dst()?;
                if sm1 && !ps14 {
                    // tex tN
                    let coord = self.reg_value(st, d.reg, d.reg.index as i64)?;
                    (sample(d.reg.index, coord), 0xF)
                } else if ps14 {
                    // texld rD, src (stage = D)
                    (sample(d.reg.index, src(0)?), 0xF)
                } else {
                    // texld / texldp / texldb rD, src, sN
                    let mut coord = src(0)?;
                    let s = ins.src.get(1).ok_or_else(|| format!("{name}: missing sampler"))?;
                    if ins.control == 1 {
                        let w = coord[3];
                        coord = map1(coord, |a| a / w);
                    }
                    (sample(s.reg.index, coord), 0xF)
                }
            }
            _ => {
                return Err(format!(
                    "instruction `{name}` (opcode {}) not implemented in {}",
                    ins.opcode, self.ver
                ));
            }
        };
        let mut lanes = lanes;
        if let Some(p) = &ins.predicate {
            lanes &= self.pred_lanes(st, p);
        }
        Ok(Eff::Write {
            dst: dst()?,
            val,
            lanes,
            round,
        })
    }

    fn apply(&self, st: &mut State, dst: &Dst, val: V4, lanes: u8, round: bool) -> Result<(), String> {
        if dst.rel.is_some() {
            return Err("relative destination addressing not implemented".into());
        }
        let scale = (dst.shift as f32).exp2();
        let mut out = map1(val, |a| if dst.shift == 0 { a } else { a * scale });
        if dst.saturate {
            out = map1(out, |a| a.clamp(0.0, 1.0));
        }
        let mask = dst.mask & lanes;
        let on = |k: usize| mask >> k & 1 != 0;
        let idx = dst.reg.index as usize;
        let pixel = self.ver.pixel;
        let oob = || {
            format!(
                "cannot write register type {} index {idx} in {}",
                dst.reg.ty, self.ver
            )
        };
        let target: &mut V4 = match dst.reg.ty {
            reg::TEMP => st.r.get_mut(idx).ok_or_else(oob)?,
            reg::ADDR if !pixel => {
                for k in (0..4).filter(|&k| on(k)) {
                    st.a0[k] = if round {
                        out[k].round() as i32
                    } else {
                        out[k].floor() as i32
                    };
                }
                return Ok(());
            }
            reg::PREDICATE => {
                for k in (0..4).filter(|&k| on(k)) {
                    st.p[k] = out[k] != 0.0;
                }
                return Ok(());
            }
            reg::TEXTURE if pixel => st.t.get_mut(idx).ok_or_else(oob)?,
            reg::RASTOUT if !pixel => match idx {
                0 => &mut st.pos,
                1 => &mut st.fog,
                2 => &mut st.pts,
                _ => return Err(oob()),
            },
            reg::ATTROUT if !pixel => st.od.get_mut(idx).ok_or_else(oob)?,
            reg::TEXCRDOUT if !pixel => st.ot.get_mut(idx).ok_or_else(oob)?,
            reg::COLOROUT if pixel => st.oc.get_mut(idx).ok_or_else(oob)?,
            reg::DEPTHOUT if pixel => {
                st.depth_written = true;
                &mut st.depth
            }
            _ => return Err(oob()),
        };
        for k in (0..4).filter(|&k| on(k)) {
            target[k] = out[k];
        }
        Ok(())
    }

    fn unwind(st: &mut State, loops: &mut Vec<LoopFrame>, depth: usize) {
        while loops.len() > depth {
            let f = loops.pop().expect("non-empty");
            if f.is_loop {
                st.al = f.prev_al;
            }
        }
    }

    /// Leaves the innermost loop; returns the instruction after its end.
    fn break_loop(st: &mut State, loops: &mut Vec<LoopFrame>) -> Result<usize, String> {
        let f = loops.pop().ok_or("break outside a loop")?;
        if f.is_loop {
            st.al = f.prev_al;
        }
        Ok(f.end + 1)
    }

    /// Executes the shader; returns whether the pixel was killed.
    fn run(&self, sh: &Shader, st: &mut State) -> Result<bool, String> {
        let ins = &sh.instrs;
        let (jump, labels) = structure(ins)?;
        let mut pc = 0usize;
        let mut calls: Vec<(usize, usize)> = Vec::new();
        let mut loops: Vec<LoopFrame> = Vec::new();
        let mut steps = 0u64;
        let src = |i: &Instr, k: usize| -> Result<Src, String> {
            i.src
                .get(k)
                .copied()
                .ok_or_else(|| format!("{}: missing source {k}", self.name(i)))
        };
        while pc < ins.len() {
            steps += 1;
            if steps > MAX_STEPS {
                return Err(format!("more than {MAX_STEPS} instructions executed"));
            }
            let i = &ins[pc];
            match i.opcode {
                // A label reached in sequence ends the routine like `ret`.
                op::RET | op::LABEL => match calls.pop() {
                    None => break,
                    Some((ret, depth)) => {
                        Self::unwind(st, &mut loops, depth);
                        pc = ret;
                    }
                },
                op::CALL | op::CALLNZ => {
                    let l = src(i, 0)?;
                    let take = i.opcode == op::CALL || self.cond(st, &src(i, 1)?)?;
                    if take {
                        let at = *labels
                            .get(&l.reg.index)
                            .ok_or_else(|| format!("call to missing label l{}", l.reg.index))?;
                        if calls.len() >= MAX_CALL_DEPTH {
                            return Err("call depth limit exceeded".into());
                        }
                        calls.push((pc + 1, loops.len()));
                        pc = at + 1;
                    } else {
                        pc += 1;
                    }
                }
                op::IF | op::IFC => {
                    let take = if i.opcode == op::IF {
                        self.cond(st, &src(i, 0)?)?
                    } else {
                        let a = self.read(st, &src(i, 0)?, 0)?[3];
                        let b = self.read(st, &src(i, 1)?, 0)?[3];
                        compare(i.control, a, b)?
                    };
                    pc = if take { pc + 1 } else { jump[pc] + 1 };
                }
                op::ELSE => pc = jump[pc] + 1,
                op::ENDIF => pc += 1,
                op::REP | op::LOOP => {
                    let ireg = src(i, if i.opcode == op::REP { 0 } else { 1 })?;
                    if ireg.reg.ty != reg::CONSTINT {
                        return Err(format!(
                            "{}: count register is not an integer constant",
                            self.name(i)
                        ));
                    }
                    let iv = self.iconst(ireg.reg.index);
                    let count = iv[0].clamp(0, 255) as u32;
                    if count == 0 {
                        pc = jump[pc] + 1;
                        continue;
                    }
                    let is_loop = i.opcode == op::LOOP;
                    loops.push(LoopFrame {
                        start: pc + 1,
                        end: jump[pc],
                        remaining: count,
                        is_loop,
                        step: iv[2],
                        prev_al: st.al,
                    });
                    if is_loop {
                        st.al = iv[1];
                    }
                    pc += 1;
                }
                op::ENDREP | op::ENDLOOP => {
                    let f = loops.last_mut().ok_or("loop end outside a loop")?;
                    f.remaining -= 1;
                    if f.remaining > 0 {
                        if f.is_loop {
                            st.al = st.al.wrapping_add(f.step);
                        }
                        pc = f.start;
                    } else {
                        let depth = loops.len() - 1;
                        Self::unwind(st, &mut loops, depth);
                        pc += 1;
                    }
                }
                op::BREAK => pc = Self::break_loop(st, &mut loops)?,
                op::BREAKC => {
                    let a = self.read(st, &src(i, 0)?, 0)?[3];
                    let b = self.read(st, &src(i, 1)?, 0)?[3];
                    if compare(i.control, a, b)? {
                        pc = Self::break_loop(st, &mut loops)?;
                    } else {
                        pc += 1;
                    }
                }
                op::BREAKP => {
                    if self.cond(st, &src(i, 0)?)? {
                        pc = Self::break_loop(st, &mut loops)?;
                    } else {
                        pc += 1;
                    }
                }
                _ => {
                    // This instruction and the coissued ones after it read the same state.
                    let mut end = pc + 1;
                    while end < ins.len() && ins[end].coissue {
                        end += 1;
                    }
                    let effs = ins[pc..end]
                        .iter()
                        .map(|x| self.compute(st, x))
                        .collect::<Result<Vec<_>, _>>()?;
                    for e in effs {
                        match e {
                            Eff::Nothing => {}
                            Eff::Kill => return Ok(true),
                            Eff::Write {
                                dst,
                                val,
                                lanes,
                                round,
                            } => self.apply(st, &dst, val, lanes, round)?,
                        }
                    }
                    pc = end;
                }
            }
        }
        Ok(false)
    }
}

// ------------------------------------------------------------------------- tests -----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sm::{Rel, Version};

    const INF: f32 = f32::INFINITY;

    fn dst(ty: u8, index: u32, mask: u8) -> Dst {
        Dst {
            reg: Reg { ty, index },
            mask,
            saturate: false,
            partial: false,
            centroid: false,
            shift: 0,
            rel: None,
        }
    }
    fn src(ty: u8, index: u32) -> Src {
        Src {
            reg: Reg { ty, index },
            swizzle: [0, 1, 2, 3],
            modifier: 0,
            rel: None,
        }
    }
    /// Swizzle from text ("wzyx"; one letter = replicate).
    fn sw(mut s: Src, t: &str) -> Src {
        let c: Vec<u8> = t.chars().map(|ch| "xyzw".find(ch).unwrap() as u8).collect();
        s.swizzle = [0, 1, 2, 3].map(|k| *c.get(k).unwrap_or(c.last().unwrap()));
        s
    }
    fn md(mut s: Src, m: u8) -> Src {
        s.modifier = m;
        s
    }
    fn r(n: u32) -> Src {
        src(reg::TEMP, n)
    }
    fn c(n: u32) -> Src {
        src(reg::CONST, n)
    }
    fn v(n: u32) -> Src {
        src(reg::INPUT, n)
    }
    fn rd(n: u32, mask: u8) -> Dst {
        dst(reg::TEMP, n, mask)
    }
    fn ins(opcode: u16, d: Dst, s: Vec<Src>) -> Instr {
        let mut i = Instr::new(opcode);
        i.dst = Some(d);
        i.src = s;
        i
    }
    fn flow(opcode: u16, s: Vec<Src>) -> Instr {
        let mut i = Instr::new(opcode);
        i.src = s;
        i
    }
    fn ctl(mut i: Instr, control: u8) -> Instr {
        i.control = control;
        i
    }
    fn def(n: u32, x: V4) -> Instr {
        let mut i = Instr::new(op::DEF);
        i.dst = Some(dst(reg::CONST, n, 0xF));
        i.imm = x.map(f32::to_bits);
        i
    }
    fn defi(n: u32, x: [i32; 4]) -> Instr {
        let mut i = Instr::new(op::DEFI);
        i.dst = Some(dst(reg::CONSTINT, n, 0xF));
        i.imm = x.map(|a| a as u32);
        i
    }
    fn shader(pixel: bool, major: u8, minor: u8, instrs: Vec<Instr>) -> Shader {
        Shader {
            version: Version { pixel, major, minor },
            instrs,
        }
    }
    fn consts(f: &[V4]) -> Consts<'_> {
        Consts { f, i: &[], b: &[] }
    }
    fn vs(sh: &Shader, f: &[V4], vin: &[V4]) -> State {
        let mut v = [[0.0; 4]; 16];
        v[..vin.len()].copy_from_slice(vin);
        vs_state(sh, &consts(f), &v).unwrap_or_else(|e| panic!("{e}\n{sh}"))
    }
    fn close(a: V4, b: V4) -> bool {
        (0..4).all(|k| a[k] == b[k] || (a[k] - b[k]).abs() <= 1e-5 * b[k].abs().max(1.0))
    }
    #[track_caller]
    fn eq(a: V4, b: V4) {
        assert!(close(a, b), "got {a:?}, want {b:?}");
    }
    fn sampler(stage: u32, c: V4) -> V4 {
        [stage as f32, c[0], c[1], c[2]]
    }

    #[test]
    fn swizzle_and_modifiers() {
        use srcmod::*;
        let cases: Vec<(Src, V4)> = vec![
            (md(sw(c(0), "wzyx"), NEG), [-8.0, -4.0, -2.0, -1.0]),
            (sw(c(0), "y"), [2.0; 4]),
            (sw(c(0), "xxyw"), [1.0, 1.0, 2.0, 8.0]),
            (md(c(1), BIAS), [-0.25, 0.0, 0.5, 1.5]),
            (md(c(1), BIASNEG), [0.25, 0.0, -0.5, -1.5]),
            (md(c(1), SIGN), [-0.5, 0.0, 1.0, 3.0]),
            (md(c(1), SIGNNEG), [0.5, 0.0, -1.0, -3.0]),
            (md(c(1), COMP), [0.75, 0.5, 0.0, -1.0]),
            (md(c(1), X2), [0.5, 1.0, 2.0, 4.0]),
            (md(c(1), X2NEG), [-0.5, -1.0, -2.0, -4.0]),
            (md(c(0), DZ), [0.25, 0.5, 1.0, 2.0]),
            (md(c(0), DW), [0.125, 0.25, 0.5, 1.0]),
            // modifier after swizzle: z of the swizzled value (2) divides
            (md(sw(c(0), "wzyx"), DZ), [4.0, 2.0, 1.0, 0.5]),
            (md(c(2), ABS), [1.0, 2.0, 3.0, 0.0]),
            (md(c(2), ABSNEG), [-1.0, -2.0, -3.0, -0.0]),
            (md(c(2), NOT), [0.0, 0.0, 0.0, 1.0]),
        ];
        let f = [
            [1.0, 2.0, 4.0, 8.0],
            [0.25, 0.5, 1.0, 2.0],
            [-1.0, 2.0, -3.0, 0.0],
        ];
        for (s, want) in cases {
            let sh = shader(false, 2, 0, vec![ins(op::MOV, rd(0, 0xF), vec![s])]);
            let st = vs(&sh, &f, &[]);
            assert!(close(st.r[0], want), "{sh}: got {:?}, want {want:?}", st.r[0]);
        }
    }

    #[test]
    fn mask_saturate_shift() {
        let mut add = ins(op::ADD, rd(0, 0b0101), vec![v(0), v(1)]);
        add.dst.as_mut().unwrap().saturate = true;
        add.dst.as_mut().unwrap().shift = 1;
        let mut mul = ins(op::MUL, rd(1, 0xF), vec![v(0), c(0)]);
        mul.dst.as_mut().unwrap().shift = -1;
        let sh = shader(true, 1, 1, vec![add, mul]);
        let inp = PsIn {
            d: [[0.2, 0.3, 0.4, 0.5], [0.1; 4]],
            ..PsIn::default()
        };
        let st = ps_state(&sh, &consts(&[[1.0; 4]]), &inp, &sampler)
            .unwrap()
            .unwrap();
        eq(st.r[0], [0.6, 0.0, 1.0, 0.0]);
        eq(st.r[1], [0.1, 0.15, 0.2, 0.25]);
        // saturate clamps below too
        let mut m = ins(op::MOV, rd(0, 0xF), vec![c(0)]);
        m.dst.as_mut().unwrap().saturate = true;
        let st = vs(&shader(false, 1, 1, vec![m]), &[[-2.0, 0.5, 3.0, 1.0]], &[]);
        eq(st.r[0], [0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn rcp_rsq_special_values() {
        let mut p = Vec::new();
        for (k, comp) in ["x", "y", "z", "w"].iter().enumerate() {
            p.push(ins(op::RCP, rd(0, 1 << k), vec![sw(c(0), comp)]));
            p.push(ins(op::RSQ, rd(1, 1 << k), vec![sw(c(1), comp)]));
        }
        // scalar source = lane w of the swizzled value
        p.push(ins(op::RCP, rd(2, 0xF), vec![c(0)]));
        let st = vs(
            &shader(false, 1, 1, p),
            &[[0.0, 1.0, 4.0, -0.5], [0.0, 1.0, 4.0, -4.0]],
            &[],
        );
        assert_eq!(st.r[0], [INF, 1.0, 0.25, -2.0]);
        assert_eq!(st.r[1], [INF, 1.0, 0.5, 0.5]);
        assert_eq!(st.r[2], [-2.0; 4]);
    }

    #[test]
    fn exp_log_family() {
        let f = [
            [0.0, 0.0, 0.0, 2.5],
            [0.0, 0.0, 0.0, -6.0],
            [0.0; 4],
            [8.0, 0.0, 0.0, 0.0],
        ];
        let prog = || {
            vec![
                ins(op::EXPP, rd(0, 0xF), vec![sw(c(0), "w")]),
                ins(op::LOGP, rd(1, 0xF), vec![sw(c(1), "w")]),
                ins(op::LOGP, rd(2, 0xF), vec![sw(c(2), "w")]),
                ins(op::EXP, rd(3, 0xF), vec![sw(c(0), "w")]),
                ins(op::LOG, rd(4, 0xF), vec![sw(c(3), "x")]),
                ins(op::LOG, rd(5, 0xF), vec![sw(c(2), "x")]),
            ]
        };
        let e = 2.5f32.exp2();
        let st = vs(&shader(false, 1, 1, prog()), &f, &[]);
        eq(st.r[0], [4.0, 0.5, e, 1.0]);
        eq(st.r[1], [2.0, 1.5, 6f32.log2(), 1.0]);
        assert_eq!(st.r[2], [-INF, 1.0, -INF, 1.0]);
        eq(st.r[3], [e; 4]);
        eq(st.r[4], [3.0; 4]);
        assert_eq!(st.r[5], [-INF; 4]);
        let st = vs(&shader(false, 2, 0, prog()), &f, &[]);
        eq(st.r[0], [e; 4]);
        eq(st.r[1], [6f32.log2(); 4]);
        assert_eq!(st.r[2], [-INF; 4]);
    }

    #[test]
    fn arithmetic_ops() {
        let f = [
            [0.5, -2.0, 3.0, 0.0],  // c0
            [1.0, 1.0, 1.0, 1.0],   // c1
            [2.0, 0.25, -1.0, 4.0], // c2
            [0.6, 0.5, -0.1, 0.0],  // c3
        ];
        let p = vec![
            ins(op::ADD, rd(0, 0xF), vec![c(0), c(2)]),
            ins(op::SUB, rd(1, 0xF), vec![c(0), c(2)]),
            ins(op::MAD, rd(2, 0xF), vec![c(0), c(2), c(1)]),
            ins(op::MIN, rd(3, 0xF), vec![c(0), c(2)]),
            ins(op::MAX, rd(4, 0xF), vec![c(0), c(2)]),
            ins(op::SLT, rd(5, 0xF), vec![c(0), c(2)]),
            ins(op::SGE, rd(6, 0xF), vec![c(0), md(c(0), srcmod::NEG)]),
            ins(op::FRC, rd(7, 0xF), vec![md(c(2), srcmod::X2NEG)]),
            ins(op::ABS, rd(8, 0xF), vec![c(0)]),
            ins(op::SGN, rd(9, 0xF), vec![c(0), r(30), r(31)]),
            ins(op::LRP, rd(10, 0xF), vec![c(3), c(2), c(0)]),
            ins(op::DP3, rd(11, 0xF), vec![c(0), c(2)]),
            ins(op::DP4, rd(12, 0xF), vec![c(0), c(2)]),
            ins(op::DP2ADD, rd(13, 0xF), vec![c(0), c(2), sw(c(2), "w")]),
            ins(op::CND, rd(14, 0xF), vec![c(3), c(0), c(2)]),
            ins(op::CMP, rd(15, 0xF), vec![c(3), c(0), c(2)]),
            ins(op::POW, rd(16, 0xF), vec![sw(c(2), "z"), sw(c(0), "z")]),
            ins(op::POW, rd(17, 0xF), vec![sw(c(0), "w"), sw(c(0), "w")]),
            ins(op::MUL, rd(18, 0xF), vec![c(0), c(2)]),
            ins(op::MOV, rd(19, 0b1010), vec![c(2)]),
        ];
        let st = vs(&shader(false, 2, 0, p), &f, &[]);
        eq(st.r[0], [2.5, -1.75, 2.0, 4.0]);
        eq(st.r[1], [-1.5, -2.25, 4.0, -4.0]);
        eq(st.r[2], [2.0, 0.5, -2.0, 1.0]);
        eq(st.r[3], [0.5, -2.0, -1.0, 0.0]);
        eq(st.r[4], [2.0, 0.25, 3.0, 4.0]);
        eq(st.r[5], [1.0, 1.0, 0.0, 1.0]);
        eq(st.r[6], [1.0, 0.0, 1.0, 1.0]);
        // -2 * (2, 0.25, -1, 4) = (-4, -0.5, 2, -8)
        eq(st.r[7], [0.0, 0.5, 0.0, 0.0]);
        eq(st.r[8], [0.5, 2.0, 3.0, 0.0]);
        eq(st.r[9], [1.0, -1.0, 1.0, 0.0]);
        // c3 * (c2 - c0) + c0
        eq(
            st.r[10],
            [0.6 * 1.5 + 0.5, 0.5 * 2.25 - 2.0, -0.1 * -4.0 + 3.0, 0.0],
        );
        eq(st.r[11], [1.0 - 0.5 - 3.0; 4]);
        eq(st.r[12], [-2.5; 4]);
        eq(st.r[13], [1.0 - 0.5 + 4.0; 4]);
        eq(st.r[14], [0.5, 0.25, -1.0, 4.0]);
        eq(st.r[15], [0.5, -2.0, -1.0, 0.0]);
        eq(st.r[16], [1.0; 4]); // |-1|^3
        eq(st.r[17], [1.0; 4]); // 0^0
        eq(st.r[18], [1.0, -0.5, -3.0, 0.0]);
        eq(st.r[19], [0.0, 0.25, 0.0, 4.0]);
    }

    #[test]
    fn lit_dst_nrm_crs_sincos() {
        let f = [
            [0.5, 0.25, 0.0, 2.0],                        // c0: lit with x>0, y>0
            [-0.5, 0.25, 0.0, 2.0],                       // c1: x<=0
            [0.5, 0.0, 0.0, 200.0],                       // c2: y<=0
            [3.0, 0.0, 4.0, 7.0],                         // c3: nrm
            [1.0, 0.0, 0.0, 0.0],                         // c4
            [0.0, 1.0, 0.0, 0.0],                         // c5
            [0.0, 0.0, 0.0, std::f32::consts::FRAC_PI_3], // c6
            [9.0, 2.0, 3.0, 4.0],                         // c7
            [9.0, 5.0, 6.0, 7.0],                         // c8
        ];
        let p = vec![
            ins(op::LIT, rd(0, 0xF), vec![c(0)]),
            ins(op::LIT, rd(1, 0xF), vec![c(1)]),
            ins(op::LIT, rd(2, 0xF), vec![c(2)]),
            ins(op::NRM, rd(3, 0xF), vec![c(3)]),
            ins(op::MOV, rd(4, 0xF), vec![c(7)]),
            ins(op::CRS, rd(4, 0xF), vec![c(4), c(5)]),
            ins(op::MOV, rd(5, 0xF), vec![c(7)]),
            ins(op::SINCOS, rd(5, 0b0011), vec![sw(c(6), "w"), r(30), r(31)]),
            ins(op::MOV, rd(6, 0xF), vec![c(7)]),
            ins(op::SINCOS, rd(6, 0b0010), vec![sw(c(6), "w"), r(30), r(31)]),
            ins(op::DST, rd(7, 0xF), vec![c(7), c(8)]),
            ins(op::NRM, rd(8, 0xF), vec![r(31)]),
        ];
        let st = vs(&shader(false, 2, 0, p), &f, &[]);
        eq(st.r[0], [1.0, 0.5, 0.0625, 1.0]);
        eq(st.r[1], [1.0, 0.0, 0.0, 1.0]);
        eq(st.r[2], [1.0, 0.5, 0.0, 1.0]);
        eq(st.r[3], [0.6, 0.0, 0.8, 1.4]);
        eq(st.r[4], [0.0, 0.0, 1.0, 4.0]);
        eq(st.r[5], [0.5, 3f32.sqrt() / 2.0, 3.0, 4.0]);
        eq(st.r[6], [9.0, 3f32.sqrt() / 2.0, 3.0, 4.0]);
        eq(st.r[7], [1.0, 10.0, 3.0, 7.0]);
        // nrm of zero: 0 * inf
        assert!(st.r[8].iter().all(|x| x.is_nan()));
        // lit power clamp: 0.5^200 clamps the exponent to 127.9961
        let st = vs(
            &shader(false, 1, 1, vec![ins(op::LIT, rd(0, 0xF), vec![c(0)])]),
            &[[1.0, 0.5, 0.0, 200.0]],
            &[],
        );
        eq(st.r[0], [1.0, 1.0, 0.5f32.powf(127.9961), 1.0]);
    }

    #[test]
    fn matrix_ops_relative_and_temp() {
        let mut f = vec![[0.0; 4]; 16];
        f[0] = [2.0, 0.0, 0.0, 0.0];
        f[12] = [1.0, 0.0, 0.0, 0.0];
        f[13] = [0.0, 2.0, 0.0, 0.0];
        f[14] = [0.0, 0.0, 0.0, 3.0];
        f[15] = [1.0, 1.0, 1.0, 1.0];
        let mut rel = c(10);
        rel.rel = Some(Rel {
            reg: Reg {
                ty: reg::ADDR,
                index: 0,
            },
            comp: 0,
        });
        let p = vec![
            ins(op::MOV, dst(reg::ADDR, 0, 0b0001), vec![sw(c(0), "x")]),
            ins(op::MOV, rd(0, 0xF), vec![c(15)]),
            ins(op::M4X3, rd(0, 0xF), vec![v(0), rel]),
            ins(op::M4X4, rd(1, 0xF), vec![v(0), rel]),
            ins(op::M4X4, rd(2, 0b0011), vec![v(0), c(12)]),
            ins(op::MOV, rd(8, 0xF), vec![c(12)]),
            ins(op::MOV, rd(9, 0xF), vec![c(13)]),
            ins(op::MOV, rd(10, 0xF), vec![c(15)]),
            ins(op::M3X3, rd(3, 0xF), vec![v(0), r(8)]),
            ins(op::M3X4, rd(4, 0xF), vec![v(0), r(8)]),
            ins(op::M3X2, rd(5, 0xF), vec![v(0), md(r(8), srcmod::NEG)]),
        ];
        let st = vs(&shader(false, 1, 1, p), &f, &[[5.0, 6.0, 7.0, 8.0]]);
        eq(st.r[0], [5.0, 12.0, 24.0, 1.0]);
        eq(st.r[1], [5.0, 12.0, 24.0, 26.0]);
        eq(st.r[2], [5.0, 12.0, 0.0, 0.0]);
        eq(st.r[3], [5.0, 12.0, 18.0, 0.0]);
        // row 3 = r11 = 0
        eq(st.r[4], [5.0, 12.0, 18.0, 0.0]);
        eq(st.r[5], [-5.0, -12.0, 0.0, 0.0]);
        // relative out of range reads zero
        let mut far = c(100);
        far.rel = rel.rel;
        let st = vs(
            &shader(false, 1, 1, vec![ins(op::MOV, rd(0, 0xF), vec![far])]),
            &f,
            &[],
        );
        eq(st.r[0], [0.0; 4]);
    }

    #[test]
    fn address_register_floor_vs_mova() {
        let f = [[1.5, -1.5, 2.5, -0.4]];
        let st = vs(
            &shader(
                false,
                1,
                1,
                vec![ins(op::MOV, dst(reg::ADDR, 0, 0xF), vec![c(0)])],
            ),
            &f,
            &[],
        );
        assert_eq!(st.a0, [1, -2, 2, -1]);
        let st = vs(
            &shader(
                false,
                2,
                0,
                vec![ins(op::MOVA, dst(reg::ADDR, 0, 0xF), vec![c(0)])],
            ),
            &f,
            &[],
        );
        assert_eq!(st.a0, [2, -2, 3, 0]);
        let st = vs(
            &shader(
                false,
                2,
                0,
                vec![ins(op::MOVA, dst(reg::ADDR, 0, 0b0010), vec![c(0)])],
            ),
            &f,
            &[],
        );
        assert_eq!(st.a0, [0, -2, 0, 0]);
        // a0.y used for relative addressing: c[a0.y + 3] = c1
        let mut rel = c(3);
        rel.rel = Some(Rel {
            reg: Reg {
                ty: reg::ADDR,
                index: 0,
            },
            comp: 1,
        });
        let p = vec![
            ins(op::MOVA, dst(reg::ADDR, 0, 0xF), vec![c(0)]),
            ins(op::MOV, rd(0, 0xF), vec![rel]),
        ];
        let st = vs(&shader(false, 2, 0, p), &[f[0], [7.0; 4]], &[]);
        eq(st.r[0], [7.0; 4]);
    }

    #[test]
    fn def_overrides_consts() {
        let p = vec![
            ins(op::MOV, rd(0, 0xF), vec![c(1)]),
            ins(op::MOV, rd(1, 0xF), vec![c(0)]),
            ins(op::MOV, rd(2, 0xF), vec![c(50)]),
            def(1, [9.0, 8.0, 7.0, 6.0]),
            def(60, [5.0; 4]),
            ins(op::MOV, rd(3, 0xF), vec![c(60)]),
        ];
        let st = vs(&shader(false, 1, 1, p), &[[2.0; 4], [1.0; 4]], &[]);
        eq(st.r[0], [9.0, 8.0, 7.0, 6.0]);
        eq(st.r[1], [2.0; 4]);
        eq(st.r[2], [0.0; 4]);
        eq(st.r[3], [5.0; 4]);
    }

    #[test]
    fn vs_outputs() {
        let p = vec![
            ins(op::MOV, dst(reg::RASTOUT, 0, 0xF), vec![v(0)]),
            ins(op::MOV, dst(reg::ATTROUT, 0, 0xF), vec![c(0)]),
            ins(op::MOV, dst(reg::ATTROUT, 1, 0b0100), vec![v(0)]),
            ins(op::MOV, dst(reg::TEXCRDOUT, 7, 0b0011), vec![sw(v(0), "wzyx")]),
            ins(op::MOV, dst(reg::RASTOUT, 2, 0b0001), vec![sw(c(0), "y")]),
        ];
        let mut vin = [[0.0; 4]; 16];
        vin[0] = [1.0, 2.0, 3.0, 4.0];
        let out = run_vs(
            &shader(false, 1, 1, p.clone()),
            &consts(&[[2.0, -3.0, 0.5, 1.0]]),
            &vin,
        )
        .unwrap();
        assert_eq!(out.pos, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(out.d[0], [2.0, -3.0, 0.5, 1.0]);
        assert_eq!(out.d[1], [0.0, 0.0, 3.0, 0.0]);
        assert_eq!(out.t[7], [4.0, 3.0, 0.0, 0.0]);
        assert_eq!(out.t[0], [0.0; 4]);
        assert_eq!(out.fog, 1.0);
        assert_eq!(out.psize, -3.0);
        let mut p2 = p;
        p2.push(ins(op::MOV, dst(reg::RASTOUT, 1, 0b0001), vec![sw(v(0), "z")]));
        let out = run_vs(&shader(false, 1, 1, p2), &consts(&[[0.0; 4]]), &vin).unwrap();
        assert_eq!(out.fog, 3.0);
    }

    #[test]
    fn ps11_tex_texcoord_texkill() {
        let t = |n| dst(reg::TEXTURE, n, 0xF);
        let p = vec![
            ins(op::TEX, t(0), vec![]),
            ins(op::TEXCOORD, t(1), vec![]),
            ins(
                op::ADD,
                rd(0, 0xF),
                vec![src(reg::TEXTURE, 0), src(reg::TEXTURE, 1)],
            ),
        ];
        let mut inp = PsIn::default();
        inp.t[0] = [0.5, 0.25, 0.0, 1.0];
        inp.t[1] = [1.5, -0.5, 0.3, 7.0];
        let out = run_ps(&shader(true, 1, 1, p), &consts(&[]), &inp, &sampler)
            .unwrap()
            .unwrap();
        eq(out.color, [1.0, 0.5, 0.55, 1.0]);
        assert_eq!(out.depth, None);
        let kill = shader(
            true,
            1,
            1,
            vec![
                ins(op::TEXKILL, t(2), vec![]),
                ins(op::MOV, rd(0, 0xF), vec![c(0)]),
            ],
        );
        inp.t[2] = [0.1, 0.1, -0.1, 1.0];
        assert_eq!(run_ps(&kill, &consts(&[]), &inp, &sampler).unwrap(), None);
        inp.t[2] = [0.1, 0.1, 0.1, -1.0]; // w is not tested
        assert!(run_ps(&kill, &consts(&[]), &inp, &sampler).unwrap().is_some());
    }

    #[test]
    fn ps1_coissue_reads_old_state() {
        let mut co = ins(op::MOV, rd(0, 0b1000), vec![sw(r(0), "z")]);
        co.coissue = true;
        let mut co2 = ins(op::MOV, rd(1, 0b1000), vec![v(0)]);
        co2.coissue = true;
        let p = vec![
            ins(op::MOV, rd(0, 0xF), vec![v(0)]),
            ins(op::MUL, rd(0, 0b0111), vec![r(0), c(0)]),
            co,
            ins(op::MOV, rd(1, 0b0111), vec![sw(r(1), "w")]),
            co2,
        ];
        let inp = PsIn {
            d: [[0.1, 0.2, 0.3, 0.4], [0.0; 4]],
            ..PsIn::default()
        };
        let st = ps_state(&shader(true, 1, 1, p), &consts(&[[0.5; 4]]), &inp, &sampler)
            .unwrap()
            .unwrap();
        eq(st.r[0], [0.05, 0.1, 0.15, 0.3]);
        eq(st.r[1], [0.0, 0.0, 0.0, 0.4]);
    }

    #[test]
    fn ps1_constant_clamp() {
        let p = vec![
            def(0, [2.0, -3.0, 0.5, 1.0]),
            ins(op::MOV, rd(0, 0xF), vec![c(0)]),
            ins(op::ADD, rd(1, 0xF), vec![c(1), c(1)]),
        ];
        let sh = shader(true, 1, 4, p);
        let st = ps_state(
            &sh,
            &consts(&[[0.0; 4], [5.0, -5.0, 0.25, 0.0]]),
            &PsIn::default(),
            &sampler,
        )
        .unwrap()
        .unwrap();
        eq(st.r[0], [1.0, -1.0, 0.5, 1.0]);
        eq(st.r[1], [2.0, -2.0, 0.5, 0.0]);
        // ps_2_0 and vertex shaders are not clamped
        let p = vec![ins(op::MOV, dst(reg::COLOROUT, 0, 0xF), vec![c(1)])];
        let out = run_ps(
            &shader(true, 2, 0, p),
            &consts(&[[0.0; 4], [5.0, -5.0, 0.25, 0.0]]),
            &PsIn::default(),
            &sampler,
        );
        assert_eq!(out.unwrap().unwrap().color, [5.0, -5.0, 0.25, 0.0]);
    }

    #[test]
    fn ps14_texcrd_texld_phase() {
        let p = vec![
            ins(
                op::TEXCOORD,
                rd(1, 0b0111),
                vec![md(src(reg::TEXTURE, 0), srcmod::DZ)],
            ),
            ins(op::TEX, rd(2, 0xF), vec![md(src(reg::TEXTURE, 1), srcmod::DW)]),
            Instr::new(op::PHASE),
            ins(op::TEX, rd(3, 0xF), vec![r(1)]),
            ins(op::MOV, rd(0, 0xF), vec![r(2)]),
            ins(op::TEXKILL, rd(1, 0xF), vec![]),
        ];
        let mut inp = PsIn::default();
        inp.t[0] = [2.0, 4.0, 2.0, 1.0];
        inp.t[1] = [1.0, 2.0, 3.0, 4.0];
        let sh = shader(true, 1, 4, p);
        let st = ps_state(&sh, &consts(&[]), &inp, &sampler).unwrap().unwrap();
        eq(st.r[1], [1.0, 2.0, 1.0, 0.0]);
        eq(st.r[2], [2.0, 0.25, 0.5, 0.75]);
        eq(st.r[3], [3.0, 1.0, 2.0, 1.0]);
        eq(
            run_ps(&sh, &consts(&[]), &inp, &sampler).unwrap().unwrap().color,
            [2.0, 0.25, 0.5, 0.75],
        );
        inp.t[0] = [2.0, -4.0, 2.0, 1.0];
        assert_eq!(run_ps(&sh, &consts(&[]), &inp, &sampler).unwrap(), None);
    }

    #[test]
    fn ps20_texld_texldp_outputs_texkill() {
        let s3 = src(reg::SAMPLER, 3);
        let t0 = src(reg::TEXTURE, 0);
        let mut dcl = Instr::new(op::DCL);
        dcl.dst = Some(dst(reg::SAMPLER, 3, 0xF));
        dcl.dcl = Some(crate::sm::DclKind::Sampler(2));
        let p = vec![
            dcl,
            ctl(ins(op::TEX, rd(0, 0xF), vec![t0, s3]), 1),
            ins(op::TEX, rd(1, 0xF), vec![t0, s3]),
            ctl(ins(op::TEX, rd(2, 0xF), vec![t0, s3]), 2),
            ins(op::ADD, dst(reg::COLOROUT, 0, 0xF), vec![r(0), r(1)]),
            ins(op::MOV, dst(reg::DEPTHOUT, 0, 0xF), vec![sw(r(0), "y")]),
            ins(op::TEXKILL, rd(3, 0b0011), vec![]),
        ];
        let mut inp = PsIn::default();
        inp.t[0] = [2.0, 4.0, 6.0, 2.0];
        let sh = shader(true, 2, 0, p);
        let st = ps_state(&sh, &consts(&[]), &inp, &sampler).unwrap().unwrap();
        eq(st.r[0], [3.0, 1.0, 2.0, 3.0]);
        eq(st.r[1], [3.0, 2.0, 4.0, 6.0]);
        eq(st.r[2], [3.0, 2.0, 4.0, 6.0]);
        let out = run_ps(&sh, &consts(&[]), &inp, &sampler).unwrap().unwrap();
        eq(out.color, [6.0, 3.0, 6.0, 9.0]);
        assert_eq!(out.depth, Some(1.0));
        // texkill r3.xy: only masked lanes count
        let mut p2 = sh.instrs.clone();
        p2.insert(1, ins(op::MOV, rd(3, 0xF), vec![md(c(0), srcmod::NEG)]));
        let f = [[0.0, 0.0, 1.0, 1.0]];
        assert!(
            run_ps(&shader(true, 2, 0, p2.clone()), &consts(&f), &inp, &sampler)
                .unwrap()
                .is_some()
        );
        let f = [[0.0, 1.0, 0.0, 0.0]];
        assert_eq!(
            run_ps(&shader(true, 2, 0, p2), &consts(&f), &inp, &sampler).unwrap(),
            None
        );
    }

    #[test]
    fn flow_rep_loop_if() {
        let mut f = vec![[0.0; 4]; 12];
        f[1] = [1.0; 4];
        f[2] = [1.0; 4];
        f[5] = [10.0; 4];
        f[8] = [100.0; 4];
        f[11] = [1000.0; 4];
        let mut al = c(0);
        al.rel = Some(Rel {
            reg: Reg {
                ty: reg::LOOP,
                index: 0,
            },
            comp: 0,
        });
        let ireg = |n| src(reg::CONSTINT, n);
        let b = |n| src(reg::CONSTBOOL, n);
        let p = vec![
            defi(0, [3, 0, 0, 0]),
            defi(1, [4, 2, 3, 0]),
            defi(2, [0, 0, 0, 0]),
            def(20, [2.0; 4]),
            def(21, [5.0; 4]),
            // rep i0: r0 += 1 (x3)
            flow(op::REP, vec![ireg(0)]),
            ins(op::ADD, rd(0, 0xF), vec![r(0), c(1)]),
            flow(op::ENDREP, vec![]),
            // loop aL, i1: aL = 2, 5, 8, 11
            flow(op::LOOP, vec![src(reg::LOOP, 0), ireg(1)]),
            ins(op::ADD, rd(1, 0xF), vec![r(1), al]),
            flow(op::ENDLOOP, vec![]),
            // rep with zero count is skipped
            flow(op::REP, vec![ireg(2)]),
            ins(op::ADD, rd(1, 0xF), vec![r(1), c(1)]),
            flow(op::ENDREP, vec![]),
            // if b0 (true) / else
            flow(op::IF, vec![b(0)]),
            ins(op::MOV, rd(2, 0xF), vec![c(1)]),
            flow(op::ELSE, vec![]),
            ins(op::MOV, rd(2, 0xF), vec![c(20)]),
            flow(op::ENDIF, vec![]),
            // if !b1 (b1 false)
            flow(op::IF, vec![md(b(1), srcmod::NOT)]),
            ins(op::ADD, rd(2, 0xF), vec![r(2), c(1)]),
            flow(op::ENDIF, vec![]),
            // if b1 (false) without else
            flow(op::IF, vec![b(1)]),
            ins(op::ADD, rd(2, 0xF), vec![r(2), c(20)]),
            flow(op::ENDIF, vec![]),
            // rep i0 with breakc_ge r3.x, c20.x: stops at 2
            flow(op::REP, vec![ireg(0)]),
            ins(op::ADD, rd(3, 0xF), vec![r(3), c(1)]),
            ctl(flow(op::BREAKC, vec![sw(r(3), "x"), sw(c(20), "x")]), 3),
            flow(op::ENDREP, vec![]),
            // ifc lt: 1 < 5 taken, ifc gt: 1 > 5 not taken (else branch)
            ctl(flow(op::IFC, vec![sw(c(1), "x"), sw(c(21), "x")]), 4),
            ins(op::ADD, rd(5, 0xF), vec![r(5), c(1)]),
            flow(op::ENDIF, vec![]),
            ctl(flow(op::IFC, vec![sw(c(1), "x"), sw(c(21), "x")]), 1),
            ins(op::ADD, rd(5, 0xF), vec![r(5), c(20)]),
            flow(op::ELSE, vec![]),
            ins(op::ADD, rd(5, 0xF), vec![r(5), c(21)]),
            flow(op::ENDIF, vec![]),
            // nested: rep i0 { loop aL,i1 { break } r6 += aL } -> aL restored to 0 outside
            flow(op::REP, vec![ireg(0)]),
            flow(op::LOOP, vec![src(reg::LOOP, 0), ireg(1)]),
            flow(op::BREAK, vec![]),
            flow(op::ENDLOOP, vec![]),
            ins(op::ADD, rd(6, 0xF), vec![r(6), src(reg::LOOP, 0)]),
            flow(op::ENDREP, vec![]),
            // subroutines
            flow(op::CALL, vec![src(reg::LABEL, 0)]),
            flow(op::CALLNZ, vec![src(reg::LABEL, 1), b(0)]),
            flow(op::CALLNZ, vec![src(reg::LABEL, 1), b(1)]),
            flow(op::RET, vec![]),
            flow(op::LABEL, vec![src(reg::LABEL, 0)]),
            ins(op::ADD, rd(4, 0xF), vec![r(4), c(1)]),
            flow(op::RET, vec![]),
            flow(op::LABEL, vec![src(reg::LABEL, 1)]),
            ins(op::ADD, rd(4, 0xF), vec![r(4), c(20)]),
            flow(op::RET, vec![]),
        ];
        let sh = shader(false, 2, 0, p);
        let mut v = [[0.0; 4]; 16];
        v[0] = [0.0; 4];
        let st = vs_state(
            &sh,
            &Consts {
                f: &f,
                i: &[],
                b: &[true, false],
            },
            &v,
        )
        .unwrap();
        eq(st.r[0], [3.0; 4]);
        eq(st.r[1], [1111.0; 4]);
        eq(st.r[2], [2.0; 4]);
        eq(st.r[3], [2.0; 4]);
        eq(st.r[4], [3.0; 4]);
        eq(st.r[5], [6.0; 4]);
        eq(st.r[6], [0.0; 4]);
        assert_eq!(st.al, 0);
    }

    #[test]
    fn predicates() {
        let p0 = src(reg::PREDICATE, 0);
        let mut pm = ins(op::MOV, rd(0, 0xF), vec![c(2)]);
        pm.predicate = Some(p0);
        let mut pn = ins(op::MOV, rd(1, 0xF), vec![c(2)]);
        pn.predicate = Some(md(p0, srcmod::NOT));
        let p = vec![
            ctl(ins(op::SETP, dst(reg::PREDICATE, 0, 0xF), vec![c(0), c(1)]), 4),
            pm,
            pn,
            flow(op::REP, vec![src(reg::CONSTINT, 0)]),
            ins(op::ADD, rd(2, 0xF), vec![r(2), c(2)]),
            flow(op::BREAKP, vec![sw(p0, "y")]),
            flow(op::ENDREP, vec![]),
        ];
        let f = [[1.0, 2.0, 3.0, 4.0], [2.0, 1.0, 4.0, 3.0], [9.0; 4]];
        let st = vs_state(
            &shader(false, 2, 1, p),
            &Consts {
                f: &f,
                i: &[[5, 0, 0, 0]],
                b: &[],
            },
            &[[0.0; 4]; 16],
        )
        .unwrap();
        assert_eq!(st.p, [true, false, true, false]);
        eq(st.r[0], [9.0, 0.0, 9.0, 0.0]);
        eq(st.r[1], [0.0, 9.0, 0.0, 9.0]);
        eq(st.r[2], [45.0; 4]);
    }

    #[test]
    fn unimplemented_and_bad_programs() {
        let p = vec![ins(
            op::TEXBEM,
            dst(reg::TEXTURE, 1, 0xF),
            vec![src(reg::TEXTURE, 0)],
        )];
        let e = run_ps(&shader(true, 1, 1, p), &consts(&[]), &PsIn::default(), &sampler).unwrap_err();
        assert!(e.contains("texbem"), "{e}");
        let p = vec![ins(op::DSX, rd(0, 0xF), vec![r(1)])];
        let e = run_ps(&shader(true, 2, 1, p), &consts(&[]), &PsIn::default(), &sampler).unwrap_err();
        assert!(e.contains("dsx"), "{e}");
        let p = vec![flow(op::REP, vec![src(reg::CONSTINT, 0)])];
        assert!(vs_state(&shader(false, 2, 0, p), &consts(&[]), &[[0.0; 4]; 16]).is_err());
    }

    #[test]
    fn decoded_token_stream() {
        // vs_1_1: dp4 oPos.x, v0, c0 / mov oD0, c[a0.x + 1] after mov a0.x, c0.w / end
        let tokens = [
            0xFFFE_0101,
            0x0000_0001,
            0x8001_0000 | 3 << 28,
            0xA0FF_0000, // mov a0.x, c0.w
            0x0000_0009,
            0xC001_0000,
            0x90E4_0000,
            0xA0E4_0000, // dp4 oPos.x, v0, c0
            0x0000_0001,
            0xD00F_0000,
            0xA0E4_2001, // mov oD0, c[a0.x + 1]
            0x0000_FFFF,
        ];
        let sh = crate::sm::parse(&tokens).unwrap();
        let mut vin = [[0.0; 4]; 16];
        vin[0] = [1.0, 2.0, 3.0, 4.0];
        let f = [[1.0, 1.0, 1.0, 1.0], [0.0; 4], [0.5, 0.25, 0.0, 1.0]];
        let out = run_vs(&sh, &consts(&f), &vin).unwrap_or_else(|e| panic!("{e}\n{sh}"));
        assert_eq!(out.pos, [10.0, 0.0, 0.0, 0.0]);
        assert_eq!(out.d[0], [0.5, 0.25, 0.0, 1.0]);
    }
}
