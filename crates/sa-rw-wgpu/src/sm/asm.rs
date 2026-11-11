//! D3DX shader assembler syntax → D3D9 token stream.
//!
//! Accepts the subset of the `D3DXAssembleShader` language that shader models 1.x/2.x use:
//! version lines (`vs_1_1`, `vs_2_0`, `vs_2_x`, `ps_1_1`..`ps_1_4`, `ps_2_0`, `ps_2_x`, also the
//! 3_0 versions), arithmetic, texture and flow-control instructions, `dcl`/`def`/`defi`/`defb`,
//! instruction modifiers (`_sat _pp _centroid _x2 _x4 _x8 _d2 _d4 _d8`), the coissue `+` prefix,
//! predicates (`(p0.x) add …`), source modifiers (`-x`, `1-x`, `_bias`, `_bx2`, `_x2`, `_dz`/`_db`,
//! `_dw`/`_da`, `_abs`/`abs(x)`, `!x`), swizzles and write masks (`xyzw` or `rgba`) and relative
//! constant addressing (`c[a0.x + 12]`, `c[12 + a0.x]`, `c[3 + 5]`, `c[aL + 1]`).
//!
//! Encoding follows d3d9types.h: SM2+ instruction tokens carry the parameter count in bits
//! 24..27, SM1 ones do not; parameter tokens have bit 31 set; vs_1_1 relative addressing is the
//! bit 13 flag alone (`a0.x` implied), SM2+ appends an address-register token. It does not run
//! D3DX's validator: register ranges and per-version instruction availability are not checked.

use super::parse::{has_dst, src_count};
use super::{op, reg, srcmod, Version};

/// An assembly error at a 1-based source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsmError {
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for AsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

impl std::error::Error for AsmError {}

/// Assembles `src` into D3D9 shader tokens (version token first, end token last).
pub fn assemble(src: &str) -> Result<Vec<u32>, AsmError> {
    let mut out = Vec::new();
    let mut version: Option<Version> = None;
    let mut last = 0;
    for (n, raw) in src.lines().enumerate() {
        let line = n + 1;
        last = line;
        let text = strip_comment(raw).trim();
        if text.is_empty() {
            continue;
        }
        let res = match version {
            None => parse_version(text).map(|v| {
                version = Some(v);
                out.push(v.token());
            }),
            Some(v) => assemble_line(v, text, &mut out),
        };
        res.map_err(|msg| AsmError { line, msg })?;
    }
    if version.is_none() {
        return Err(AsmError { line: last.max(1), msg: "missing version line".into() });
    }
    out.push(0x0000_FFFF);
    Ok(out)
}

fn strip_comment(s: &str) -> &str {
    let mut end = s.len();
    if let Some(i) = s.find("//") {
        end = end.min(i);
    }
    if let Some(i) = s.find(';') {
        end = end.min(i);
    }
    &s[..end]
}

fn parse_version(text: &str) -> Result<Version, String> {
    let t = text.to_ascii_lowercase();
    let parts: Vec<&str> = t.split(['_', '.']).collect();
    let bad = || format!("expected a shader version, found `{text}`");
    let [kind, major, minor] = parts[..] else { return Err(bad()) };
    let pixel = match kind {
        "vs" => false,
        "ps" => true,
        _ => return Err(bad()),
    };
    let major: u8 = major.parse().map_err(|_| bad())?;
    let minor: u8 = match minor {
        "x" if major == 2 => 1,
        m => m.parse().map_err(|_| bad())?,
    };
    let ok = match (pixel, major) {
        (false, 1) => minor <= 1,
        (true, 1) => minor <= 4,
        (_, 2) => minor <= 1,
        (_, 3) => minor == 0,
        _ => false,
    };
    if !ok {
        return Err(format!("unsupported shader version `{text}`"));
    }
    Ok(Version { pixel, major, minor })
}

/// Register type field bits of a parameter token.
fn type_bits(ty: u8) -> u32 {
    ((ty as u32 & 7) << 28) | (((ty as u32 >> 3) & 3) << 11)
}

/// Relative address of a register reference: (address register type, component).
type RelAddr = (u8, u8);

/// A parsed register reference.
struct RegRef {
    ty: u8,
    index: u32,
    rel: Option<RelAddr>,
}

const MAX_INDEX: i64 = 0x7FF;

/// Parses a register at the start of `s` (lower case, no whitespace); returns it and the rest.
fn parse_reg(v: Version, s: &str) -> Result<(RegRef, &str), String> {
    let alpha = s.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(s.len());
    let name = &s[..alpha];
    let after = &s[alpha..];
    // registers without an index
    let fixed = match name {
        "opos" => Some((reg::RASTOUT, 0)),
        "ofog" => Some((reg::RASTOUT, 1)),
        "opts" => Some((reg::RASTOUT, 2)),
        "odepth" => Some((reg::DEPTHOUT, 0)),
        "al" => Some((reg::LOOP, 0)),
        "vpos" => Some((reg::MISCTYPE, 0)),
        "vface" => Some((reg::MISCTYPE, 1)),
        _ => None,
    };
    if let Some((ty, index)) = fixed {
        return Ok((RegRef { ty, index, rel: None }, after));
    }
    let ty = match name {
        "r" => reg::TEMP,
        "v" => reg::INPUT,
        "c" => reg::CONST,
        "a" if !v.pixel => reg::ADDR,
        "t" if v.pixel => reg::TEXTURE,
        "s" => reg::SAMPLER,
        "b" => reg::CONSTBOOL,
        "i" => reg::CONSTINT,
        "p" => reg::PREDICATE,
        "l" => reg::LABEL,
        "od" => reg::ATTROUT,
        "ot" => reg::TEXCRDOUT,
        "o" if v.major >= 3 => reg::TEXCRDOUT,
        "oc" => reg::COLOROUT,
        _ => return Err(format!("unknown register `{s}`")),
    };
    if let Some(inner) = after.strip_prefix('[') {
        let close = inner.find(']').ok_or_else(|| format!("missing `]` in `{s}`"))?;
        let (index, rel) = parse_index_expr(v, &inner[..close])?;
        return Ok((RegRef { ty, index, rel }, &inner[close + 1..]));
    }
    let digits = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
    if digits == 0 {
        return Err(format!("register `{s}` needs an index"));
    }
    let index: i64 = after[..digits].parse().map_err(|_| format!("bad register index in `{s}`"))?;
    if index > MAX_INDEX {
        return Err(format!("register index out of range in `{s}`"));
    }
    Ok((RegRef { ty, index: index as u32, rel: None }, &after[digits..]))
}

