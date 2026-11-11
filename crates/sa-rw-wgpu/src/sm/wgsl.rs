//! D3D9 shader ([`Shader`]) → WGSL.
//!
//! A vertex shader becomes `fn vs_body(v: array<vec4f, 16>) -> SmVsOut` and a pixel shader
//! `fn ps_body(d0: vec4f, d1: vec4f, tc: array<vec4f, 8>) -> SmPsOut`: plain functions over the
//! D3D9 registers, which the device wraps in entry points (vertex layout, half-pixel offset, fog,
//! alpha test) and the tests call from compute/fragment harnesses. Both read the constant
//! registers from [`PRELUDE`]'s `sm` uniform (group 0, binding 1) and pixel shaders sample stage
//! `s` through `sm_tex{s}`/`sm_smp{s}` (group 1, bindings `2s`/`2s + 1`).
//!
//! Semantics follow the reference interpreter ([`super::interp`]): each instruction computes a
//! `vec4f` from its swizzled/modified sources, applies the result shift and saturation and writes
//! the masked components; coissued pairs compute both results before writing either; `def`
//! constants become exact bit-pattern literals; relative constant reads go through a bounds-checked
//! function (out of range → 0). One deliberate difference: WGSL may assume finite floats, so
//! `rcp`/`rsq`/`log`/`nrm` of 0 give ±`f32::MAX` where the interpreter gives ±infinity.

use std::fmt::Write as _;

use super::{op, reg, srcmod, Dst, Instr, Rel, Shader, Src, Version};

/// Constant registers the translated code reads (`SetVertexShaderConstantF/I/B`, pixel
/// equivalents), at group 0 binding 1. Byte layout: [`ConstBlock`](crate::sm::wgsl::ConstBlock).
pub const PRELUDE: &str = r#"
struct SmConsts {
    vc: array<vec4f, 256>,
    pc: array<vec4f, 32>,
    vi: array<vec4i, 16>,
    pi: array<vec4i, 16>,
    // bool registers b0..b15: [n / 4][n % 4]
    vb: array<vec4u, 4>,
    pb: array<vec4u, 4>,
}
@group(0) @binding(1) var<uniform> sm: SmConsts;

const SM_MAX: f32 = 3.40282347e38;

struct SmVsOut {
    pos: vec4f,
    d0: vec4f,
    d1: vec4f,
    t: array<vec4f, 8>,
    fog: f32,
    psize: f32,
}

struct SmPsOut {
    color: vec4f,
    depth: f32,
}

fn sm_rcp(x: f32) -> f32 {
    if x == 1.0 {
        return 1.0;
    }
    if x == 0.0 {
        return SM_MAX;
    }
    return 1.0 / x;
}

fn sm_rsq(x: f32) -> f32 {
    let a = abs(x);
    if a == 1.0 {
        return 1.0;
    }
    if a == 0.0 {
        return SM_MAX;
    }
    return inverseSqrt(a);
}

fn sm_log(x: f32) -> f32 {
    let a = abs(x);
    if a == 0.0 {
        return -SM_MAX;
    }
    return log2(a);
}

fn sm_pow(x: f32, y: f32) -> f32 {
    let a = abs(x);
    if y == 0.0 {
        return 1.0;
    }
    if a == 0.0 {
        return select(SM_MAX, 0.0, y > 0.0);
    }
    return exp2(y * log2(a));
}

fn sm_expp1(x: f32) -> vec4f {
    let f = floor(x);
    return vec4f(exp2(f), x - f, exp2(x), 1.0);
}

fn sm_logp1(x: f32) -> vec4f {
    let v = abs(x);
    if v == 0.0 {
        return vec4f(-SM_MAX, 1.0, -SM_MAX, 1.0);
    }
    let b = bitcast<u32>(v);
    return vec4f(f32(i32((b >> 23u) & 0xffu) - 127), bitcast<f32>((b & 0x7fffffu) | 0x3f800000u), log2(v), 1.0);
}

fn sm_lit(s: vec4f) -> vec4f {
    var d = vec4f(1.0, 0.0, 0.0, 1.0);
    let power = clamp(s.w, -127.9961, 127.9961);
    if s.x > 0.0 {
        d.y = s.x;
        if s.y > 0.0 {
            d.z = sm_pow(s.y, power);
        }
    }
    return d;
}

