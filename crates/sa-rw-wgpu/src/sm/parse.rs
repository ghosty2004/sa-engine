//! Token stream → [`Shader`], and [`Shader`] → assembly text (`Display`).

use std::fmt;

use super::{op, reg, srcmod, DclKind, Dst, Instr, Reg, Rel, Shader, Src, Version};

/// Why a token stream is not a shader this decoder accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Whether an opcode writes a destination parameter.
pub fn has_dst(opcode: u16) -> bool {
    !matches!(
        opcode,
        op::NOP
            | op::CALL
            | op::CALLNZ
            | op::LOOP
            | op::RET
            | op::ENDLOOP
            | op::LABEL
            | op::REP
            | op::ENDREP
            | op::IF
            | op::IFC
            | op::ELSE
            | op::ENDIF
            | op::BREAK
            | op::BREAKC
            | op::BREAKP
            | op::PHASE
    )
}

/// Number of source parameters of an opcode in shader `v` (SM1 streams carry no lengths).
pub fn src_count(opcode: u16, v: Version) -> Option<usize> {
    Some(match opcode {
        op::NOP | op::RET | op::ENDLOOP | op::ENDREP | op::ELSE | op::ENDIF | op::BREAK | op::PHASE => 0,
        op::MOV | op::RCP | op::RSQ | op::EXP | op::LOG | op::LIT | op::FRC | op::ABS | op::NRM | op::MOVA | op::EXPP | op::LOGP => 1,
        op::DSX | op::DSY => 1,
        op::ADD | op::SUB | op::MUL | op::DP3 | op::DP4 | op::MIN | op::MAX | op::SLT | op::SGE | op::DST | op::POW | op::CRS => 2,
        op::M4X4 | op::M4X3 | op::M3X4 | op::M3X3 | op::M3X2 | op::BEM | op::SETP | op::TEXLDL => 2,
        op::MAD | op::LRP | op::CND | op::CMP | op::DP2ADD | op::SGN => 3,
        op::TEXLDD => 4,
        op::SINCOS => {
            if v.major >= 3 {
                1
            } else {
                3
            }
        }
        op::CALL | op::LABEL | op::REP | op::IF | op::BREAKP => 1,
        op::CALLNZ | op::LOOP | op::IFC | op::BREAKC => 2,
        op::TEXCOORD | op::TEX => {
            if v.major >= 2 {
                if opcode == op::TEX { 2 } else { 0 }
            } else if v.minor >= 4 {
                1
            } else {
                0
            }
        }
        op::TEXKILL | op::TEXDEPTH => 0,
        op::TEXBEM | op::TEXBEML | op::TEXREG2AR | op::TEXREG2GB | op::TEXREG2RGB | op::TEXM3X2PAD | op::TEXM3X2TEX => 1,
        op::TEXM3X3PAD | op::TEXM3X3TEX | op::TEXM3X3VSPEC | op::TEXDP3TEX | op::TEXM3X2DEPTH | op::TEXDP3 | op::TEXM3X3 => 1,
        op::TEXM3X3SPEC => 2,
        _ => return None,
    })
}

fn reg_of(t: u32) -> Reg {
    Reg { ty: (((t >> 28) & 7) | ((t >> 8) & 0x18)) as u8, index: t & 0x7FF }
}

fn decode_dst(t: u32) -> Dst {
    let shift = ((t >> 24) & 0xF) as i8;
    Dst {
        reg: reg_of(t),
        mask: ((t >> 16) & 0xF) as u8,
        saturate: t & (1 << 20) != 0,
        partial: t & (2 << 20) != 0,
        centroid: t & (4 << 20) != 0,
        shift: if shift >= 8 { shift - 16 } else { shift },
        rel: None,
    }
}

fn decode_src(t: u32) -> Src {
    let sw = (t >> 16) & 0xFF;
    Src {
        reg: reg_of(t),
        swizzle: [0, 1, 2, 3].map(|i| ((sw >> (2 * i)) & 3) as u8),
        modifier: ((t >> 24) & 0xF) as u8,
        rel: None,
    }
}