/// `a0.x + 12`, `12 + a0.x`, `3 + 5`, `aL`, … → (constant index, relative address).
fn parse_index_expr(v: Version, e: &str) -> Result<(u32, Option<RelAddr>), String> {
    let bad = || format!("bad index expression `[{e}]`");
    let mut sum: i64 = 0;
    let mut rel = None;
    let mut rest = e;
    let mut neg = false;
    let mut first = true;
    loop {
        if !first || rest.starts_with(['+', '-']) {
            match rest.as_bytes().first() {
                Some(b'+') => neg = false,
                Some(b'-') => neg = true,
                _ => return Err(bad()),
            }
            rest = &rest[1..];
        }
        first = false;
        let end = rest.find(['+', '-']).unwrap_or(rest.len());
        let term = &rest[..end];
        rest = &rest[end..];
        if term.is_empty() {
            return Err(bad());
        }
        if term.as_bytes()[0].is_ascii_digit() {
            let n: i64 = term.parse().map_err(|_| bad())?;
            sum += if neg { -n } else { n };
        } else {
            if neg || rel.is_some() {
                return Err(bad());
            }
            let (ty, comp_text) = if let Some(c) = term.strip_prefix("a0") {
                if v.pixel {
                    return Err(format!("`a0` is not a pixel shader register (`[{e}]`)"));
                }
                (reg::ADDR, c)
            } else if let Some(c) = term.strip_prefix("al") {
                (reg::LOOP, c)
            } else {
                return Err(bad());
            };
            let comp = match comp_text {
                "" => 0,
                c => {
                    let c = c.strip_prefix('.').ok_or_else(bad)?;
                    let mut ch = c.chars();
                    let (Some(x), None) = (ch.next(), ch.next()) else { return Err(bad()) };
                    comp_index(x).ok_or_else(bad)?
                }
            };
            if v.major < 2 && !(ty == reg::ADDR && comp == 0) {
                return Err(format!("only `a0.x` relative addressing exists in {v} (`[{e}]`)"));
            }
            rel = Some((ty, comp));
        }
        if rest.is_empty() {
            break;
        }
    }
    if !(0..=MAX_INDEX).contains(&sum) {
        return Err(format!("register index out of range in `[{e}]`"));
    }
    if v.major < 2 && v.pixel && rel.is_some() {
        return Err(format!("relative addressing does not exist in {v}"));
    }
    Ok((sum as u32, rel))
}

fn comp_index(c: char) -> Option<u8> {
    Some(match c {
        'x' | 'r' => 0,
        'y' | 'g' => 1,
        'z' | 'b' => 2,
        'w' | 'a' => 3,
        _ => return None,
    })
}

/// Lower-cases an operand and drops its whitespace.
fn normalize(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).map(|c| c.to_ascii_lowercase()).collect()
}

/// Destination register with write mask; returns the token without result modifiers.
fn parse_dst(v: Version, text: &str) -> Result<(u32, u8), String> {
    let (r, rest) = parse_reg(v, text)?;
    if r.rel.is_some() {
        return Err(format!("relative addressing of a destination is not supported (`{text}`)"));
    }
    let mask = if rest.is_empty() {
        0xF
    } else {
        let m = rest.strip_prefix('.').ok_or_else(|| format!("bad destination `{text}`"))?;
        let mut mask = 0u8;
        let mut prev: i8 = -1;
        for ch in m.chars() {
            let c = comp_index(ch).ok_or_else(|| format!("bad write mask in `{text}`"))?;
            if c as i8 <= prev {
                return Err(format!("write mask components out of order in `{text}`"));
            }
            prev = c as i8;
            mask |= 1 << c;
        }
        if mask == 0 {
            return Err(format!("empty write mask in `{text}`"));
        }
        mask
    };
    Ok((0x8000_0000 | type_bits(r.ty) | r.index | (mask as u32) << 16, r.ty))
}

/// Source parameter: the source token and, for SM2+ relative addressing, the address token.
fn parse_src(v: Version, text: &str) -> Result<(u32, Option<u32>), String> {
    let bad = || format!("bad source operand `{text}`");
    let mut s: String = text.to_string();
    let mut neg = false;
    let mut comp = false;
    let mut not = false;
    if let Some(r) = s.strip_prefix('-') {
        neg = true;
        s = r.to_string();
    } else if let Some(r) = s.strip_prefix("1-") {
        comp = true;
        s = r.to_string();
    } else if let Some(r) = s.strip_prefix('!') {
        not = true;
        s = r.to_string();
    }
    let mut abs = false;
    if let Some(r) = s.strip_prefix("abs(") {
        let close = r.rfind(')').ok_or_else(bad)?;
        abs = true;
        s = format!("{}{}", &r[..close], &r[close + 1..]);
    }
    let (r, mut rest) = parse_reg(v, &s)?;
    let mut swizzle: Option<u32> = None;
    let mut base: Option<&str> = None;
    while !rest.is_empty() {
        if let Some(t) = rest.strip_prefix('.') {
            let end = t.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(t.len());
            let letters = &t[..end];
            if swizzle.is_some() || letters.is_empty() || letters.len() > 4 {
                return Err(bad());
            }
            let comps: Vec<u8> = letters.chars().map(comp_index).collect::<Option<_>>().ok_or_else(bad)?;
            let mut sw = 0u32;
            for k in 0..4 {
                sw |= (comps[k.min(comps.len() - 1)] as u32) << (2 * k);
            }
            swizzle = Some(sw);
            rest = &t[end..];
        } else if let Some(t) = rest.strip_prefix('_') {
            let end = t.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(t.len());
            if base.is_some() {
                return Err(bad());
            }
            base = Some(&t[..end]);
            rest = &t[end..];
        } else {
            return Err(bad());
        }
    }
    if abs {
        if base.is_some() {
            return Err(bad());
        }
        base = Some("abs");
    }
    let modifier = match (base, neg, comp, not) {
        (None, false, false, false) => srcmod::NONE,
        (None, true, _, _) => srcmod::NEG,
        (None, _, true, _) => srcmod::COMP,
        (None, _, _, true) => srcmod::NOT,
        (Some(_), _, true, _) | (Some(_), _, _, true) => return Err(bad()),
        (Some("bias"), n, ..) => if n { srcmod::BIASNEG } else { srcmod::BIAS },
        (Some("bx2"), n, ..) => if n { srcmod::SIGNNEG } else { srcmod::SIGN },
        (Some("x2"), n, ..) => if n { srcmod::X2NEG } else { srcmod::X2 },
        (Some("abs"), n, ..) => if n { srcmod::ABSNEG } else { srcmod::ABS },
        (Some("dz" | "db"), false, ..) => srcmod::DZ,
        (Some("dw" | "da"), false, ..) => srcmod::DW,
        _ => return Err(bad()),
    };
    let mut tok = 0x8000_0000 | type_bits(r.ty) | r.index | swizzle.unwrap_or(0xE4) << 16 | (modifier as u32) << 24;
    let mut extra = None;
    if let Some((aty, c)) = r.rel {
        tok |= 1 << 13;
        if v.major >= 2 {
            let sw = if aty == reg::LOOP { 0xE4 } else { c as u32 * 0x55 };
            extra = Some(0x8000_0000 | type_bits(aty) | sw << 16);
        }
    }
    Ok((tok, extra))
}

