//! x87 fidelity of Unicorn, measured against the real CPU: small probe functions (one x87 instruction
//! each, loaded at [`PROBE`]) run with the same inputs and control words on both backends.
//!
//! The measured table is in `docs/re/l1-harness.md`. [`UNFAITHFUL`] lists the instructions whose results
//! differ; a function that executes one of them is routed to L2 (`specs::Route`), which the test
//! `routes_match_hazards` enforces by scanning each spec's code.

use crate::call::*;
use crate::emu::{Allow, Emu};
use crate::generate::{self, Rng};
use crate::pe::Image;
use sa_math::x87::X87;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

/// x87 instructions measured as unfaithful under Unicorn (mnemonics as iced-x86 names them).
/// Kept in sync with the measurement in docs/re/l1-harness.md.
pub const UNFAITHFUL: &[&str] = &[
    "Fpatan", "Fsin", "Fcos", "Fsincos", "Fptan", "F2xm1", "Fyl2x", "Fyl2xp1", "Fprem", "Fprem1", "Fscale",
    "Fxtract",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operands {
    /// one operand: `fld [esp+4]`
    One,
    /// two operands: ST1 = first, ST0 = second
    Two,
}

pub struct Probe {
    pub name: &'static str,
    pub ops: Operands,
    /// instruction bytes after the operand loads (before `ret`)
    pub body: &'static [u8],
    /// the probe returns in EAX (x87 stack empty) instead of ST0
    pub eax: bool,
}

pub const PROBES: &[Probe] = &[
    Probe {
        name: "fld/fst (load, store)",
        ops: Operands::One,
        body: &[],
        eax: false,
    },
    Probe {
        name: "fadd",
        ops: Operands::Two,
        body: &[0xDE, 0xC1],
        eax: false,
    },
    Probe {
        name: "fsub",
        ops: Operands::Two,
        body: &[0xDE, 0xE9],
        eax: false,
    },
    Probe {
        name: "fmul",
        ops: Operands::Two,
        body: &[0xDE, 0xC9],
        eax: false,
    },
    Probe {
        name: "fdiv",
        ops: Operands::Two,
        body: &[0xDE, 0xF9],
        eax: false,
    },
    Probe {
        name: "fsqrt",
        ops: Operands::One,
        body: &[0xD9, 0xFA],
        eax: false,
    },
    Probe {
        name: "fcom (condition codes)",
        ops: Operands::Two,
        body: &[0xD8, 0xD1, 0xDD, 0xD9],
        eax: false,
    },
    Probe {
        name: "frndint",
        ops: Operands::One,
        body: &[0xD9, 0xFC],
        eax: false,
    },
    Probe {
        name: "fistp (int conversion)",
        ops: Operands::One,
        body: &[0xDB, 0x5C, 0x24, 0x04, 0x8B, 0x44, 0x24, 0x04],
        eax: true,
    },
    Probe {
        name: "fscale",
        ops: Operands::Two,
        body: &[0xD9, 0xFD, 0xDD, 0xD9],
        eax: false,
    },
    Probe {
        name: "fxtract",
        ops: Operands::One,
        body: &[0xD9, 0xF4, 0xDD, 0xD9],
        eax: false,
    },
    Probe {
        name: "fpatan",
        ops: Operands::Two,
        body: &[0xD9, 0xF3],
        eax: false,
    },
    Probe {
        name: "fsin",
        ops: Operands::One,
        body: &[0xD9, 0xFE],
        eax: false,
    },
    Probe {
        name: "fcos",
        ops: Operands::One,
        body: &[0xD9, 0xFF],
        eax: false,
    },
    Probe {
        name: "fsincos (cos)",
        ops: Operands::One,
        body: &[0xD9, 0xFB, 0xDD, 0xD9],
        eax: false,
    },
    Probe {
        name: "fptan",
        ops: Operands::One,
        body: &[0xD9, 0xF2, 0xDD, 0xD8],
        eax: false,
    },
    Probe {
        name: "f2xm1",
        ops: Operands::One,
        body: &[0xD9, 0xF0],
        eax: false,
    },
    Probe {
        name: "fyl2x",
        ops: Operands::Two,
        body: &[0xD9, 0xF1],
        eax: false,
    },
    Probe {
        name: "fyl2xp1",
        ops: Operands::Two,
        body: &[0xD9, 0xF9],
        eax: false,
    },
    Probe {
        name: "fprem",
        ops: Operands::Two,
        body: &[0xD9, 0xF8, 0xDD, 0xD9],
        eax: false,
    },
    Probe {
        name: "fprem1",
        ops: Operands::Two,
        body: &[0xD9, 0xF5, 0xDD, 0xD9],
        eax: false,
    },
];

const SLOT: usize = 32;

/// Operand width of a probe variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    F32,
    F64,
}