/// Decodes a token stream (version token first). Trailing tokens after `end` are ignored; a
/// stream without `end` is an error.
pub fn parse(tokens: &[u32]) -> Result<Shader, ParseError> {
    let err = |m: String| Err(ParseError(m));
    let Some(&vt) = tokens.first() else { return err("empty token stream".into()) };
    let Some(version) = Version::from_token(vt) else { return err(format!("bad version token {vt:#010x}")) };
    if version.major > 3 || version.major == 0 {
        return err(format!("unsupported shader version {version}"));
    }
    let sm1 = version.major < 2;
    let mut instrs = Vec::new();
    let mut i = 1usize;
    loop {
        let Some(&t) = tokens.get(i) else { return err("token stream without end token".into()) };
        i += 1;
        let opcode = (t & 0xFFFF) as u16;
        match opcode {
            op::END => break,
            op::COMMENT => {
                i += ((t >> 16) & 0x7FFF) as usize;
                continue;
            }
            _ => {}
        }
        let at = i - 1;
        // parameter tokens of this instruction
        let params: &[u32] = if opcode == op::PHASE {
            &[]
        } else if !sm1 {
            let n = ((t >> 24) & 0xF) as usize;
            let Some(p) = tokens.get(i..i + n) else { return err(format!("instruction at token {at} runs past the stream")) };
            p
        } else {
            let n = match opcode {
                op::DEF => 5,
                op::DCL => 2,
                _ => {
                    // SM1: parameters are the following tokens with bit 31 set (no lengths)
                    let mut n = 0;
                    while tokens.get(i + n).is_some_and(|&p| p & 0x8000_0000 != 0) {
                        n += 1;
                    }
                    n
                }
            };
            let Some(p) = tokens.get(i..i + n) else { return err(format!("instruction at token {at} runs past the stream")) };
            p
        };
        i += params.len();
        let mut ins = Instr::new(opcode);
        ins.control = ((t >> 16) & 0xFF) as u8;
        ins.coissue = t & (1 << 30) != 0;
        let predicated = t & (1 << 28) != 0;
        let mut p = params.iter().copied();
        match opcode {
            op::DCL => {
                let (Some(d), Some(r)) = (p.next(), p.next()) else { return err(format!("dcl at token {at}: missing parameters")) };
                let dst = decode_dst(r);
                ins.dcl = Some(if dst.reg.ty == reg::SAMPLER {
                    DclKind::Sampler(((d >> 27) & 0xF) as u8)
                } else {
                    DclKind::Usage { usage: (d & 0x1F) as u8, index: ((d >> 16) & 0xF) as u8 }
                });
                ins.dst = Some(dst);
            }
            op::DEF | op::DEFI | op::DEFB => {
                let Some(r) = p.next() else { return err(format!("def at token {at}: missing register")) };
                ins.dst = Some(decode_dst(r));
                for (k, v) in p.take(4).enumerate() {
                    ins.imm[k] = v;
                }
            }
            _ => {
                if has_dst(opcode) {
                    let Some(d) = p.next() else { return err(format!("opcode {opcode} at token {at}: missing destination")) };
                    let mut dst = decode_dst(d);
                    if d & (1 << 13) != 0 && !sm1 {
                        let a = p.next().map(decode_src);
                        dst.rel = a.map(|a| Rel { reg: a.reg, comp: a.swizzle[0] });
                    }
                    ins.dst = Some(dst);
                }
                if predicated {
                    ins.predicate = p.next().map(decode_src);
                }
                while let Some(s) = p.next() {
                    let mut src = decode_src(s);
                    if s & (1 << 13) != 0 {
                        if sm1 {
                            src.rel = Some(Rel { reg: Reg { ty: reg::ADDR, index: 0 }, comp: 0 });
                        } else {
                            let Some(a) = p.next() else { return err(format!("opcode {opcode} at token {at}: missing address token")) };
                            let a = decode_src(a);
                            src.rel = Some(Rel { reg: a.reg, comp: a.swizzle[0] });
                        }
                    }
                    ins.src.push(src);
                }
                if let Some(n) = src_count(opcode, version)
                    && n != ins.src.len()
                {
                    return err(format!("opcode {opcode} at token {at}: {} sources, expected {n}", ins.src.len()));
                }
            }
        }
        instrs.push(ins);
    }
    Ok(Shader { version, instrs })
}

/// Length in tokens of the shader starting at `tokens[0]` (through the end token), skipping
/// comment payloads; `None` without an end token. Used to copy a shader out of guest memory.
pub fn token_length(mut next: impl FnMut(usize) -> Option<u32>) -> Option<usize> {
    let v = Version::from_token(next(0)?)?;
    let mut i = 1;
    loop {
        let t = next(i)?;
        i += 1;
        match (t & 0xFFFF) as u16 {
            op::END => return Some(i),
            op::COMMENT => i += ((t >> 16) & 0x7FFF) as usize,
            _ if v.major >= 2 && (t & 0xFFFF) as u16 != op::PHASE => i += ((t >> 24) & 0xF) as usize,
            _ => {}
        }
        if i > 1 << 20 {
            return None;
        }
    }
}