fn sm_nrm(s: vec4f) -> vec4f {
    let f = dot(s.xyz, s.xyz);
    return s * select(inverseSqrt(f), SM_MAX, f == 0.0);
}

fn sm_bool(b: array<vec4u, 4>, n: u32) -> bool {
    return b[n / 4u][n % 4u] != 0u;
}
"#;

/// Byte image of `SmConsts` (5248 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct ConstBlock {
    pub vc: [[f32; 4]; 256],
    pub pc: [[f32; 4]; 32],
    pub vi: [[i32; 4]; 16],
    pub pi: [[i32; 4]; 16],
    pub vb: [u32; 16],
    pub pb: [u32; 16],
}

impl Default for ConstBlock {
    fn default() -> ConstBlock {
        bytemuck::Zeroable::zeroed()
    }
}

/// Texture view dimension a pixel shader samples a stage with ([`super::sampler_type`] values).
pub type SamplerKind = u8;

/// A translated shader.
#[derive(Debug, Clone)]
pub struct Translated {
    pub version: Version,
    /// WGSL functions (needs [`PRELUDE`] before it).
    pub code: String,
    /// Vertex shaders: declared inputs (register, D3DDECLUSAGE, usage index).
    pub inputs: Vec<(u32, u8, u8)>,
    /// Pixel shaders: sampled stages and the dimension each is sampled with.
    pub samplers: Vec<(u32, SamplerKind)>,
    /// Pixel shaders: `oDepth` written (`SmPsOut::depth` valid).
    pub writes_depth: bool,
    /// Pixel shaders: texcoord inputs read (bit n = `tc[n]`).
    pub texcoords_used: u8,
}

/// Translation failure (unsupported instruction or operand).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslateError(pub String);

impl std::fmt::Display for TranslateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TranslateError {}

const C: [char; 4] = ['x', 'y', 'z', 'w'];

fn v4lit(bits: [u32; 4]) -> String {
    format!("bitcast<vec4f>(vec4u({:#x}u, {:#x}u, {:#x}u, {:#x}u))", bits[0], bits[1], bits[2], bits[3])
}

struct Tr<'a> {
    sh: &'a Shader,
    v: Version,
    kinds: &'a dyn Fn(u32) -> SamplerKind,
    fdefs: Vec<(u32, [u32; 4])>,
    idefs: Vec<(u32, [u32; 4])>,
    bdefs: Vec<(u32, bool)>,
    temps: u32,
    samplers: Vec<(u32, SamplerKind)>,
    tc_used: u8,
    writes_depth: bool,
    tmp: u32,
}

type R<T> = Result<T, TranslateError>;

fn err<T>(m: String) -> R<T> {
    Err(TranslateError(m))
}

