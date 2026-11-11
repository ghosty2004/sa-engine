//! Direct3D 9 shader model 1.x/2.0 bytecode: decoding ([`parse`]), the D3DX assembler syntax
//! ([`asm`]), a reference CPU interpreter ([`interp`]) and the WGSL translator ([`wgsl`]).
//!
//! The game creates its shaders in two ways (docs/arch/gpu.md): RenderWare's D3D9 vertex-shader
//! generator (`_rpD3D9VertexShaderGenerate`, 0x75F0B0) prints `vs_1_1`/`vs_2_0` assembly and
//! assembles it with the statically linked `D3DXAssembleShader` (0x7652AE); RpMatFX creates six
//! precompiled `ps_1_1` pixel shaders from `.rdata` (0x8852F0..). Both reach
//! `IDirect3DDevice9::CreateVertexShader`/`CreatePixelShader` as token streams, which [`parse`]
//! decodes into [`Shader`], [`wgsl`] translates and [`interp`] executes on the CPU (the reference
//! the GPU tests compare against).
//!
//! Token format (d3d9types.h): a version token (`0xFFFE_mmnn` vertex, `0xFFFF_mmnn` pixel), then
//! instructions (opcode in bits 0..15, SM2+ instruction length in bits 24..27, `D3DSI_COISSUE` bit
//! 30) each followed by parameter tokens with bit 31 set, ending with `0x0000_FFFF`.

pub mod asm;
pub mod interp;
pub mod parse;
pub mod wgsl;

#[cfg(test)]
mod gpu_tests;

pub use parse::{parse, ParseError};

/// `D3DSHADER_INSTRUCTION_OPCODE_TYPE`.
pub mod op {
    pub const NOP: u16 = 0;
    pub const MOV: u16 = 1;
    pub const ADD: u16 = 2;
    pub const SUB: u16 = 3;
    pub const MAD: u16 = 4;
    pub const MUL: u16 = 5;
    pub const RCP: u16 = 6;
    pub const RSQ: u16 = 7;
    pub const DP3: u16 = 8;
    pub const DP4: u16 = 9;
    pub const MIN: u16 = 10;
    pub const MAX: u16 = 11;
    pub const SLT: u16 = 12;
    pub const SGE: u16 = 13;
    pub const EXP: u16 = 14;
    pub const LOG: u16 = 15;
    pub const LIT: u16 = 16;
    pub const DST: u16 = 17;
    pub const LRP: u16 = 18;
    pub const FRC: u16 = 19;
    pub const M4X4: u16 = 20;
    pub const M4X3: u16 = 21;
    pub const M3X4: u16 = 22;
    pub const M3X3: u16 = 23;
    pub const M3X2: u16 = 24;
    pub const CALL: u16 = 25;
    pub const CALLNZ: u16 = 26;
    pub const LOOP: u16 = 27;
    pub const RET: u16 = 28;
    pub const ENDLOOP: u16 = 29;
    pub const LABEL: u16 = 30;
    pub const DCL: u16 = 31;
    pub const POW: u16 = 32;
    pub const CRS: u16 = 33;
    pub const SGN: u16 = 34;
    pub const ABS: u16 = 35;
    pub const NRM: u16 = 36;
    pub const SINCOS: u16 = 37;
    pub const REP: u16 = 38;
    pub const ENDREP: u16 = 39;
    pub const IF: u16 = 40;
    pub const IFC: u16 = 41;
    pub const ELSE: u16 = 42;
    pub const ENDIF: u16 = 43;
    pub const BREAK: u16 = 44;
    pub const BREAKC: u16 = 45;
    pub const MOVA: u16 = 46;
    pub const DEFB: u16 = 47;
    pub const DEFI: u16 = 48;
    pub const TEXCOORD: u16 = 64;
    pub const TEXKILL: u16 = 65;
    pub const TEX: u16 = 66;
    pub const TEXBEM: u16 = 67;
    pub const TEXBEML: u16 = 68;
    pub const TEXREG2AR: u16 = 69;
    pub const TEXREG2GB: u16 = 70;
    pub const TEXM3X2PAD: u16 = 71;
    pub const TEXM3X2TEX: u16 = 72;
    pub const TEXM3X3PAD: u16 = 73;
    pub const TEXM3X3TEX: u16 = 74;
    pub const TEXM3X3SPEC: u16 = 76;
    pub const TEXM3X3VSPEC: u16 = 77;
    pub const EXPP: u16 = 78;
    pub const LOGP: u16 = 79;
    pub const CND: u16 = 80;
    pub const DEF: u16 = 81;
    pub const TEXREG2RGB: u16 = 82;
    pub const TEXDP3TEX: u16 = 83;
    pub const TEXM3X2DEPTH: u16 = 84;
    pub const TEXDP3: u16 = 85;
    pub const TEXM3X3: u16 = 86;
    pub const TEXDEPTH: u16 = 87;
    pub const CMP: u16 = 88;
    pub const BEM: u16 = 89;
    pub const DP2ADD: u16 = 90;
    pub const DSX: u16 = 91;
    pub const DSY: u16 = 92;
    pub const TEXLDD: u16 = 93;
    pub const SETP: u16 = 94;
    pub const TEXLDL: u16 = 95;
    pub const BREAKP: u16 = 96;
    pub const PHASE: u16 = 0xFFFD;
    pub const COMMENT: u16 = 0xFFFE;
    pub const END: u16 = 0xFFFF;
}