// ------------------------------------------------------------------------- disassembly -------

/// Mnemonic of an opcode (D3DX assembler spelling; `tex`/`texcoord` depend on the version).
pub fn mnemonic(opcode: u16, v: Version, control: u8) -> &'static str {
    match opcode {
        op::NOP => "nop",
        op::MOV => "mov",
        op::ADD => "add",
        op::SUB => "sub",
        op::MAD => "mad",
        op::MUL => "mul",
        op::RCP => "rcp",
        op::RSQ => "rsq",
        op::DP3 => "dp3",
        op::DP4 => "dp4",
        op::MIN => "min",
        op::MAX => "max",
        op::SLT => "slt",
        op::SGE => "sge",
        op::EXP => "exp",
        op::LOG => "log",
        op::LIT => "lit",
        op::DST => "dst",
        op::LRP => "lrp",
        op::FRC => "frc",
        op::M4X4 => "m4x4",
        op::M4X3 => "m4x3",
        op::M3X4 => "m3x4",
        op::M3X3 => "m3x3",
        op::M3X2 => "m3x2",
        op::CALL => "call",
        op::CALLNZ => "callnz",
        op::LOOP => "loop",
        op::RET => "ret",
        op::ENDLOOP => "endloop",
        op::LABEL => "label",
        op::DCL => "dcl",
        op::POW => "pow",
        op::CRS => "crs",
        op::SGN => "sgn",
        op::ABS => "abs",
        op::NRM => "nrm",
        op::SINCOS => "sincos",
        op::REP => "rep",
        op::ENDREP => "endrep",
        op::IF => "if",
        op::IFC => "if",
        op::ELSE => "else",
        op::ENDIF => "endif",
        op::BREAK => "break",
        op::BREAKC => "break",
        op::MOVA => "mova",
        op::DEFB => "defb",
        op::DEFI => "defi",
        op::TEXCOORD if v.pixel && v.major == 1 && v.minor >= 4 => "texcrd",
        op::TEXCOORD => "texcoord",
        op::TEXKILL => "texkill",
        op::TEX if v.major >= 2 && control == 1 => "texldp",
        op::TEX if v.major >= 2 && control == 2 => "texldb",
        op::TEX if v.major >= 2 || v.minor >= 4 => "texld",
        op::TEX => "tex",
        op::TEXBEM => "texbem",
        op::TEXBEML => "texbeml",
        op::TEXREG2AR => "texreg2ar",
        op::TEXREG2GB => "texreg2gb",
        op::TEXM3X2PAD => "texm3x2pad",
        op::TEXM3X2TEX => "texm3x2tex",
        op::TEXM3X3PAD => "texm3x3pad",
        op::TEXM3X3TEX => "texm3x3tex",
        op::TEXM3X3SPEC => "texm3x3spec",
        op::TEXM3X3VSPEC => "texm3x3vspec",
        op::EXPP => "expp",
        op::LOGP => "logp",
        op::CND => "cnd",
        op::DEF => "def",
        op::TEXREG2RGB => "texreg2rgb",
        op::TEXDP3TEX => "texdp3tex",
        op::TEXM3X2DEPTH => "texm3x2depth",
        op::TEXDP3 => "texdp3",
        op::TEXM3X3 => "texm3x3",
        op::TEXDEPTH => "texdepth",
        op::CMP => "cmp",
        op::BEM => "bem",
        op::DP2ADD => "dp2add",
        op::DSX => "dsx",
        op::DSY => "dsy",
        op::TEXLDD => "texldd",
        op::SETP => "setp",
        op::TEXLDL => "texldl",
        op::BREAKP => "breakp",
        op::PHASE => "phase",
        _ => "?",
    }
}