impl Tr<'_> {
    fn ps1(&self) -> bool {
        self.v.pixel && self.v.major == 1
    }

    fn fresh(&mut self) -> String {
        self.tmp += 1;
        format!("_t{}", self.tmp)
    }

    fn sampler_kind(&mut self, stage: u32) -> SamplerKind {
        if let Some(&(_, k)) = self.samplers.iter().find(|s| s.0 == stage) {
            return k;
        }
        // ps_2_x: the declared dimension; ps_1_x: the bound texture's
        let declared = self.sh.sampler_decls().into_iter().find(|d| d.0 == stage).map(|d| d.1);
        let k = match declared.unwrap_or_else(|| (self.kinds)(stage)) {
            3 => 3,
            4 => 4,
            _ => 2,
        };
        self.samplers.push((stage, k));
        k
    }

    /// The constant float register `idx` (non-relative).
    fn fconst(&self, idx: u32) -> String {
        if let Some((_, b)) = self.fdefs.iter().find(|d| d.0 == idx) {
            return v4lit(*b);
        }
        if self.v.pixel {
            if idx >= 32 {
                return "vec4f(0.0)".into();
            }
            format!("sm.pc[{idx}]")
        } else {
            if idx >= 256 {
                return "vec4f(0.0)".into();
            }
            format!("sm.vc[{idx}]")
        }
    }

    fn rel_index(&self, rel: &Rel, off: u32) -> R<String> {
        match rel.reg.ty {
            reg::ADDR if !self.v.pixel => Ok(format!("(a0.{} + {off})", C[rel.comp as usize])),
            reg::LOOP => Ok(format!("(aL + {off})")),
            t => err(format!("relative addressing through register type {t}")),
        }
    }

    /// Register value as `vec4f` before swizzle/modifier.
    fn reg_value(&mut self, s: &Src, offset: u32) -> R<String> {
        let i = s.reg.index + offset;
        Ok(match s.reg.ty {
            reg::TEMP => {
                self.temps = self.temps.max(i + 1);
                format!("r{i}")
            }
            reg::INPUT if self.v.pixel => match i {
                0 => "d0".into(),
                1 => "d1".into(),
                _ => return err(format!("pixel input v{i}")),
            },
            reg::INPUT => {
                if i >= 16 {
                    return err(format!("vertex input v{i}"));
                }
                format!("v[{i}]")
            }
            reg::CONST => {
                if let Some(rel) = s.rel {
                    let f = if self.v.pixel { "sm_pc_rel" } else { "sm_vc_rel" };
                    format!("{f}({})", self.rel_index(&rel, i)?)
                } else {
                    let c = self.fconst(i);
                    if self.ps1() { format!("clamp({c}, vec4f(-1.0), vec4f(1.0))") } else { c }
                }
            }
            reg::TEXTURE if self.v.pixel => {
                if i >= 8 {
                    return err(format!("t{i}"));
                }
                self.tc_used |= 1 << i;
                format!("t{i}")
            }
            reg::CONSTINT => {
                if let Some((_, b)) = self.idefs.iter().find(|d| d.0 == i) {
                    format!("vec4f(bitcast<vec4i>(vec4u({:#x}u, {:#x}u, {:#x}u, {:#x}u)))", b[0], b[1], b[2], b[3])
                } else {
                    format!("vec4f(sm.{}i[{i}])", if self.v.pixel { 'p' } else { 'v' })
                }
            }
            reg::LOOP => "vec4f(f32(aL))".into(),
            t => return err(format!("source register type {t}")),
        })
    }

    fn src_off(&mut self, s: &Src, offset: u32) -> R<String> {
        let base = self.reg_value(s, offset)?;
        let sw = if s.swizzle == [0, 1, 2, 3] {
            base
        } else {
            format!("({base}).{}", s.swizzle.iter().map(|&c| C[c as usize]).collect::<String>())
        };
        Ok(match s.modifier {
            srcmod::NONE => sw,
            srcmod::NEG => format!("(-{sw})"),
            srcmod::BIAS => format!("({sw} - vec4f(0.5))"),
            srcmod::BIASNEG => format!("(vec4f(0.5) - {sw})"),
            srcmod::SIGN => format!("({sw} * 2.0 - vec4f(1.0))"),
            srcmod::SIGNNEG => format!("(vec4f(1.0) - {sw} * 2.0)"),
            srcmod::COMP => format!("(vec4f(1.0) - {sw})"),
            srcmod::X2 => format!("({sw} * 2.0)"),
            srcmod::X2NEG => format!("({sw} * -2.0)"),
            srcmod::DZ => format!("({sw} / ({sw}).z)"),
            srcmod::DW => format!("({sw} / ({sw}).w)"),
            srcmod::ABS => format!("abs({sw})"),
            srcmod::ABSNEG => format!("(-abs({sw}))"),
            m => return err(format!("source modifier {m}")),
        })
    }

    fn src(&mut self, s: &Src) -> R<String> {
        self.src_off(s, 0)
    }

    /// Bool condition of `if bN` / `callnz`.
    fn bool_src(&self, s: &Src) -> R<String> {
        if s.reg.ty != reg::CONSTBOOL {
            return err(format!("bool source of register type {}", s.reg.ty));
        }
        let i = s.reg.index;
        let c = if let Some((_, b)) = self.bdefs.iter().find(|d| d.0 == i) {
            b.to_string()
        } else {
            format!("sm_bool(sm.{}b, {i}u)", if self.v.pixel { 'p' } else { 'v' })
        };
        Ok(if s.modifier == srcmod::NOT { format!("!({c})") } else { c })
    }

    /// Destination l-value name and whether it is the address register.
    fn dst_name(&mut self, d: &Dst) -> R<String> {
        let i = d.reg.index;
        Ok(match d.reg.ty {
            reg::TEMP => {
                self.temps = self.temps.max(i + 1);
                format!("r{i}")
            }
            reg::ADDR if !self.v.pixel => "a0".into(),
            reg::TEXTURE if self.v.pixel && i < 8 => {
                self.tc_used |= 1 << i;
                format!("t{i}")
            }
            reg::RASTOUT if !self.v.pixel => match i {
                0 => "o_pos".into(),
                1 => "o_fog".into(),
                2 => "o_pts".into(),
                _ => return err(format!("oR{i}")),
            },
            reg::ATTROUT if !self.v.pixel && i < 2 => format!("o_d{i}"),
            reg::TEXCRDOUT if !self.v.pixel && i < 8 => format!("o_t{i}"),
            reg::COLOROUT if self.v.pixel && i == 0 => "o_c0".into(),
            reg::COLOROUT if self.v.pixel => format!("o_c{i}"),
            reg::DEPTHOUT if self.v.pixel => {
                self.writes_depth = true;
                "o_depth".into()
            }
            t => return err(format!("destination register type {t} index {i}")),
        })
    }

    /// Statements writing `value` (a `vec4f` expression already computed into a `let`) to `d`.
    fn write(&mut self, d: &Dst, value: &str, mask_limit: u8, out: &mut Vec<String>) -> R<()> {
        let name = self.dst_name(d)?;
        let mask = d.mask & mask_limit;
        let mut v = value.to_string();
        if d.shift != 0 {
            let f = 2f32.powi(d.shift as i32);
            v = format!("({v} * {f:?})");
        }
        if d.saturate {
            v = format!("clamp({v}, vec4f(0.0), vec4f(1.0))");
        }
        if name == "a0" {
            let conv = if self.v.major == 1 {
                format!("vec4i(floor({v}))")
            } else {
                format!("vec4i(sign({v}) * floor(abs({v}) + vec4f(0.5)))")
            };
            let t = self.fresh();
            out.push(format!("let {t} = {conv};"));
            v = t;
        } else if v != value {
            let t = self.fresh();
            out.push(format!("let {t} = {v};"));
            v = t;
        }
        if mask == 0xF {
            out.push(format!("{name} = {v};"));
        } else {
            for (k, c) in C.iter().enumerate() {
                if mask >> k & 1 != 0 {
                    out.push(format!("{name}.{c} = {v}.{c};"));
                }
            }
        }
        Ok(())
    }

    fn sample(&mut self, stage: u32, coord: &str, control: u8) -> String {
        let k = self.sampler_kind(stage);
        let c = match k {
            3 | 4 => format!("({coord}).xyz"),
            _ => format!("({coord}).xy"),
        };
        match control {
            1 => {
                let c = match k {
                    3 | 4 => format!("(({coord}).xyz / ({coord}).w)"),
                    _ => format!("(({coord}).xy / ({coord}).w)"),
                };
                format!("textureSample(sm_tex{stage}, sm_smp{stage}, {c})")
            }
            2 => format!("textureSampleBias(sm_tex{stage}, sm_smp{stage}, {c}, ({coord}).w)"),
            _ => format!("textureSample(sm_tex{stage}, sm_smp{stage}, {c})"),
        }
    }

    /// Computes one instruction: `let` statements (`pre`), the result writes (`post`).
    fn instr(&mut self, ins: &Instr, pre: &mut Vec<String>, post: &mut Vec<String>) -> R<()> {
        let o = ins.opcode;
        let n = ins.src.len();
        let s = |tr: &mut Self, k: usize| -> R<String> {
            match ins.src.get(k) {
                Some(x) => tr.src(x),
                None => err(format!("opcode {o}: missing source {k}")),
            }
        };
        let dst = ins.dst;
        let mut value: Option<(String, u8)> = None;
        match o {
            op::NOP | op::DCL | op::DEF | op::DEFI | op::DEFB | op::PHASE => {}
            op::MOV | op::MOVA => value = Some((s(self, 0)?, 0xF)),
            op::ADD => value = Some((format!("({} + {})", s(self, 0)?, s(self, 1)?), 0xF)),
            op::SUB => value = Some((format!("({} - {})", s(self, 0)?, s(self, 1)?), 0xF)),
            op::MUL => value = Some((format!("({} * {})", s(self, 0)?, s(self, 1)?), 0xF)),
            op::MAD => value = Some((format!("({} * {} + {})", s(self, 0)?, s(self, 1)?, s(self, 2)?), 0xF)),
            op::RCP => value = Some((format!("vec4f(sm_rcp(({}).w))", s(self, 0)?), 0xF)),
            op::RSQ => value = Some((format!("vec4f(sm_rsq(({}).w))", s(self, 0)?), 0xF)),
            op::DP3 => value = Some((format!("vec4f(dot(({}).xyz, ({}).xyz))", s(self, 0)?, s(self, 1)?), 0xF)),
            op::DP4 => value = Some((format!("vec4f(dot({}, {}))", s(self, 0)?, s(self, 1)?), 0xF)),
            op::MIN => value = Some((format!("min({}, {})", s(self, 0)?, s(self, 1)?), 0xF)),
            op::MAX => value = Some((format!("max({}, {})", s(self, 0)?, s(self, 1)?), 0xF)),
            op::SLT => value = Some((format!("select(vec4f(0.0), vec4f(1.0), {} < {})", s(self, 0)?, s(self, 1)?), 0xF)),
            op::SGE => value = Some((format!("select(vec4f(0.0), vec4f(1.0), {} >= {})", s(self, 0)?, s(self, 1)?), 0xF)),
            op::EXP => value = Some((format!("vec4f(exp2(({}).w))", s(self, 0)?), 0xF)),
            op::LOG => value = Some((format!("vec4f(sm_log(({}).w))", s(self, 0)?), 0xF)),
            op::EXPP if self.v.major == 1 => value = Some((format!("sm_expp1(({}).w)", s(self, 0)?), 0xF)),
            op::EXPP => value = Some((format!("vec4f(exp2(({}).w))", s(self, 0)?), 0xF)),
            op::LOGP if self.v.major == 1 => value = Some((format!("sm_logp1(({}).w)", s(self, 0)?), 0xF)),
            op::LOGP => value = Some((format!("vec4f(sm_log(({}).w))", s(self, 0)?), 0xF)),
            op::LIT => value = Some((format!("sm_lit({})", s(self, 0)?), 0xF)),
            op::DST => {
                let (a, b) = (s(self, 0)?, s(self, 1)?);
                value = Some((format!("vec4f(1.0, ({a}).y * ({b}).y, ({a}).z, ({b}).w)"), 0xF));
            }
            op::LRP => {
                let (a, b, c) = (s(self, 0)?, s(self, 1)?, s(self, 2)?);
                value = Some((format!("({a} * ({b} - {c}) + {c})"), 0xF));
            }
            op::FRC => {
                let a = s(self, 0)?;
                value = Some((format!("({a} - floor({a}))"), 0xF));
            }
            op::M4X4 | op::M4X3 | op::M3X4 | op::M3X3 | op::M3X2 => {
                let (rows, four, lim) = match o {
                    op::M4X4 => (4, true, 0xF),
                    op::M4X3 => (3, true, 0x7),
                    op::M3X4 => (4, false, 0xF),
                    op::M3X3 => (3, false, 0x7),
                    _ => (2, false, 0x3),
                };
                let a = s(self, 0)?;
                let ta = self.fresh();
                pre.push(format!("let {ta} = {a};"));
                let row_src = *ins.src.get(1).ok_or_else(|| TranslateError("matrix without rows".into()))?;
                let mut lanes = Vec::new();
                for k in 0..4 {
                    if k < rows {
                        let r = self.src_off(&row_src, k as u32)?;
                        lanes.push(if four { format!("dot({ta}, {r})") } else { format!("dot({ta}.xyz, ({r}).xyz)") });
                    } else {
                        lanes.push("0.0".into());
                    }
                }
                value = Some((format!("vec4f({})", lanes.join(", ")), lim));
            }
            op::POW => value = Some((format!("vec4f(sm_pow(({}).w, ({}).w))", s(self, 0)?, s(self, 1)?), 0xF)),
            op::CRS => value = Some((format!("vec4f(cross(({}).xyz, ({}).xyz), 0.0)", s(self, 0)?, s(self, 1)?), 0x7)),
            op::SGN => value = Some((format!("sign({})", s(self, 0)?), 0xF)),
            op::ABS => value = Some((format!("abs({})", s(self, 0)?), 0xF)),
            op::NRM => value = Some((format!("sm_nrm({})", s(self, 0)?), 0xF)),
            op::SINCOS => {
                let a = s(self, 0)?;
                value = Some((format!("vec4f(cos(({a}).w), sin(({a}).w), 0.0, 0.0)"), 0x3));
            }
            op::CND => {
                let (a, b, c) = (s(self, 0)?, s(self, 1)?, s(self, 2)?);
                value = Some((format!("select({c}, {b}, {a} > vec4f(0.5))"), 0xF));
            }
            op::CMP => {
                let (a, b, c) = (s(self, 0)?, s(self, 1)?, s(self, 2)?);
                value = Some((format!("select({c}, {b}, {a} >= vec4f(0.0))"), 0xF));
            }
            op::DP2ADD => {
                let (a, b, c) = (s(self, 0)?, s(self, 1)?, s(self, 2)?);
                value = Some((format!("vec4f(({a}).x * ({b}).x + ({a}).y * ({b}).y + ({c}).w)"), 0xF));
            }
            op::DSX if self.v.pixel => value = Some((format!("dpdx({})", s(self, 0)?), 0xF)),
            op::DSY if self.v.pixel => value = Some((format!("dpdy({})", s(self, 0)?), 0xF)),
            op::TEXCOORD if self.v.pixel => {
                let d = dst.ok_or_else(|| TranslateError("texcoord without destination".into()))?;
                if self.v.major == 1 && self.v.minor < 4 {
                    let i = d.reg.index;
                    self.tc_used |= 1 << i;
                    value = Some((format!("vec4f(clamp(tc[{i}].xyz, vec3f(0.0), vec3f(1.0)), 1.0)"), 0xF));
                } else {
                    value = Some((s(self, 0)?, 0xF));
                }
            }
            op::TEX if self.v.pixel => {
                let d = dst.ok_or_else(|| TranslateError("texld without destination".into()))?;
                if self.v.major >= 2 {
                    let samp = ins.src.get(1).ok_or_else(|| TranslateError("texld without sampler".into()))?;
                    let c = s(self, 0)?;
                    let t = self.fresh();
                    pre.push(format!("let {t} = {c};"));
                    value = Some((self.sample(samp.reg.index, &t, ins.control), 0xF));
                } else if self.v.minor >= 4 {
                    let c = s(self, 0)?;
                    let t = self.fresh();
                    pre.push(format!("let {t} = {c};"));
                    value = Some((self.sample(d.reg.index, &t, 0), 0xF));
                } else {
                    let i = d.reg.index;
                    self.tc_used |= 1 << i;
                    value = Some((self.sample(i, &format!("t{i}"), 0), 0xF));
                }
            }
            op::TEXKILL if self.v.pixel => {
                let d = dst.ok_or_else(|| TranslateError("texkill without register".into()))?;
                let name = self.dst_name(&d)?;
                let mask = if self.v.major == 1 { 0x7 } else { d.mask };
                let lanes: Vec<String> = (0..4).filter(|k| mask >> k & 1 != 0).map(|k| format!("{name}.{} < 0.0", C[k])).collect();
                if !lanes.is_empty() {
                    post.push(format!("if {} {{ discard; }}", lanes.join(" || ")));
                }
                return Ok(());
            }
            op::REP => {
                let c = s(self, 0)?;
                let k = self.fresh();
                post.push(format!("for (var {k} = 0; {k} < clamp(i32(({c}).x), 0, 255); {k}++) {{"));
                return Ok(());
            }
            op::LOOP => {
                // loop aL, iN: src0 = aL, src1 = iN
                let c = s(self, 1)?;
                let k = self.fresh();
                let t = self.fresh();
                post.push(format!("let {t} = vec4i({c});"));
                post.push(format!("for (var {k} = 0; {k} < clamp({t}.x, 0, 255); {k}++) {{"));
                post.push(format!("aL = {t}.y + {k} * {t}.z;"));
                return Ok(());
            }
            op::ENDREP | op::ENDLOOP | op::ENDIF => {
                post.push("}".into());
                return Ok(());
            }
            op::ELSE => {
                post.push("} else {".into());
                return Ok(());
            }
            op::IF => {
                let b = ins.src.first().ok_or_else(|| TranslateError("if without source".into()))?;
                post.push(format!("if {} {{", self.bool_src(b)?));
                return Ok(());
            }
            op::IFC | op::BREAKC => {
                let (a, b) = (s(self, 0)?, s(self, 1)?);
                let cmp = match ins.control & 7 {
                    1 => ">",
                    2 => "==",
                    3 => ">=",
                    4 => "<",
                    5 => "!=",
                    6 => "<=",
                    c => return err(format!("comparison {c}")),
                };
                let cond = format!("({a}).w {cmp} ({b}).w");
                post.push(if o == op::IFC { format!("if {cond} {{") } else { format!("if {cond} {{ break; }}") });
                return Ok(());
            }
            op::BREAK => {
                post.push("break;".into());
                return Ok(());
            }
            op::RET if n == 0 => {
                return Ok(());
            }
            _ => {
                let name = super::parse::mnemonic(o, self.v, ins.control);
                return err(format!("{} instruction `{name}` (opcode {o}) is not translated", self.v));
            }
        }
        if ins.predicate.is_some() {
            return err("predicated instructions are not translated".into());
        }
        if let (Some((val, lim)), Some(d)) = (value, dst) {
            if matches!(d.reg.ty, reg::RASTOUT | reg::ATTROUT | reg::TEXCRDOUT) && d.rel.is_some() {
                return err("relative output addressing".into());
            }
            let t = self.fresh();
            pre.push(format!("let {t} = {val};"));
            self.write(&d, &t, lim, post)?;
        }
        Ok(())
    }

    fn run(&mut self) -> R<String> {
        let instrs: &[Instr] = &self.sh.instrs;
        // subroutines are not translated: stop at the first top-level `ret`
        let mut out_lines: Vec<String> = Vec::new();
        let mut held: Vec<String> = Vec::new();
        for (k, ins) in instrs.iter().enumerate() {
            if ins.opcode == op::RET || ins.opcode == op::LABEL {
                if instrs[k + 1..].iter().any(|i| !matches!(i.opcode, op::LABEL | op::RET)) && ins.opcode == op::LABEL {
                    return err("subroutines (label/call) are not translated".into());
                }
                break;
            }
            if matches!(ins.opcode, op::CALL | op::CALLNZ) {
                return err("subroutines (call) are not translated".into());
            }
            let mut pre = Vec::new();
            let mut post = Vec::new();
            self.instr(ins, &mut pre, &mut post)?;
            let next_coissue = instrs.get(k + 1).is_some_and(|i| i.coissue);
            if ins.coissue {
                // this instruction read the state from before its partner: emit its computation,
                // then the partner's held writes, then its own
                out_lines.extend(pre);
                out_lines.append(&mut held);
                out_lines.extend(post);
            } else if next_coissue {
                out_lines.extend(pre);
                held = post;
            } else {
                out_lines.extend(pre);
                out_lines.extend(post);
            }
        }
        out_lines.append(&mut held);
        Ok(out_lines.join("\n"))
    }
}