/// `D3DSHADER_PARAM_REGISTER_TYPE` (bits 28..30 of a parameter token, plus bits 11..12 as bits 3..4).
pub mod reg {
    pub const TEMP: u8 = 0;
    pub const INPUT: u8 = 1;
    pub const CONST: u8 = 2;
    /// `a0` in vertex shaders, `tN` in pixel shaders (`D3DSPR_ADDR` = `D3DSPR_TEXTURE`).
    pub const ADDR: u8 = 3;
    pub const TEXTURE: u8 = 3;
    /// `oPos` (0), `oFog` (1), `oPts` (2).
    pub const RASTOUT: u8 = 4;
    /// `oD0`, `oD1`.
    pub const ATTROUT: u8 = 5;
    /// `oTn` (`D3DSPR_TEXCRDOUT` = `D3DSPR_OUTPUT`).
    pub const TEXCRDOUT: u8 = 6;
    pub const CONSTINT: u8 = 7;
    pub const COLOROUT: u8 = 8;
    pub const DEPTHOUT: u8 = 9;
    pub const SAMPLER: u8 = 10;
    pub const CONST2: u8 = 11;
    pub const CONST3: u8 = 12;
    pub const CONST4: u8 = 13;
    pub const CONSTBOOL: u8 = 14;
    pub const LOOP: u8 = 15;
    pub const TEMPFLOAT16: u8 = 16;
    pub const MISCTYPE: u8 = 17;
    pub const LABEL: u8 = 18;
    pub const PREDICATE: u8 = 19;
}

/// `D3DSHADER_PARAM_SRCMOD_TYPE` (bits 24..27 of a source token).
pub mod srcmod {
    pub const NONE: u8 = 0;
    pub const NEG: u8 = 1;
    pub const BIAS: u8 = 2;
    pub const BIASNEG: u8 = 3;
    /// `_bx2`
    pub const SIGN: u8 = 4;
    pub const SIGNNEG: u8 = 5;
    /// `1 - x`
    pub const COMP: u8 = 6;
    pub const X2: u8 = 7;
    pub const X2NEG: u8 = 8;
    /// `_dz` / `_db`
    pub const DZ: u8 = 9;
    /// `_dw` / `_da`
    pub const DW: u8 = 10;
    pub const ABS: u8 = 11;
    pub const ABSNEG: u8 = 12;
    /// `!` (boolean / predicate)
    pub const NOT: u8 = 13;
}

/// `D3DSAMPLER_TEXTURE_TYPE` (bits 27..30 of a sampler `dcl` token).
pub mod sampler_type {
    pub const UNKNOWN: u8 = 0;
    pub const TEX2D: u8 = 2;
    pub const CUBE: u8 = 3;
    pub const VOLUME: u8 = 4;
}

/// Shader kind and version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub pixel: bool,
    pub major: u8,
    pub minor: u8,
}

impl Version {
    pub fn from_token(t: u32) -> Option<Version> {
        let pixel = match t >> 16 {
            0xFFFE => false,
            0xFFFF => true,
            _ => return None,
        };
        Some(Version { pixel, major: (t >> 8) as u8, minor: t as u8 })
    }
    pub fn token(self) -> u32 {
        (if self.pixel { 0xFFFF_0000 } else { 0xFFFE_0000 }) | (self.major as u32) << 8 | self.minor as u32
    }
    /// At least `major.minor`.
    pub fn at_least(self, major: u8, minor: u8) -> bool {
        (self.major, self.minor) >= (major, minor)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = if self.pixel { "ps" } else { "vs" };
        if self.major == 2 && self.minor == 1 {
            return write!(f, "{kind}_2_x");
        }
        write!(f, "{kind}_{}_{}", self.major, self.minor)
    }
}