fn slot(k: usize, w: Width) -> u32 {
    PROBE + (SLOT * (2 * k + (w == Width::F64) as usize)) as u32
}

/// The probe code blob (each probe in an f32 and an f64 variant).
pub fn probe_code() -> Vec<u8> {
    let mut code = vec![0xCCu8; SLOT * 2 * PROBES.len()];
    for (k, p) in PROBES.iter().enumerate() {
        for w in [Width::F32, Width::F64] {
            let mut b: Vec<u8> = match (w, p.ops) {
                (Width::F32, Operands::One) => vec![0xD9, 0x44, 0x24, 0x04],
                (Width::F32, Operands::Two) => vec![0xD9, 0x44, 0x24, 0x04, 0xD9, 0x44, 0x24, 0x08],
                (Width::F64, Operands::One) => vec![0xDD, 0x44, 0x24, 0x04],
                (Width::F64, Operands::Two) => vec![0xDD, 0x44, 0x24, 0x04, 0xDD, 0x44, 0x24, 0x0C],
            };
            b.extend_from_slice(p.body);
            b.push(0xC3);
            assert!(b.len() <= SLOT);
            let o = (slot(k, w) - PROBE) as usize;
            code[o..o + b.len()].copy_from_slice(&b);
        }
    }
    code
}