/// Register name in shader `v` (`r0`, `v1`, `c2`, `a0`, `t3`, `oPos`, `oD0`, `oT1`, …).
pub fn reg_name(r: Reg, v: Version) -> String {
    let i = r.index;
    match r.ty {
        reg::TEMP => format!("r{i}"),
        reg::INPUT => format!("v{i}"),
        reg::CONST => format!("c{i}"),
        reg::ADDR if v.pixel => format!("t{i}"),
        reg::ADDR => format!("a{i}"),
        reg::RASTOUT => ["oPos", "oFog", "oPts"].get(i as usize).copied().unwrap_or("oR?").to_string(),
        reg::ATTROUT => format!("oD{i}"),
        reg::TEXCRDOUT if v.major >= 3 => format!("o{i}"),
        reg::TEXCRDOUT => format!("oT{i}"),
        reg::CONSTINT => format!("i{i}"),
        reg::COLOROUT => format!("oC{i}"),
        reg::DEPTHOUT => "oDepth".into(),
        reg::SAMPLER => format!("s{i}"),
        reg::CONSTBOOL => format!("b{i}"),
        reg::LOOP => "aL".into(),
        reg::MISCTYPE => ["vPos", "vFace"].get(i as usize).copied().unwrap_or("misc?").to_string(),
        reg::LABEL => format!("l{i}"),
        reg::PREDICATE => format!("p{i}"),
        t => format!("R{t}_{i}"),
    }
}

const COMP: [char; 4] = ['x', 'y', 'z', 'w'];

fn fmt_src(s: &Src, v: Version) -> String {
    let mut name = reg_name(s.reg, v);
    if let Some(rel) = s.rel {
        let a = reg_name(rel.reg, v);
        name = if rel.reg.ty == reg::LOOP {
            format!("{}[{a} + {}]", &name[..1], s.reg.index)
        } else {
            format!("{}[{a}.{} + {}]", &name[..1], COMP[rel.comp as usize], s.reg.index)
        };
    }
    let sw = if s.swizzle == [0, 1, 2, 3] {
        String::new()
    } else if s.swizzle.iter().all(|&c| c == s.swizzle[0]) {
        format!(".{}", COMP[s.swizzle[0] as usize])
    } else {
        format!(".{}", s.swizzle.iter().map(|&c| COMP[c as usize]).collect::<String>())
    };
    match s.modifier {
        srcmod::NEG => format!("-{name}{sw}"),
        srcmod::BIAS => format!("{name}_bias{sw}"),
        srcmod::BIASNEG => format!("-{name}_bias{sw}"),
        srcmod::SIGN => format!("{name}_bx2{sw}"),
        srcmod::SIGNNEG => format!("-{name}_bx2{sw}"),
        srcmod::COMP => format!("1-{name}{sw}"),
        srcmod::X2 => format!("{name}_x2{sw}"),
        srcmod::X2NEG => format!("-{name}_x2{sw}"),
        srcmod::DZ => format!("{name}_dz{sw}"),
        srcmod::DW => format!("{name}_dw{sw}"),
        srcmod::ABS => format!("{name}_abs{sw}"),
        srcmod::ABSNEG => format!("-{name}_abs{sw}"),
        srcmod::NOT => format!("!{name}{sw}"),
        _ => format!("{name}{sw}"),
    }
}

fn fmt_dst(d: &Dst, v: Version) -> String {
    let mut s = reg_name(d.reg, v);
    if d.mask != 0xF && d.mask != 0 {
        s.push('.');
        s.extend((0..4).filter(|k| d.mask >> k & 1 != 0).map(|k| COMP[k]));
    }
    s
}

const USAGE: [&str; 14] = [
    "position", "blendweight", "blendindices", "normal", "psize", "texcoord", "tangent", "binormal", "tessfactor",
    "positiont", "color", "fog", "depth", "sample",
];