const USAGES: [&str; 14] = [
    "position", "blendweight", "blendindices", "normal", "psize", "texcoord", "tangent", "binormal", "tessfactor",
    "positiont", "color", "fog", "depth", "sample",
];

/// Result-modifier bits of an instruction suffix (`sat`, `x2`, …) for a destination token.
fn dst_modifier(part: &str) -> Option<u32> {
    Some(match part {
        "sat" => 1 << 20,
        "pp" => 2 << 20,
        "centroid" => 4 << 20,
        "x2" => 1 << 24,
        "x4" => 2 << 24,
        "x8" => 3 << 24,
        "d2" => 15 << 24,
        "d4" => 14 << 24,
        "d8" => 13 << 24,
        _ => return None,
    })
}

fn comparison(part: &str) -> Option<u8> {
    Some(match part {
        "gt" => 1,
        "eq" => 2,
        "ge" => 3,
        "lt" => 4,
        "ne" => 5,
        "le" => 6,
        _ => return None,
    })
}

fn base_opcode(base: &str) -> Option<u16> {
    Some(match base {
        "nop" => op::NOP,
        "mov" => op::MOV,
        "add" => op::ADD,
        "sub" => op::SUB,
        "mad" => op::MAD,
        "mul" => op::MUL,
        "rcp" => op::RCP,
        "rsq" => op::RSQ,
        "dp3" => op::DP3,
        "dp4" => op::DP4,
        "min" => op::MIN,
        "max" => op::MAX,
        "slt" => op::SLT,
        "sge" => op::SGE,
        "exp" => op::EXP,
        "log" => op::LOG,
        "lit" => op::LIT,
        "dst" => op::DST,
        "lrp" => op::LRP,
        "frc" => op::FRC,
        "m4x4" => op::M4X4,
        "m4x3" => op::M4X3,
        "m3x4" => op::M3X4,
        "m3x3" => op::M3X3,
        "m3x2" => op::M3X2,
        "call" => op::CALL,
        "callnz" => op::CALLNZ,
        "loop" => op::LOOP,
        "ret" => op::RET,
        "endloop" => op::ENDLOOP,
        "label" => op::LABEL,
        "pow" => op::POW,
        "crs" => op::CRS,
        "sgn" => op::SGN,
        "abs" => op::ABS,
        "nrm" => op::NRM,
        "sincos" => op::SINCOS,
        "rep" => op::REP,
        "endrep" => op::ENDREP,
        "if" => op::IF,
        "else" => op::ELSE,
        "endif" => op::ENDIF,
        "break" => op::BREAK,
        "breakp" => op::BREAKP,
        "mova" => op::MOVA,
        "texcoord" | "texcrd" => op::TEXCOORD,
        "texkill" => op::TEXKILL,
        "tex" | "texld" | "texldp" | "texldb" => op::TEX,
        "texldd" => op::TEXLDD,
        "texldl" => op::TEXLDL,
        "texbem" => op::TEXBEM,
        "texbeml" => op::TEXBEML,
        "texreg2ar" => op::TEXREG2AR,
        "texreg2gb" => op::TEXREG2GB,
        "texreg2rgb" => op::TEXREG2RGB,
        "texm3x2pad" => op::TEXM3X2PAD,
        "texm3x2tex" => op::TEXM3X2TEX,
        "texm3x3pad" => op::TEXM3X3PAD,
        "texm3x3tex" => op::TEXM3X3TEX,
        "texm3x3spec" => op::TEXM3X3SPEC,
        "texm3x3vspec" => op::TEXM3X3VSPEC,
        "expp" => op::EXPP,
        "logp" => op::LOGP,
        "cnd" => op::CND,
        "texdp3tex" => op::TEXDP3TEX,
        "texm3x2depth" => op::TEXM3X2DEPTH,
        "texdp3" => op::TEXDP3,
        "texm3x3" => op::TEXM3X3,
        "texdepth" => op::TEXDEPTH,
        "cmp" => op::CMP,
        "bem" => op::BEM,
        "dp2add" => op::DP2ADD,
        "dsx" => op::DSX,
        "dsy" => op::DSY,
        "setp" => op::SETP,
        _ => return None,
    })
}

/// Instruction token: opcode, control, SM2+ length, coissue and predicate flags.
fn instr_token(v: Version, opcode: u16, control: u8, nparams: usize, coissue: bool, predicated: bool) -> u32 {
    let mut t = opcode as u32 | (control as u32) << 16;
    if v.major >= 2 {
        t |= (nparams as u32 & 0xF) << 24;
    }
    if coissue {
        t |= 1 << 30;
    }
    if predicated {
        t |= 1 << 28;
    }
    t
}