/// A register reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg {
    /// [`reg`] type.
    pub ty: u8,
    pub index: u32,
}

/// Relative addressing of a source: `c[a0.x + n]` (`reg` = `a0` or `aL`, `comp` = 0..3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rel {
    pub reg: Reg,
    pub comp: u8,
}

/// A destination parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dst {
    pub reg: Reg,
    /// Write mask bits (1 = x … 8 = w).
    pub mask: u8,
    /// `_sat`
    pub saturate: bool,
    /// `_pp`
    pub partial: bool,
    /// `_centroid`
    pub centroid: bool,
    /// Result shift (ps_1_x): 1 = x2, 2 = x4, 3 = x8, -1 = d2, -2 = d4, -3 = d8.
    pub shift: i8,
    /// Relative addressing (vs_3_0 outputs only; kept for completeness).
    pub rel: Option<Rel>,
}

/// A source parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Src {
    pub reg: Reg,
    /// Component selected for each of x, y, z, w (0..3).
    pub swizzle: [u8; 4],
    /// [`srcmod`] value.
    pub modifier: u8,
    pub rel: Option<Rel>,
}

/// What a `dcl` declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DclKind {
    /// Vertex input / output semantic (`D3DDECLUSAGE`, usage index); also ps_2_0 `dcl v0`/`dcl t0`
    /// (usage 0).
    Usage { usage: u8, index: u8 },
    /// Sampler texture type ([`sampler_type`]).
    Sampler(u8),
}

/// One decoded instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct Instr {
    pub opcode: u16,
    /// Bits 16..23 of the instruction token (`texldp`/`texldb` control, comparison of `ifc`/`setp`).
    pub control: u8,
    /// `D3DSI_COISSUE` (ps_1_x `+` prefix).
    pub coissue: bool,
    pub dst: Option<Dst>,
    pub src: Vec<Src>,
    /// `dcl`
    pub dcl: Option<DclKind>,
    /// `def`/`defi`/`defb` values (bit patterns).
    pub imm: [u32; 4],
    /// Predicate source (`(p0.x) instr`), when present.
    pub predicate: Option<Src>,
}

impl Instr {
    pub fn new(opcode: u16) -> Instr {
        Instr { opcode, control: 0, coissue: false, dst: None, src: Vec::new(), dcl: None, imm: [0; 4], predicate: None }
    }
}

/// A decoded shader.
#[derive(Debug, Clone, PartialEq)]
pub struct Shader {
    pub version: Version,
    /// Every instruction in order, `dcl`/`def` included (comments and `end` dropped).
    pub instrs: Vec<Instr>,
}

impl Shader {
    /// Constants the shader defines itself (`def`): (register, value).
    pub fn float_defs(&self) -> impl Iterator<Item = (u32, [f32; 4])> + '_ {
        self.instrs.iter().filter(|i| i.opcode == op::DEF).filter_map(|i| {
            let d = i.dst?;
            Some((d.reg.index, i.imm.map(f32::from_bits)))
        })
    }

    /// `dcl` usage declarations of input (`v`) registers: (register, usage, usage index).
    pub fn input_decls(&self) -> Vec<(u32, u8, u8)> {
        self.instrs
            .iter()
            .filter(|i| i.opcode == op::DCL)
            .filter_map(|i| match (i.dcl, i.dst) {
                (Some(DclKind::Usage { usage, index }), Some(d)) if d.reg.ty == reg::INPUT => Some((d.reg.index, usage, index)),
                _ => None,
            })
            .collect()
    }

    /// Sampler declarations (ps_2_0 `dcl_2d s0`): (sampler, [`sampler_type`]).
    pub fn sampler_decls(&self) -> Vec<(u32, u8)> {
        self.instrs
            .iter()
            .filter(|i| i.opcode == op::DCL)
            .filter_map(|i| match (i.dcl, i.dst) {
                (Some(DclKind::Sampler(t)), Some(d)) if d.reg.ty == reg::SAMPLER => Some((d.reg.index, t)),
                _ => None,
            })
            .collect()
    }
}