impl fmt::Display for Shader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.version;
        writeln!(f, "{v}")?;
        for ins in &self.instrs {
            if ins.coissue {
                f.write_str("+")?;
            }
            if let Some(p) = &ins.predicate {
                write!(f, "({}) ", fmt_src(p, v))?;
            }
            match ins.opcode {
                op::DCL => {
                    let d = ins.dst.as_ref().expect("dcl has a register");
                    let m = match (d.partial, d.centroid) {
                        (true, true) => "_pp_centroid",
                        (true, false) => "_pp",
                        (false, true) => "_centroid",
                        _ => "",
                    };
                    match ins.dcl {
                        Some(DclKind::Sampler(t)) => {
                            let ty = match t {
                                2 => "_2d",
                                3 => "_cube",
                                4 => "_volume",
                                _ => "",
                            };
                            writeln!(f, "dcl{ty}{m} {}", fmt_dst(d, v))?;
                        }
                        Some(DclKind::Usage { usage, index }) if !v.pixel || v.major >= 3 => {
                            let u = USAGE.get(usage as usize).copied().unwrap_or("unknown");
                            let n = if index == 0 { String::new() } else { index.to_string() };
                            writeln!(f, "dcl_{u}{n}{m} {}", fmt_dst(d, v))?;
                        }
                        _ => writeln!(f, "dcl{m} {}", fmt_dst(d, v))?,
                    }
                }
                op::DEF => {
                    let d = ins.dst.as_ref().expect("def has a register");
                    let x = ins.imm.map(f32::from_bits);
                    writeln!(f, "def {}, {:?}, {:?}, {:?}, {:?}", fmt_dst(d, v), x[0], x[1], x[2], x[3])?;
                }
                op::DEFI => {
                    let d = ins.dst.as_ref().expect("defi has a register");
                    let x = ins.imm.map(|b| b as i32);
                    writeln!(f, "defi {}, {}, {}, {}, {}", fmt_dst(d, v), x[0], x[1], x[2], x[3])?;
                }
                op::DEFB => {
                    let d = ins.dst.as_ref().expect("defb has a register");
                    writeln!(f, "defb {}, {}", fmt_dst(d, v), if ins.imm[0] != 0 { "true" } else { "false" })?;
                }
                _ => {
                    let mut m = mnemonic(ins.opcode, v, ins.control).to_string();
                    if let Some(d) = &ins.dst {
                        if d.saturate {
                            m += "_sat";
                        }
                        if d.partial {
                            m += "_pp";
                        }
                        if d.centroid {
                            m += "_centroid";
                        }
                        m += match d.shift {
                            1 => "_x2",
                            2 => "_x4",
                            3 => "_x8",
                            -1 => "_d2",
                            -2 => "_d4",
                            -3 => "_d8",
                            _ => "",
                        };
                    }
                    if matches!(ins.opcode, op::IFC | op::BREAKC | op::SETP) {
                        m += ["", "_gt", "_eq", "_ge", "_lt", "_ne", "_le", ""][(ins.control & 7) as usize];
                    }
                    let mut args: Vec<String> = Vec::new();
                    if let Some(d) = &ins.dst {
                        args.push(fmt_dst(d, v));
                    }
                    args.extend(ins.src.iter().map(|s| fmt_src(s, v)));
                    if args.is_empty() {
                        writeln!(f, "{m}")?;
                    } else {
                        writeln!(f, "{m} {}", args.join(", "))?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_sm1_and_sm2() {
        // vs_1_1: dcl_position v0; m4x4 oPos, v0, c[a0.x + 4]; mov oD0, -c4.x; end
        let vs = [
            0xFFFE_0101,
            0x0000_001F,
            0x8000_0000,
            0x900F_0000,
            0x0000_0014,
            0xC00F_0000,
            0x90E4_0000,
            0xA0E4_2004,
            0x0000_0001,
            0xD00F_0000,
            0xA100_0004,
            0x0000_FFFF,
        ];
        let s = parse(&vs).unwrap();
        assert_eq!(s.instrs.len(), 3);
        assert_eq!(s.input_decls(), vec![(0, 0, 0)]);
        let m = &s.instrs[1];
        assert_eq!(m.opcode, op::M4X4);
        assert_eq!(m.src[1].rel, Some(Rel { reg: Reg { ty: reg::ADDR, index: 0 }, comp: 0 }));
        assert_eq!(s.instrs[2].src[0].modifier, srcmod::NEG);
        assert_eq!(s.instrs[2].src[0].swizzle, [0; 4]);
        let text = s.to_string();
        assert!(text.contains("m4x4 oPos, v0, c[a0.x + 4]"), "{text}");
        assert!(text.contains("mov oD0, -c4.x"), "{text}");

        // vs_2_0 with a relative-address token and a comment
        let vs2 = [
            0xFFFE_0200,
            0x0002_FFFE,
            0x1234_5678,
            0x0000_FFFF, // inside the comment: not the end
            0x0300_0001,
            0x800F_0000,
            0xA0E4_2005,
            0xB0AA_0000, // a0.z
            0x0000_FFFF,
        ];
        let s = parse(&vs2).unwrap();
        assert_eq!(s.instrs.len(), 1);
        assert_eq!(s.instrs[0].src[0].rel, Some(Rel { reg: Reg { ty: reg::ADDR, index: 0 }, comp: 2 }));
        assert_eq!(token_length(|i| vs2.get(i).copied()), Some(vs2.len()));
    }
}