/// Translates `sh`. `kinds(stage)` gives the texture dimension bound to a stage for pixel
/// shaders without sampler declarations (ps_1_x): [`super::sampler_type`] (2D when unknown).
pub fn translate(sh: &Shader, kinds: &dyn Fn(u32) -> SamplerKind) -> Result<Translated, TranslateError> {
    let v = sh.version;
    if v.major >= 3 {
        return err(format!("{v} is not translated"));
    }
    let mut tr = Tr {
        sh,
        v,
        kinds,
        fdefs: Vec::new(),
        idefs: Vec::new(),
        bdefs: Vec::new(),
        temps: 0,
        samplers: Vec::new(),
        tc_used: 0,
        writes_depth: false,
        tmp: 0,
    };
    for ins in &sh.instrs {
        let Some(d) = ins.dst else { continue };
        match ins.opcode {
            op::DEF => {
                let mut b = ins.imm;
                if tr.ps1() {
                    b = b.map(|x| f32::from_bits(x).clamp(-1.0, 1.0).to_bits());
                }
                tr.fdefs.retain(|e| e.0 != d.reg.index);
                tr.fdefs.push((d.reg.index, b));
            }
            op::DEFI => tr.idefs.push((d.reg.index, ins.imm)),
            op::DEFB => tr.bdefs.push((d.reg.index, ins.imm[0] != 0)),
            _ => {}
        }
    }
    let body = tr.run()?;
    let mut code = String::new();
    let ps = v.pixel;
    // relative constant reads
    let (arr, n) = if ps { ("sm.pc", 32) } else { ("sm.vc", 256) };
    let _ = writeln!(code, "fn sm_{}c_rel(i: i32) -> vec4f {{", if ps { 'p' } else { 'v' });
    let _ = writeln!(code, "    if i < 0 || i >= {n} {{\n        return vec4f(0.0);\n    }}");
    for (i, b) in &tr.fdefs {
        let _ = writeln!(code, "    if i == {i} {{\n        return {};\n    }}", v4lit(*b));
    }
    let _ = writeln!(code, "    return {arr}[i];\n}}\n");
    if ps {
        for &(s, k) in &tr.samplers {
            let ty = match k {
                3 => "texture_cube<f32>",
                4 => "texture_3d<f32>",
                _ => "texture_2d<f32>",
            };
            let _ = writeln!(code, "@group(1) @binding({}) var sm_tex{s}: {ty};", 2 * s);
            let _ = writeln!(code, "@group(1) @binding({}) var sm_smp{s}: sampler;", 2 * s + 1);
        }
        let _ = writeln!(code, "\nfn ps_body(d0: vec4f, d1: vec4f, tc: array<vec4f, 8>) -> SmPsOut {{");
        for i in 0..tr.temps {
            let _ = writeln!(code, "    var r{i} = vec4f(0.0);");
        }
        for i in 0..8 {
            if tr.tc_used >> i & 1 != 0 {
                let _ = writeln!(code, "    var t{i} = tc[{i}];");
            }
        }
        let _ = writeln!(code, "    var o_c0 = vec4f(0.0);\n    var o_depth = vec4f(0.0);\n    var aL = 0;");
        code.push_str(&indent(&body));
        let color = if v.major == 1 { "r0" } else { "o_c0" };
        if v.major == 1 && tr.temps == 0 {
            let _ = writeln!(code, "    let r0 = vec4f(0.0);");
        }
        let _ = writeln!(code, "    return SmPsOut({color}, o_depth.x);\n}}");
    } else {
        let _ = writeln!(code, "fn vs_body(v: array<vec4f, 16>) -> SmVsOut {{");
        for i in 0..tr.temps {
            let _ = writeln!(code, "    var r{i} = vec4f(0.0);");
        }
        let _ = writeln!(code, "    var a0 = vec4i(0);\n    var aL = 0;");
        let _ = writeln!(code, "    var o_pos = vec4f(0.0);\n    var o_fog = vec4f(1.0);\n    var o_pts = vec4f(0.0);");
        let _ = writeln!(code, "    var o_d0 = vec4f(0.0);\n    var o_d1 = vec4f(0.0);");
        for i in 0..8 {
            let _ = writeln!(code, "    var o_t{i} = vec4f(0.0);");
        }
        code.push_str(&indent(&body));
        let _ = writeln!(
            code,
            "    return SmVsOut(o_pos, o_d0, o_d1, array<vec4f, 8>(o_t0, o_t1, o_t2, o_t3, o_t4, o_t5, o_t6, o_t7), o_fog.x, o_pts.x);\n}}"
        );
    }
    Ok(Translated {
        version: v,
        code,
        inputs: if ps { Vec::new() } else { sh.input_decls() },
        samplers: tr.samplers,
        writes_depth: tr.writes_depth,
        texcoords_used: tr.tc_used,
    })
}

fn indent(body: &str) -> String {
    let mut s = String::new();
    let mut depth = 1usize;
    for l in body.lines() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        if l.starts_with('}') {
            depth = depth.saturating_sub(1).max(1);
        }
        for _ in 0..depth {
            s.push_str("    ");
        }
        s.push_str(l);
        s.push('\n');
        if l.ends_with('{') {
            depth += 1;
        }
    }
    s
}