fn assemble_line(v: Version, text: &str, out: &mut Vec<u32>) -> Result<(), String> {
    let mut s = text;
    let coissue = match s.strip_prefix('+') {
        Some(r) => {
            s = r.trim_start();
            true
        }
        None => false,
    };
    let mut predicate = None;
    if let Some(r) = s.strip_prefix('(') {
        let close = r.find(')').ok_or_else(|| format!("missing `)` in `{text}`"))?;
        let p = parse_src(v, &normalize(&r[..close]))?;
        if p.1.is_some() {
            return Err(format!("bad predicate in `{text}`"));
        }
        predicate = Some(p.0);
        s = r[close + 1..].trim_start();
    }
    let split = s.find(char::is_whitespace).unwrap_or(s.len());
    let mn = s[..split].to_ascii_lowercase();
    let rest = s[split..].trim();
    let operands: Vec<String> = if rest.is_empty() { Vec::new() } else { rest.split(',').map(normalize).collect() };
    if operands.iter().any(String::is_empty) {
        return Err(format!("empty operand in `{text}`"));
    }
    let parts: Vec<&str> = mn.split('_').collect();
    let base = parts[0];
    let plain = |what: &str| -> Result<(), String> {
        if coissue || predicate.is_some() {
            return Err(format!("`{what}` takes no coissue or predicate"));
        }
        Ok(())
    };
    let want = |n: usize| -> Result<(), String> {
        if operands.len() != n {
            return Err(format!("`{mn}` expects {n} operands, found {}", operands.len()));
        }
        Ok(())
    };

    match base {
        "phase" if parts.len() == 1 => {
            plain("phase")?;
            want(0)?;
            out.push(0x0000_FFFD);
            return Ok(());
        }
        "dcl" => {
            plain("dcl")?;
            want(1)?;
            let (mut dst, ty) = parse_dst(v, &operands[0])?;
            let mut sampler: Option<u8> = None;
            let mut usage: Option<(u32, u32)> = None;
            for &p in &parts[1..] {
                if let Some(m) = dst_modifier(p) {
                    dst |= m;
                    continue;
                }
                if sampler.is_some() || usage.is_some() {
                    return Err(format!("bad dcl `{mn}`"));
                }
                match p {
                    "2d" => sampler = Some(2),
                    "cube" => sampler = Some(3),
                    "volume" => sampler = Some(4),
                    _ => {
                        let d = p.find(|c: char| c.is_ascii_digit()).unwrap_or(p.len());
                        let u = USAGES.iter().position(|&u| u == &p[..d]).ok_or_else(|| format!("unknown dcl usage `{p}`"))?;
                        let idx: u32 = if d == p.len() { 0 } else { p[d..].parse().map_err(|_| format!("bad usage index `{p}`"))? };
                        if idx > 15 {
                            return Err(format!("usage index out of range `{p}`"));
                        }
                        usage = Some((u as u32, idx));
                    }
                }
            }
            let decl = if ty == reg::SAMPLER {
                if usage.is_some() {
                    return Err(format!("a sampler takes a texture type, not `{mn}`"));
                }
                0x8000_0000 | (sampler.unwrap_or(0) as u32) << 27
            } else {
                if sampler.is_some() {
                    return Err(format!("`{mn}` needs a sampler register"));
                }
                let (u, i) = usage.unwrap_or((0, 0));
                0x8000_0000 | u | i << 16
            };
            out.extend([instr_token(v, op::DCL, 0, 2, false, false), decl, dst]);
            return Ok(());
        }
        "def" | "defi" | "defb" => {
            plain(base)?;
            if parts.len() != 1 {
                return Err(format!("unknown instruction `{mn}`"));
            }
            let (opcode, n) = match base {
                "def" => (op::DEF, 5),
                "defi" => (op::DEFI, 5),
                _ => (op::DEFB, 2),
            };
            want(n)?;
            let (dst, _) = parse_dst(v, &operands[0])?;
            out.extend([instr_token(v, opcode, 0, n, false, false), dst]);
            for o in &operands[1..] {
                let bits = match opcode {
                    op::DEF => parse_float(o)?.to_bits(),
                    op::DEFI => o.parse::<i32>().map_err(|_| format!("bad integer `{o}`"))? as u32,
                    _ => match o.as_str() {
                        "true" | "1" => 1,
                        "false" | "0" => 0,
                        _ => return Err(format!("bad boolean `{o}`")),
                    },
                };
                out.push(bits);
            }
            return Ok(());
        }
        _ => {}
    }

    let mut opcode = base_opcode(base).ok_or_else(|| format!("unknown instruction `{mn}`"))?;
    let mut control: u8 = match base {
        "texldp" => 1,
        "texldb" => 2,
        _ => 0,
    };
    let mut result_mods = 0u32;
    let mut has_cmp = false;
    for &p in &parts[1..] {
        if let Some(c) = comparison(p).filter(|_| matches!(opcode, op::IF | op::BREAK | op::SETP) && !has_cmp) {
            has_cmp = true;
            control = c;
            opcode = match opcode {
                op::IF => op::IFC,
                op::BREAK => op::BREAKC,
                o => o,
            };
        } else if let Some(m) = dst_modifier(p) {
            result_mods |= m;
        } else {
            return Err(format!("unknown instruction `{mn}`"));
        }
    }
    if opcode == op::SETP && !has_cmp {
        return Err("`setp` needs a comparison (`setp_gt` …)".into());
    }
    let dst = has_dst(opcode);
    let nsrc = src_count(opcode, v).ok_or_else(|| format!("unknown instruction `{mn}`"))?;
    want(nsrc + dst as usize)?;
    let mut params = Vec::new();
    let mut ops = operands.iter();
    if dst {
        let (t, _) = parse_dst(v, ops.next().expect("counted"))?;
        params.push(t | result_mods);
    } else if result_mods != 0 {
        return Err(format!("`{mn}`: result modifiers need a destination"));
    }
    if let Some(p) = predicate {
        params.push(p);
    }
    for o in ops {
        let (t, rel) = parse_src(v, o)?;
        params.push(t);
        params.extend(rel);
    }
    out.push(instr_token(v, opcode, control, params.len(), coissue, predicate.is_some()));
    out.extend(params);
    Ok(())
}