fn f64_any(r: &mut Rng) -> f64 {
    match r.below(100) {
        0..=29 => f64::from_bits(r.next_u64()),
        30..=69 => {
            let range = r.pick(&[1.0f64, 4.0, 100.0, 1e4, 1e8]);
            let u = (r.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            (u * 2.0 - 1.0) * range
        }
        _ => generate::f32_any(r) as f64,
    }
}

fn probe_call(k: usize, w: Width, cw: u16, r: &mut Rng) -> Call {
    let n = match PROBES[k].ops {
        Operands::One => 1,
        Operands::Two => 2,
    };
    let args: Vec<Arg> = (0..n)
        .map(|_| match w {
            Width::F32 => Arg::F32(generate::f32_any(r)),
            Width::F64 => Arg::F64(f64_any(r)),
        })
        .collect();
    let mut c = Call::new(slot(k, w), Conv::Cdecl, &args);
    c.fpu = Fpu {
        cw,
        mxcsr: GAME_MXCSR,
    };
    c
}

#[derive(Debug, Default, Clone)]
pub struct Row {
    pub probe: &'static str,
    pub width: &'static str,
    pub cw: u16,
    pub n: u64,
    /// field -> cases (fields of cases with a signalling-NaN argument are counted as "<field> [SNaN arg]")
    pub diffs: BTreeMap<String, u64>,
    pub example: Option<String>,
}

/// A signalling NaN among the call's float arguments.
fn has_snan(c: &Call, w: Width) -> bool {
    match w {
        Width::F32 => c
            .stack
            .iter()
            .any(|&b| b & 0x7F80_0000 == 0x7F80_0000 && b & 0x7F_FFFF != 0 && b & 0x40_0000 == 0),
        Width::F64 => c.stack.chunks(2).any(|p| {
            let b = p[0] as u64 | (p[1] as u64) << 32;
            let frac = b & ((1 << 52) - 1);
            (b >> 52) & 0x7FF == 0x7FF && frac != 0 && frac & (1 << 51) == 0
        }),
    }
}

fn show(o: &CallResult) -> String {
    match o {
        Ok(o) => format!(
            "st0={} f32={:#010x} fpsw={:#06x} eax={:#x}",
            o.st0.map(|e| ext_hex(&e)).unwrap_or("-".into()),
            o.st0_f32.unwrap_or(0),
            o.fpsw,
            o.eax
        ),
        Err(e) => e.to_string(),
    }
}

/// Runs every probe x width x control word with `n` cases on both backends (MXCSR = the game's).
/// `ftz_workaround`: clear MXCSR.FZ inside Unicorn (see `emu`).
pub fn measure(
    img: &Arc<Image>,
    n: u64,
    cws: &[u16],
    seed: u64,
    ftz_workaround: bool,
) -> Result<Vec<Row>, String> {
    let code = probe_code();
    let mut emu = Emu::new(img.clone())?;
    emu.ftz_workaround = ftz_workaround;
    emu.load_probes(&code)?;
    emu.budget = 1000;
    let allow = Allow::range(PROBE, PROBE + code.len() as u32);
    let mut keys = Vec::new();
    for k in 0..PROBES.len() {
        for w in [Width::F32, Width::F64] {
            for &cw in cws {
                keys.push((k, w, cw));
            }
        }
    }
    // Rows are processed in chunks of at most ~200k calls (one Wine run each), so memory stays bounded
    // (all rows at once took 2.7 GB at 20k cases per row).
    let per_chunk = ((200_000 / n.max(1)) as usize).max(1);
    let mut rows = Vec::new();
    for chunk in keys.chunks(per_chunk) {
        let mut calls = Vec::with_capacity(chunk.len() * n as usize);
        for &(k, w, cw) in chunk {
            let mut r = Rng::new(seed ^ ((k as u64) << 32) ^ ((cw as u64) << 8) ^ (w == Width::F64) as u64);
            for _ in 0..n {
                calls.push(probe_call(k, w, cw, &mut r));
            }
        }
        let uni: Vec<CallResult> = calls.iter().map(|c| emu.call(c, &allow)).collect();
        if let Some(e) = uni.iter().find_map(|r| r.as_ref().err()) {
            return Err(format!("probe failed under Unicorn: {e}"));
        }
        let cpu = crate::native::run(img, &calls, Some(&code))?;
        for (g, &(k, w, cw)) in chunk.iter().enumerate() {
            rows.push(row_of(k, w, cw, n, &calls, &uni, &cpu, g));
        }
    }
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
fn row_of(
    k: usize,
    w: Width,
    cw: u16,
    n: u64,
    calls: &[Call],
    uni: &[CallResult],
    cpu: &[Outcome],
    g: usize,
) -> Row {
    let p = &PROBES[k];
    let mut row = Row {
        probe: p.name,
        width: if w == Width::F32 { "f32" } else { "f64" },
        cw,
        n,
        ..Default::default()
    };
    for j in g * n as usize..(g + 1) * n as usize {
        let a = uni[j].as_ref().unwrap();
        let d = a.diff(&cpu[j], if p.eax { !0 } else { 0 });
        let snan = has_snan(&calls[j], w);
        for f in &d {
            let key = if snan {
                format!("{f} [SNaN arg]")
            } else {
                f.to_string()
            };
            *row.diffs.entry(key).or_default() += 1;
        }
        let value = d.iter().any(|f| !matches!(*f, "C1" | "exc-flags"));
        if value && !snan && row.example.is_none() {
            let args: Vec<String> = match w {
                Width::F32 => calls[j]
                    .stack
                    .iter()
                    .map(|&b| format!("{:?}", f32::from_bits(b)))
                    .collect(),
                Width::F64 => calls[j]
                    .stack
                    .chunks(2)
                    .map(|c| format!("{:?}", f64::from_bits(c[0] as u64 | (c[1] as u64) << 32)))
                    .collect(),
            };
            row.example = Some(format!(
                "args [{}]: unicorn {} | cpu {}",
                args.join(", "),
                show(&uni[j]),
                show(&Ok(cpu[j].clone()))
            ));
        }
    }
    row
}

pub fn print_rows(rows: &[Row]) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{:<24} {:<4} {:<7} {:>8}  differing fields (count)",
        "probe", "op", "cw", "cases"
    );
    for r in rows {
        let d = if r.diffs.is_empty() {
            "identical".to_string()
        } else {
            r.diffs
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let _ = writeln!(s, "{:<24} {:<4} {:#06x}  {:>8}  {d}", r.probe, r.width, r.cw, r.n);
        if let Some(e) = &r.example {
            let _ = writeln!(s, "{:>40} e.g. {e}", "");
        }
    }
    s
}

// ------------------------------------------------------------------------------ NaN matrix ------

/// One instruction form of the NaN matrix: code after `fld dword [esp+4]` (and, for register forms,
/// `fld dword [esp+8]`), and the `sa_math::x87` model of it for operands A = [esp+4], B = [esp+8].
pub struct Form {
    pub name: &'static str,
    pub reg: bool,
    pub code: &'static [u8],
    pub model: fn(f32, f32) -> X87,
}

pub const FORMS: &[Form] = &[
    Form {
        name: "faddp st1,st",
        reg: true,
        code: &[0xDE, 0xC1],
        model: |a, b| X87::from_f32(a).fadd(X87::from_f32(b)),
    },
    Form {
        name: "fsubp st1,st",
        reg: true,
        code: &[0xDE, 0xE9],
        model: |a, b| X87::from_f32(a).fsub(X87::from_f32(b)),
    },
    Form {
        name: "fsubrp st1,st",
        reg: true,
        code: &[0xDE, 0xE1],
        model: |a, b| X87::from_f32(a).fsubr(X87::from_f32(b)),
    },
    Form {
        name: "fmulp st1,st",
        reg: true,
        code: &[0xDE, 0xC9],
        model: |a, b| X87::from_f32(a).fmul(X87::from_f32(b)),
    },
    Form {
        name: "fdivp st1,st",
        reg: true,
        code: &[0xDE, 0xF9],
        model: |a, b| X87::from_f32(a).fdiv(X87::from_f32(b)),
    },
    Form {
        name: "fdivrp st1,st",
        reg: true,
        code: &[0xDE, 0xF1],
        model: |a, b| X87::from_f32(a).fdivr(X87::from_f32(b)),
    },
    Form {
        name: "fpatan",
        reg: true,
        code: &[0xD9, 0xF3],
        model: |a, b| X87::from_f32(a).fpatan(X87::from_f32(b)),
    },
    Form {
        name: "fadd m32",
        reg: false,
        code: &[0xD8, 0x44, 0x24, 0x08],
        model: |a, b| X87::from_f32(a).fadd_m(b),
    },
    Form {
        name: "fsub m32",
        reg: false,
        code: &[0xD8, 0x64, 0x24, 0x08],
        model: |a, b| X87::from_f32(a).fsub_m(b),
    },
    Form {
        name: "fsubr m32",
        reg: false,
        code: &[0xD8, 0x6C, 0x24, 0x08],
        model: |a, b| X87::from_f32(a).fsubr_m(b),
    },
    Form {
        name: "fmul m32",
        reg: false,
        code: &[0xD8, 0x4C, 0x24, 0x08],
        model: |a, b| X87::from_f32(a).fmul_m(b),
    },
    Form {
        name: "fdiv m32",
        reg: false,
        code: &[0xD8, 0x74, 0x24, 0x08],
        model: |a, b| X87::from_f32(a).fdiv_m(b),
    },
    Form {
        name: "fdivr m32",
        reg: false,
        code: &[0xD8, 0x7C, 0x24, 0x08],
        model: |a, b| X87::from_f32(a).fdivr_m(b),
    },
    Form {
        name: "fsqrt (A)",
        reg: false,
        code: &[0xD9, 0xFA],
        model: |a, _| X87::from_f32(a).fsqrt(),
    },
    Form {
        name: "fchs (A)",
        reg: false,
        code: &[0xD9, 0xE0],
        model: |a, _| X87::from_f32(a).fchs(),
    },
];

/// Operand values: quiet and signalling NaNs of both signs and several payloads, infinities, zeros,
/// ordinary numbers.
pub const MATRIX_VALUES: &[u32] = &[
    0x7FC0_0000,
    0xFFC0_0000,
    0x7FC0_1234,
    0xFFC0_1234,
    0x7FFF_FFFF,
    0xFFFF_FFFF,
    0x7F80_0001,
    0xFF80_0001,
    0x7FA0_0000,
    0xFFA0_0000,
    0x7F81_2345,
    0x3F80_0000,
    0xBFC0_0000,
    0x7F80_0000,
    0xFF80_0000,
    0x0000_0000,
    0x8000_0000,
    0x0000_0001,
];

fn matrix_code() -> Vec<u8> {
    let mut code = vec![0xCCu8; SLOT * FORMS.len()];
    for (k, f) in FORMS.iter().enumerate() {
        let mut b = vec![0xD9, 0x44, 0x24, 0x04];
        if f.reg {
            b.extend_from_slice(&[0xD9, 0x44, 0x24, 0x08]);
        }
        b.extend_from_slice(f.code);
        b.push(0xC3);
        code[SLOT * k..SLOT * k + b.len()].copy_from_slice(&b);
    }
    code
}

#[derive(Debug, Default)]
pub struct MatrixReport {
    pub cases: usize,
    /// model vs CPU (bit-exact ST0; FPATAN of two non-NaN operands compares the f32 only)
    pub model_mismatches: Vec<String>,
    /// Unicorn vs CPU on ST0 (bit-exact)
    pub unicorn_mismatches: usize,
    pub unicorn_example: Option<String>,
}

/// Every form x every operand pair on the CPU and under Unicorn, against the `sa_math::x87` model.
pub fn nan_matrix(img: &Arc<Image>) -> Result<MatrixReport, String> {
    let code = matrix_code();
    let mut calls = Vec::new();
    let mut meta = Vec::new();
    for (k, _) in FORMS.iter().enumerate() {
        for &a in MATRIX_VALUES {
            for &b in MATRIX_VALUES {
                let args = [Arg::F32(f32::from_bits(a)), Arg::F32(f32::from_bits(b))];
                calls.push(Call::new(PROBE + (SLOT * k) as u32, Conv::Cdecl, &args));
                meta.push((k, a, b));
            }
        }
    }
    let mut emu = Emu::new(img.clone())?;
    emu.load_probes(&code)?;
    emu.budget = 100;
    let allow = Allow::range(PROBE, PROBE + code.len() as u32);
    let uni: Vec<CallResult> = calls.iter().map(|c| emu.call(c, &allow)).collect();
    let cpu = crate::native::run(img, &calls, Some(&code))?;
    let mut rep = MatrixReport {
        cases: calls.len(),
        ..Default::default()
    };
    for (j, &(k, a, b)) in meta.iter().enumerate() {
        let f = &FORMS[k];
        let model = (f.model)(f32::from_bits(a), f32::from_bits(b));
        let got = cpu[j].st0.unwrap_or_default();
        let finite_atan = f.name == "fpatan" && !model.is_nan();
        let same = if finite_atan {
            cpu[j].st0_f32 == Some(model.to_f32().to_bits())
        } else {
            got == model.to_ext()
        };
        if !same {
            rep.model_mismatches.push(format!(
                "{:<14} A={a:#010x} B={b:#010x}: cpu {} | model {}",
                f.name,
                ext_hex(&got),
                ext_hex(&model.to_ext())
            ));
        }
        if let Ok(u) = &uni[j]
            && u.st0 != cpu[j].st0
        {
            rep.unicorn_mismatches += 1;
            if rep.unicorn_example.is_none() {
                rep.unicorn_example = Some(format!(
                    "{} A={a:#010x} B={b:#010x}: unicorn {} | cpu {}",
                    f.name,
                    ext_hex(&u.st0.unwrap_or_default()),
                    ext_hex(&got)
                ));
            }
        }
    }
    Ok(rep)
}