fn parse_float(s: &str) -> Result<f32, String> {
    s.parse::<f32>()
        .or_else(|_| s.strip_suffix('f').ok_or(()).and_then(|t| t.parse::<f32>().map_err(|_| ())))
        .map_err(|_| format!("bad number `{s}`"))
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse;
    use super::super::{op, reg, srcmod, DclKind, Reg, Rel};
    use super::*;

    /// Assembles `body` after `version` and returns the tokens between version and end.
    fn body(version: &str, body: &str) -> Vec<u32> {
        let t = assemble(&format!("{version}\n{body}\n")).unwrap_or_else(|e| panic!("{e}: {body}"));
        assert_eq!(*t.last().unwrap(), 0x0000_FFFF);
        t[1..t.len() - 1].to_vec()
    }

    fn round_trip(src: &str) -> Vec<u32> {
        let t = assemble(src).unwrap_or_else(|e| panic!("{e}"));
        let text = parse(&t).unwrap().to_string();
        let again = assemble(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert_eq!(again, t, "round trip of\n{text}");
        t
    }

    #[test]
    fn version_tokens() {
        for (v, t) in [
            ("vs_1_1", 0xFFFE_0101),
            ("vs_2_0", 0xFFFE_0200),
            ("vs_2_x", 0xFFFE_0201),
            ("ps_1_1", 0xFFFF_0101),
            ("ps_1_2", 0xFFFF_0102),
            ("ps_1_3", 0xFFFF_0103),
            ("ps_1_4", 0xFFFF_0104),
            ("ps_2_0", 0xFFFF_0200),
            ("ps_2_x", 0xFFFF_0201),
            ("VS.1.1", 0xFFFE_0101),
        ] {
            assert_eq!(assemble(&format!("{v}\n")).unwrap(), vec![t, 0xFFFF], "{v}");
        }
        assert!(assemble("vs_4_0\n").is_err());
        assert!(assemble("// nothing\n").is_err());
    }

    #[test]
    fn sm1_vertex_tokens() {
        let v = "vs_1_1";
        assert_eq!(body(v, "m4x4 oPos.xyzw, v0, c[0]"), [0x14, 0xC00F_0000, 0x90E4_0000, 0xA0E4_0000]);
        assert_eq!(body(v, "m4x4 opos, v0, c0"), [0x14, 0xC00F_0000, 0x90E4_0000, 0xA0E4_0000]);
        assert_eq!(body(v, "m4x3 r5.xyz, r2, c[a0.x + 12]"), [0x15, 0x8007_0005, 0x80E4_0002, 0xA0E4_200C]);
        assert_eq!(body(v, "m4x3 r5.xyz, r2, c[12 + a0.x]"), [0x15, 0x8007_0005, 0x80E4_0002, 0xA0E4_200C]);
        assert_eq!(body(v, "mov r0, c[a0.x]"), [0x01, 0x800F_0000, 0xA0E4_2000]);
        assert_eq!(body(v, "max r6.xyz, c[5].w, -r6.xyz"), [0x0B, 0x8007_0006, 0xA0FF_0005, 0x81A4_0006]);
        assert_eq!(body(v, "max r6.x, c[5].wwww, -r6.x"), [0x0B, 0x8001_0006, 0xA0FF_0005, 0x8100_0006]);
        assert_eq!(body(v, "add r7.xyz, -c[1 + 5].xyz, r3.xyz"), [0x02, 0x8007_0007, 0xA1A4_0006, 0x80A4_0003]);
        assert_eq!(body(v, "rcp oFog, r0.w"), [0x06, 0xC00F_0001, 0x80FF_0000]);
        assert_eq!(body(v, "mov oD0, v4"), [0x01, 0xD00F_0000, 0x90E4_0004]);
        assert_eq!(body(v, "mov oT1.xy, v6"), [0x01, 0xE003_0001, 0x90E4_0006]);
        assert_eq!(body(v, "mov a0.x, v2.x"), [0x01, 0xB001_0000, 0x9000_0002]);
        assert_eq!(
            body(v, "mad r2, r4.yzxw, r5.zxyw, -r2"),
            [0x04, 0x800F_0002, 0x80C9_0004, 0x80D2_0005, 0x81E4_0002]
        );
        assert_eq!(body(v, "dcl_position0 v0"), [0x1F, 0x8000_0000, 0x900F_0000]);
        assert_eq!(body(v, "dcl_texcoord1 v6"), [0x1F, 0x8001_0005, 0x900F_0006]);
        assert_eq!(body(v, "dcl_position1 v14"), [0x1F, 0x8001_0000, 0x900F_000E]);
        assert_eq!(body(v, "dcl_blendindices0 v2.xy"), [0x1F, 0x8000_0002, 0x9003_0002]);
        assert_eq!(body(v, "def c3, 1.0, 0.5, 0, -2"), [0x51, 0xA00F_0003, 0x3F80_0000, 0x3F00_0000, 0, 0xC000_0000]);
        // vs_1_1 has only a0.x
        assert_eq!(assemble("vs_1_1\nmov r0, c[a0.y + 1]\n").unwrap_err().line, 2);
    }

    #[test]
    fn sm2_vertex_tokens() {
        let v = "vs_2_0";
        assert_eq!(
            body(v, "m4x3 r5.xyz, v0, c[a0.y + 12]"),
            [0x0400_0015, 0x8007_0005, 0x90E4_0000, 0xA0E4_200C, 0xB055_0000]
        );
        assert_eq!(body(v, "mova a0.x, v2.x"), [0x0200_002E, 0xB001_0000, 0x9000_0002]);
        assert_eq!(body(v, "mova a0.xyzw, v2"), [0x0200_002E, 0xB00F_0000, 0x90E4_0002]);
        assert_eq!(
            body(v, "mad r8, v1.y, c[a0.z + 12], r8"),
            [0x0500_0004, 0x800F_0008, 0x9055_0001, 0xA0E4_200C, 0xB0AA_0000, 0x80E4_0008]
        );
        assert_eq!(body(v, "dcl_position v0"), [0x0200_001F, 0x8000_0000, 0x900F_0000]);
        assert_eq!(body(v, "dcl_tangent0 v13"), [0x0200_001F, 0x8000_0006, 0x900F_000D]);
        assert_eq!(body(v, "if_gt r0.x, c1.y"), [0x0201_0029, 0x8000_0000, 0xA055_0001]);
        assert_eq!(body(v, "if b0"), [0x0100_0028, 0xE0E4_0800]);
        assert_eq!(body(v, "if !b3"), [0x0100_0028, 0xEDE4_0803]);
        assert_eq!(body(v, "loop aL, i0"), [0x0200_001B, 0xF0E4_0800, 0xF0E4_0000]);
        assert_eq!(body(v, "rep i1"), [0x0100_0026, 0xF0E4_0001]);
        assert_eq!(body(v, "break_ne r0.x, r1.x"), [0x0205_002D, 0x8000_0000, 0x8000_0001]);
        assert_eq!(body(v, "call l2"), [0x0100_0019, 0xA0E4_1002]);
        assert_eq!(body(v, "label l2"), [0x0100_001E, 0xA0E4_1002]);
        assert_eq!(body(v, "callnz l0, b1"), [0x0200_001A, 0xA0E4_1000, 0xE0E4_0801]);
        assert_eq!(body(v, "mov r0, c[aL + 3]"), [0x0300_0001, 0x800F_0000, 0xA0E4_2003, 0xF0E4_0800]);
        assert_eq!(body(v, "def c0, 1.0, 0.5, 0, -2"), [0x0500_0051, 0xA00F_0000, 0x3F80_0000, 0x3F00_0000, 0, 0xC000_0000]);
        assert_eq!(body(v, "defi i0, 3, 0, 1, -1"), [0x0500_0030, 0xF00F_0000, 3, 0, 1, 0xFFFF_FFFF]);
        assert_eq!(body(v, "defb b0, true"), [0x0200_002F, 0xE00F_0800, 1]);
        assert_eq!(body(v, "sincos r0.xy, r1.x, c0, c1"), [0x0400_0025, 0x8003_0000, 0x8000_0001, 0xA0E4_0000, 0xA0E4_0001]);
        assert_eq!(
            body("vs_2_x", "(p0.x) add r0, r1, r2"),
            [0x1400_0002, 0x800F_0000, 0xB000_1000, 0x80E4_0001, 0x80E4_0002]
        );
        assert_eq!(body("vs_2_x", "setp_lt p0, r0, c0"), [0x0304_005E, 0xB00F_1000, 0x80E4_0000, 0xA0E4_0000]);
        assert_eq!(body(v, "nop\nret\nendrep\nendloop\nelse\nendif\nbreak"), [0, 0x1C, 0x27, 0x1D, 0x2A, 0x2B, 0x2C]);
    }

    #[test]
    fn pixel_tokens() {
        assert_eq!(body("ps_1_1", "tex t0"), [0x42, 0xB00F_0000]);
        assert_eq!(body("ps_1_1", "texcoord t1"), [0x40, 0xB00F_0001]);
        assert_eq!(body("ps_1_1", "texkill t2"), [0x41, 0xB00F_0002]);
        assert_eq!(body("ps_1_1", "+mov r0.a, 1-t0.b"), [0x4000_0001, 0x8008_0000, 0xB6AA_0000]);
        assert_eq!(body("ps_1_1", "mul_sat_x2 r0, r1_bx2, v0"), [0x05, 0x811F_0000, 0x84E4_0001, 0x90E4_0000]);
        assert_eq!(body("ps_1_1", "mul_d2 r0, -r1_bias, -v0_bx2"), [0x05, 0x8F0F_0000, 0x83E4_0001, 0x95E4_0000]);
        assert_eq!(body("ps_1_1", "texm3x3spec t3, t0, c0"), [0x4C, 0xB00F_0003, 0xB0E4_0000, 0xA0E4_0000]);
        assert_eq!(body("ps_1_1", "texbem t1, t0"), [0x43, 0xB00F_0001, 0xB0E4_0000]);
        assert_eq!(body("ps_1_1", "cnd r0, r0.a, r1, r2"), [0x50, 0x800F_0000, 0x80FF_0000, 0x80E4_0001, 0x80E4_0002]);
        assert_eq!(body("ps_1_1", "def c0, 1, 0, 0, 1"), [0x51, 0xA00F_0000, 0x3F80_0000, 0, 0, 0x3F80_0000]);
        assert_eq!(body("ps_1_4", "texcrd r0.rgb, t0.xyz"), [0x40, 0x8007_0000, 0xB0A4_0000]);
        assert_eq!(body("ps_1_4", "texld r1, t0_dz"), [0x42, 0x800F_0001, 0xB9E4_0000]);
        assert_eq!(body("ps_1_4", "phase"), [0xFFFD]);
        assert_eq!(body("ps_1_4", "bem r0.rg, r1, r2"), [0x59, 0x8003_0000, 0x80E4_0001, 0x80E4_0002]);
        assert_eq!(body("ps_2_0", "dcl_2d s0"), [0x0200_001F, 0x9000_0000, 0xA00F_0800]);
        assert_eq!(body("ps_2_0", "dcl_cube s1"), [0x0200_001F, 0x9800_0000, 0xA00F_0801]);
        assert_eq!(body("ps_2_0", "dcl_volume s2"), [0x0200_001F, 0xA000_0000, 0xA00F_0802]);
        assert_eq!(body("ps_2_0", "dcl t0.xy"), [0x0200_001F, 0x8000_0000, 0xB003_0000]);
        assert_eq!(body("ps_2_0", "dcl_pp v1"), [0x0200_001F, 0x8000_0000, 0x902F_0001]);
        assert_eq!(body("ps_2_0", "texld r0, t0, s0"), [0x0300_0042, 0x800F_0000, 0xB0E4_0000, 0xA0E4_0800]);
        assert_eq!(body("ps_2_0", "texldp r0, t0, s0"), [0x0301_0042, 0x800F_0000, 0xB0E4_0000, 0xA0E4_0800]);
        assert_eq!(body("ps_2_0", "texldb_pp r0, t0, s0"), [0x0302_0042, 0x802F_0000, 0xB0E4_0000, 0xA0E4_0800]);
        assert_eq!(body("ps_2_0", "mov_sat_pp oC0, -r0_abs"), [0x0200_0001, 0x803F_0800, 0x8CE4_0000]);
        assert_eq!(body("ps_2_0", "mov oC0, abs(r0).x"), [0x0200_0001, 0x800F_0800, 0x8B00_0000]);
        assert_eq!(body("ps_2_0", "mov oDepth, r0.x"), [0x0200_0001, 0x900F_0800, 0x8000_0000]);
        assert_eq!(
            body("ps_2_0", "cmp r0, -r1, c0, c1"),
            [0x0400_0058, 0x800F_0000, 0x81E4_0001, 0xA0E4_0000, 0xA0E4_0001]
        );
    }

    #[test]
    fn errors_name_the_line() {
        let e = assemble("vs_1_1\n\n// x\nfoo r0, r1\n").unwrap_err();
        assert_eq!(e.line, 4);
        assert!(e.msg.contains("foo"), "{e}");
        assert_eq!(assemble("vs_1_1\nmov r0, q1\n").unwrap_err().line, 2);
        assert_eq!(assemble("vs_1_1\nmov r0\n").unwrap_err().line, 2);
        assert_eq!(assemble("vs_1_1\nmov r0.yx, r1\n").unwrap_err().line, 2);
        assert_eq!(assemble("ps_1_1\nmov r0, a0\n").unwrap_err().line, 2);
        assert_eq!(assemble("vs_1_1\nmov r0, t0\n").unwrap_err().line, 2);
        assert_eq!(assemble("vs_2_0\nmov_sat_foo r0, r1\n").unwrap_err().line, 2);
    }

    /// The shapes RenderWare's vertex-shader generator prints (with its `%u`s filled in).
    const RW_FORMS: &str = "\
dcl_position0 v0
dcl_blendweight0 v1
dcl_blendindices0 v2
dcl_normal0 v3
dcl_color0 v4
dcl_texcoord0 v5
dcl_texcoord1 v6
dcl_tangent0 v13
dcl_position1 v14
dcl_normal1 v15
dcl_texcoord2 v7
m4x4 oPos.xyzw, v0, c[0]
m4x4 oPos.xyzw, r3, c[0]
m4x3 r5.xyz, r2, c[a0.x + 12]
m4x3 r5.xyz, v0, c[a0.y + 12]
m3x3 r4.xyz, v13, c[a0.x + 9]
m3x3 r4.xyz, v13, r8
m4x3 r3.xyz, v0, r8
mov a0.x, v2.x
mova a0.x, v2.x
mova a0.xyzw, v2
mov a0.x, v2.z
mul r8, v1.x, c[a0.x + 12]
mad r8, v1.y, c[a0.z + 12], r8
mul r3.xyz, v1.x, r5.xyz
mad r3.xyz, v1.y, r5.xyz, r3.xyz
mov r3.w, v0.w
dp4 r0.w, r3, c[3]
mul r0.w, r0.w, c[4].x
mul r0.w, r0.w, r0.w
expp r0.w, r0.w
rcp oFog, r0.w
mad oFog, r0.w, c[4].x, c[4].y
dp3 r5.w, r4, r4
rsq r5.w, r5.w
mul r5.xyz, r4, r5.w
mov r11, c[4]
add r11, v4, c[4]
mov oD0, v4
mov oD0, c[4]
add oD0, v4, c[4]
mul oD0, r11, c[9]
dp3 r6.x, c[10], r5
max r6.xyz, c[5].w, -r6.xyz
max r6.x, c[5].wwww, -r6.x
mad r11.xyz, c[11].xyz, r6.x, r11.xyz
mad oD0, c[12], r6.x, r11
add r7.xyz, -c[13].xyz, r3.xyz
add r7.xyz, -c[1 + 5].xyz, r3.xyz
dp3 r1.x, r7, r7
rsq r1.x, r1.x
mul r7.xyz, r7.xyz, r1.x
mad r9.x, r7.w, c[14].x, c[14].y
mul r6.xy, r6.xy, r8.xy
add r8.xy, -c[15].xy, r8.xy
max r8.xy, c[5].xy, r1.xy
slt r4.w, v13.w, v13.w
mad r2, r4.yzxw, r5.zxyw, -r2
mul r2, r4.zxyw, r5.yzxw
mov r5.w, v3.w
mad r9.w, r9.w, c[3 + 5].x, c[3 + 5].y
dp3 r9.w, r7, c[2 + 5]
max r9.w, r9.w, c[5].w
rcp r7.w, r7.w
mad oD0.xyz, r1, c[5].z, c[5].z
mad oT2.xyz, r1, c[5].w, c[5].w
dp3 oT2.z, r7, c[20]
dp3 oT3.x, c[21], r4
mov oT0.xy, v5
mov oT1.xy, v6
mov oT2.xy, v7
dp4 oT0.x, v5, c[22]
add oT1.xy, r9.xy, v5.xy
mad r7, r7.w, r5, -r7
add r7.w, r7.w, r7.w
dp3 r7.w, r7, r5
add r7.xyz, c[23].xyz, -r3.xyz
";

    #[test]
    fn rw_forms_assemble_and_decode() {
        for version in ["vs_1_1", "vs_2_0", "vs_2_x"] {
            let sm1 = version == "vs_1_1";
            let lines: Vec<&str> =
                RW_FORMS.lines().filter(|l| !sm1 || !(l.contains("a0.y") || l.contains("a0.z") || l.contains("mova"))).collect();
            let src = format!("{version}\n{}\n", lines.join("\n"));
            let tokens = round_trip(&src);
            let s = parse(&tokens).unwrap();
            assert_eq!(s.instrs.len(), lines.len(), "{version}");
            // spot checks
            let find = |l: &str| &s.instrs[lines.iter().position(|x| *x == l).unwrap()];
            let i = find("m4x3 r5.xyz, r2, c[a0.x + 12]");
            assert_eq!(i.opcode, op::M4X3);
            assert_eq!(i.dst.unwrap().mask, 7);
            assert_eq!(i.src[1].reg, Reg { ty: reg::CONST, index: 12 });
            assert_eq!(i.src[1].rel, Some(Rel { reg: Reg { ty: reg::ADDR, index: 0 }, comp: 0 }));
            let i = find("max r6.xyz, c[5].w, -r6.xyz");
            assert_eq!(i.src[0].swizzle, [3; 4]);
            assert_eq!(i.src[1].swizzle, [0, 1, 2, 2]);
            assert_eq!(i.src[1].modifier, srcmod::NEG);
            assert_eq!(find("max r6.x, c[5].wwww, -r6.x").src[0].swizzle, [3; 4]);
            let i = find("add r7.xyz, -c[1 + 5].xyz, r3.xyz");
            assert_eq!(i.src[0].reg.index, 6);
            assert_eq!(i.src[0].rel, None);
            assert_eq!(i.src[0].modifier, srcmod::NEG);
            assert_eq!(find("mad r9.w, r9.w, c[3 + 5].x, c[3 + 5].y").src[2].reg.index, 8);
            let i = find("rcp oFog, r0.w");
            assert_eq!(i.dst.unwrap().reg, Reg { ty: reg::RASTOUT, index: 1 });
            let i = find("mad oT2.xyz, r1, c[5].w, c[5].w");
            assert_eq!(i.dst.unwrap().reg, Reg { ty: reg::TEXCRDOUT, index: 2 });
            assert_eq!(i.dst.unwrap().mask, 7);
            let i = find("mul r2, r4.zxyw, r5.yzxw");
            assert_eq!(i.src[0].swizzle, [2, 0, 1, 3]);
            assert_eq!(i.src[1].swizzle, [1, 2, 0, 3]);
            assert_eq!(s.input_decls()[8], (14, 0, 1));
            if !sm1 {
                let i = find("mad r8, v1.y, c[a0.z + 12], r8");
                assert_eq!(i.src[1].rel, Some(Rel { reg: Reg { ty: reg::ADDR, index: 0 }, comp: 2 }));
                assert_eq!(find("mova a0.xyzw, v2").opcode, op::MOVA);
            }
        }
    }

    #[test]
    fn round_trips() {
        round_trip(
            "ps_1_1
def c0, 1.0, 0.25, -0.0, 1e-7
tex t0
tex t1
texcoord t2
mul r0, t0, v0
+mov r0.a, 1-t1.b
mul_x2 r1.rgb, r0, t1_bx2
+add_sat r1.a, -r0_bias.a, c0
lrp r0, v1.a, r1, 1-r0
cnd r0, r0.a, r1, c0.z
mad_d2 r0, -t2_x2, c0, v0_bx2
",
        );
        round_trip(
            "ps_1_4
def c1, 0.5, 0.5, 0.5, 1
texcrd r0.rgb, t0.xyz
texld r1, t1
phase
texld r2, r0
mul_sat r0, r2, r1_bx2
",
        );
        let t = round_trip(
            "ps_2_0
dcl_2d s0
dcl_cube s1
dcl t0.xy
dcl t1
dcl v0
def c0, 0.5, 1.0, 2.0, 0.0
texldp r0, t0, s0
texld r1, t1, s1
texldb_pp r2, t0, s0
mul_sat r0, r0, v0
cmp r0, -r1.x, c0, r0
mov oC0, r0
",
        );
        let s = parse(&t).unwrap();
        assert_eq!(s.sampler_decls(), vec![(0, 2), (1, 3)]);
        assert_eq!(s.instrs.iter().find(|i| i.opcode == op::TEX).unwrap().control, 1);
        let t = round_trip(
            "vs_2_0
dcl_position v0
dcl_blendindices v2
defi i0, 4, 0, 1, 0
defb b0, true
mova a0.y, v2.x
mov r0, c0
rep i0
add r0, r0, c[a0.y + 12]
endrep
if b0
mul r0, r0, c1
else
mul r0, r0, c2
endif
if_lt r0.x, c3.y
mov r0, -c[a0.y]
endif
loop aL, i0
add r0, r0, c[aL + 20]
break_ge r0.w, c3.w
endloop
call l1
m4x4 oPos, v0, c4
ret
label l1
sincos r1.xy, r0.x, c8, c9
mov oD0, r1
ret
",
        );
        let s = parse(&t).unwrap();
        let rel: Vec<_> = s.instrs.iter().flat_map(|i| i.src.iter()).filter_map(|s| s.rel).collect();
        assert_eq!(rel[0], Rel { reg: Reg { ty: reg::ADDR, index: 0 }, comp: 1 });
        assert_eq!(rel[2].reg.ty, reg::LOOP);
        // predicated vs_2_x
        round_trip("vs_2_x\nsetp_gt p0, r0, c0\n(p0.x) add r0, r1, r2\n(!p0.y) mov r1, c0\n");
    }

    #[test]
    fn dcl_kinds_decode() {
        let t = assemble("vs_1_1\ndcl_normal1 v15\ndcl_texcoord7 v9.xy\n").unwrap();
        let s = parse(&t).unwrap();
        assert_eq!(s.input_decls(), vec![(15, 3, 1), (9, 5, 7)]);
        assert_eq!(s.instrs[1].dst.unwrap().mask, 3);
        let t = assemble("ps_2_0\ndcl_volume s3\n").unwrap();
        assert_eq!(parse(&t).unwrap().instrs[0].dcl, Some(DclKind::Sampler(4)));
    }

    #[test]
    fn comments_and_case() {
        let a = assemble("; header\nVS_1_1 // v\n\n  M4X4 OPOS, V0, C[A0.X + 4] ; c\n").unwrap();
        let b = assemble("vs_1_1\nm4x4 oPos, v0, c[a0.x + 4]\n").unwrap();
        assert_eq!(a, b);
        assert_eq!(a, [0xFFFE_0101, 0x14, 0xC00F_0000, 0x90E4_0000, 0xA0E4_2004, 0xFFFF]);
    }
}
