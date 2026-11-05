#![cfg(not(windows))]
//! Verifier probes for the L1 claims (INF-7 / MATH). Adversarial checks written independently of the
//! harness's own tests; findings in `docs/re/verify-l1.md`.
//!
//! Run (heavy, prints reports; tests that need the exe or the native backend skip without them):
//! `cargo test --release -p sa-oracle --test verifier_l1 -- --test-threads=1 --nocapture`
//!
//! `--test-threads=1`: the native backend writes `target/sa-oracle-native/probes.bin`, so native runs
//! must not overlap.

use sa_math::consts as k;
use sa_math::x87::{Cmp, X87};
use sa_oracle::call::*;
use sa_oracle::runner::{self, Backend, Cfg};
use sa_oracle::specs::{self, ExpRet, Expect, Inputs, Spec};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn img() -> Option<Arc<sa_oracle::pe::Image>> {
    let i = sa_oracle::image_cached();
    if i.is_none() {
        eprintln!("note: gta_sa.exe 1.0 US not found, test skipped");
    }
    i
}

fn native() -> bool {
    let ok = sa_oracle::native::available();
    if !ok {
        eprintln!("note: native backend unavailable, test skipped");
    }
    ok
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.replace('_', "").parse().ok())
        .unwrap_or(default)
}

fn cfg(cases: u64, backend: Backend) -> Cfg {
    Cfg {
        cases,
        backend,
        threads: 8,
        fixtures: false,
        keep: 1,
        shrink: false,
        snan_native: sa_oracle::native::available(),
        ..Cfg::default()
    }
}

fn f(x: u32) -> f32 {
    f32::from_bits(x)
}

fn st0(v: X87, s: &[u8; SCRATCH_LEN]) -> Expect {
    Expect::Returns {
        ret: ExpRet::St0 {
            f32: v.to_f32().to_bits(),
            ext: v.to_ext(),
        },
        scratch: *s,
    }
}

// =============================================================== 1. branch coverage ================

mod cov {
    use super::*;
    use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, Mnemonic};
    use unicorn_engine::{Arch, Mode, Prot, RegisterX86 as R, Unicorn};

    const EXIT: u32 = 0x10000;
    const ESP0: u32 = 0x1F_0000;

    #[derive(Default)]
    struct St {
        prev: u32,
        hi: u32,
        lo: u32,
        exec: BTreeSet<u32>,
        edges: BTreeSet<(u32, u32)>,
    }

    pub struct Cov {
        pub jcc: Vec<(u32, String, bool, bool)>,
        pub uncovered: Vec<(u32, String)>,
        pub instructions: usize,
    }

    /// Runs `inputs` under a bare Unicorn instance with an instruction hook over the function's extent
    /// and returns which instructions and which conditional-branch directions were executed.
    pub fn coverage(img: &sa_oracle::pe::Image, spec: &Spec, inputs: &[Inputs], max_insns: usize) -> Cov {
        let cat = sa_oracle::catalog_cached().unwrap();
        let (lo, hi) = cat.extent(spec.addr).unwrap();
        let mut uc = Unicorn::new_with_data(
            Arch::X86,
            Mode::MODE_32,
            St {
                lo,
                hi,
                ..Default::default()
            },
        )
        .unwrap();
        uc.ctl_set_cpu_model(unicorn_engine::X86CpuModel::PENTIUM3 as i32)
            .unwrap();
        let cr4 = uc.reg_read(R::CR4).unwrap();
        uc.reg_write(R::CR4, cr4 | 0x600).unwrap();
        let cr0 = uc.reg_read(R::CR0).unwrap();
        uc.reg_write(R::CR0, (cr0 & !0x4) | 0x22).unwrap();
        let page = |x: u32| (x + 0xFFF) & !0xFFF;
        uc.mem_map(EXIT as u64, 0x1000, Prot::ALL).unwrap();
        uc.mem_write(EXIT as u64, &[0xF4]).unwrap();
        uc.mem_map(0x10_0000, 0x10_0000, Prot::READ | Prot::WRITE)
            .unwrap();
        uc.mem_map(img.base as u64, page(img.flat.len() as u32) as u64, Prot::ALL)
            .unwrap();
        uc.mem_write(img.base as u64, &img.flat).unwrap();
        uc.mem_map(SCRATCH as u64, 0x1_0000, Prot::READ | Prot::WRITE)
            .unwrap();
        uc.add_code_hook(lo as u64, (hi - 1) as u64, |uc, a, _| {
            let d = uc.get_data_mut();
            let a = a as u32;
            if d.prev != 0 {
                d.edges.insert((d.prev, a));
            }
            d.exec.insert(a);
            d.prev = a;
        })
        .unwrap();
        // the last instruction before leaving the function (ret): record an edge to EXIT
        uc.add_code_hook(EXIT as u64, EXIT as u64, |uc, _, _| {
            let d = uc.get_data_mut();
            let p = d.prev;
            d.edges.insert((p, EXIT));
        })
        .unwrap();
        let _ = uc.get_data().hi + uc.get_data().lo;
        for inp in inputs {
            let c = (spec.call)(inp);
            uc.mem_write(SCRATCH as u64, &c.scratch).unwrap();
            let esp = ESP0 - 4 * c.stack.len() as u32 - 4;
            let mut st = EXIT.to_le_bytes().to_vec();
            for v in &c.stack {
                st.extend_from_slice(&v.to_le_bytes());
            }
            uc.mem_write(esp as u64, &st).unwrap();
            for (r, v) in [
                (R::ECX, c.ecx),
                (R::EDX, c.edx),
                (R::ESP, esp),
                (R::EFLAGS, 0x202),
            ] {
                uc.reg_write(r, v as u64).unwrap();
            }
            uc.reg_write(R::FPCW, c.fpu.cw as u64).unwrap();
            uc.reg_write(R::FPSW, 0).unwrap();
            uc.reg_write(R::FPTAG, 0xFFFF).unwrap();
            uc.reg_write(R::MXCSR, (c.fpu.mxcsr & !0x8000) as u64).unwrap();
            uc.get_data_mut().prev = 0;
            let _ = uc.emu_start(spec.addr as u64, EXIT as u64, 0, max_insns);
        }
        let d = uc.get_data();
        let bytes = img.bytes(lo, (hi - lo) as usize);
        let mut dec = Decoder::with_ip(32, bytes, lo as u64, DecoderOptions::NONE);
        let mut ins = Instruction::default();
        let mut out = Cov {
            jcc: vec![],
            uncovered: vec![],
            instructions: 0,
        };
        while dec.can_decode() {
            dec.decode_out(&mut ins);
            let a = ins.ip32();
            let text = format!("{:?}", ins.mnemonic());
            let filler = matches!(ins.mnemonic(), Mnemonic::Nop | Mnemonic::Int3)
                || (ins.mnemonic() == Mnemonic::Lea && ins.op0_register() == iced_x86::Register::ESP)
                || ins.is_invalid();
            if filler {
                continue;
            }
            out.instructions += 1;
            if !d.exec.contains(&a) {
                out.uncovered.push((a, text.clone()));
            }
            if ins.flow_control() == FlowControl::ConditionalBranch {
                let t = ins.near_branch32();
                let n = ins.next_ip32();
                out.jcc
                    .push((a, text, d.edges.contains(&(a, t)), d.edges.contains(&(a, n))));
            }
        }
        out
    }
}

/// Branch coverage of each original function under (a) the edge cases alone and (b) the edge cases
/// plus the generator. Reports instructions never executed and conditional branches with a direction
/// never taken.
#[test]
fn verifier_branch_coverage() {
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_COV_CASES", 200_000);
    for spec in specs::all() {
        let edges = (spec.edges)();
        let generated: Vec<Inputs> = (0..n)
            .map(|i| runner::case_input(&spec, &[], 0x5A_0001, i))
            .collect();
        let all: Vec<Inputs> = edges.iter().cloned().chain(generated.iter().cloned()).collect();
        for (label, set) in [
            ("edges only", &edges),
            ("generator only", &generated),
            ("edges + generator", &all),
        ] {
            let c = cov::coverage(&img, &spec, set, 200_000);
            let partial: Vec<_> = c.jcc.iter().filter(|j| !(j.2 && j.3)).collect();
            println!(
                "{} @ {:#x} [{label}, {} cases]: {} instructions, {} uncovered, {} jcc, {} with a direction never taken",
                spec.name,
                spec.addr,
                set.len(),
                c.instructions,
                c.uncovered.len(),
                c.jcc.len(),
                partial.len()
            );
            for (a, t) in &c.uncovered {
                println!("    uncovered {a:#x}: {t}");
            }
            for (a, t, taken, fall) in partial {
                println!("    jcc {a:#x} {t}: taken={taken} fallthrough={fall}");
            }
        }
    }
}

// ========================================== 2. Unicorn vs CPU on every non-SNaN case (L1 routes) ====

/// For the Unicorn-routed specs, Unicorn and the CPU must agree on every observable field on every
/// case that is not routed away (no signalling-NaN input). Any disagreement is a routing error.
#[test]
fn verifier_unicorn_agrees_with_cpu_outside_snan() {
    let Some(img) = img() else { return };
    if !native() {
        return;
    }
    let n = env_u64("VERIFY_CASES", 200_000);
    for spec in specs::all().into_iter().filter(|s| s.route == specs::Route::L1) {
        let prep = runner::prepare(&img, &spec).unwrap();
        let edges = (spec.edges)();
        let total = edges.len() as u64 + n;
        let mut bad: BTreeMap<String, u64> = BTreeMap::new();
        let mut example = None;
        let mut compared = 0u64;
        let mut start = 0;
        while start < total {
            let end = (start + 200_000).min(total);
            let inputs: Vec<Inputs> = (start..end)
                .map(|i| runner::case_input(&spec, &edges, 0x5A_0001, i))
                .collect();
            let calls: Vec<Call> = inputs.iter().map(|i| (spec.call)(i)).collect();
            let c = cfg(0, Backend::Native);
            let b = runner::oracle_batch(&img, &spec, &prep, &inputs, &calls, &c).unwrap();
            for (k, input) in inputs.iter().enumerate() {
                if runner::has_snan(&spec, input) {
                    continue;
                }
                if let (Ok(u), Ok(cpu)) = (&b.uni[k], &b.res[k]) {
                    compared += 1;
                    let mask = if spec.ret == specs::RetKind::Al { 0xFF } else { 0 };
                    // everything a caller could see, including C0/C2/C3 and the exception flags
                    let d = u.diff(cpu, mask);
                    let d: Vec<_> = d
                        .into_iter()
                        .filter(|f| *f != "C1" && *f != "exc-flags")
                        .collect();
                    if !d.is_empty() {
                        for x in &d {
                            *bad.entry(x.to_string()).or_default() += 1;
                        }
                        example.get_or_insert((inputs[k].clone(), u.clone(), cpu.clone()));
                    }
                }
            }
            start = end;
        }
        println!(
            "{}: {compared} non-SNaN cases on both backends; observable Unicorn/CPU differences: {bad:?}",
            spec.name
        );
        if let Some((i, u, c)) = &example {
            println!(
                "    e.g. {} unicorn {u:?}\n         cpu {c:?}",
                runner::fmt_inputs(&spec, i)
            );
        }
        // C0/C2/C3 after the return are reported, not asserted: callers never read them (finding V-6 in
        // docs/re/verify-l1.md), but they show Unicorn's condition codes are not CPU-exact.
        let caller_visible: Vec<_> = bad.keys().filter(|k| *k != "C0/C2/C3").collect();
        assert!(
            caller_visible.is_empty(),
            "{}: Unicorn wrong outside the SNaN class: {caller_visible:?}",
            spec.name
        );
    }
}

// =================================================================== 3. mutants ====================

fn normalise_spec(port: impl Fn([f32; 3]) -> [f32; 3] + Send + Sync + 'static) -> Spec {
    let mut s = specs::normalise();
    s.rust = Arc::new(move |i, sc| {
        let v = port([f(i[0]), f(i[1]), f(i[2])]);
        let mut sc = *sc;
        let o = (specs::VEC - SCRATCH) as usize;
        for (k, c) in v.iter().enumerate() {
            sc[o + 4 * k..o + 4 * k + 4].copy_from_slice(&c.to_le_bytes());
        }
        Expect::Returns {
            ret: ExpRet::Void,
            scratch: sc,
        }
    });
    s
}

/// Normalise written as in the port, with hooks for each mutation.
fn normalise_m(v: [f32; 3], m: &str) -> [f32; 3] {
    let [x, y, z] = v.map(X87::from_f32);
    if m == "f32-arith" {
        let s = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        if s > 0.0 || s.is_nan() {
            let inv = 1.0 / s.sqrt();
            return [v[0] * inv, v[1] * inv, v[2] * inv];
        }
        return [1.0, v[1], v[2]];
    }
    let s = if m == "sum-order" {
        x.fmul(x).fadd(y.fmul(y).fadd(z.fmul(z)))
    } else {
        x.fmul(x).fadd(y.fmul(y)).fadd(z.fmul(z))
    };
    let root = if m == "nan-to-zero-branch" {
        s.fcom(X87::zero()) == Cmp::Greater
    } else {
        s.fcom(X87::zero()).test_parity(0x41)
    };
    if !root {
        return [1.0, v[1], v[2]];
    }
    let one = if m == "one-ulp-const" {
        f(k::F1 + 1)
    } else {
        f(k::F1)
    };
    if m == "divide-by-sqrt" {
        let r = s.fsqrt();
        return v.map(|c| X87::from_f32(c).fdiv(r).to_f32());
    }
    let inv = s.fsqrt().fdivr_m(one);
    v.map(|c| inv.fmul_m(c).to_f32())
}

fn quadratic_m(i: &Inputs, sc: &[u8; SCRATCH_LEN], m: &str) -> Expect {
    let (a, b, c_) = (f(i[0]), f(i[1]), f(i[2]));
    let (av, bv) = (X87::from_f32(a), X87::from_f32(b));
    let mut d = bv.fmul_m(b).fsub(av.fmul_m(c_).fmul_m(f(k::F4)));
    if m == "f32-discriminant-before-sqrt" {
        d = X87::from_f32(d.to_f32());
    }
    let cmp = d.fcom(X87::zero());
    let no_root = if m == "nan-disc-no-root" {
        !matches!(cmp, Cmp::Greater | Cmp::Equal)
    } else {
        !cmp.test_parity(0x05)
    };
    let roots = if no_root {
        None
    } else {
        let s = d.fsqrt();
        let inv = X87::constant(k::F1).fdiv_m(a);
        let x2 = if m == "x2-by-division" {
            s.fsub_m(b).fdiv_m(a).fmul_m(f(k::F0_5))
        } else {
            s.fsub_m(b).fmul(inv).fmul_m(f(k::F0_5))
        };
        let half1 = if m == "half-ulp-const" {
            f(k::F0_5 + 1)
        } else {
            f(k::F0_5)
        };
        let x1 = bv.fchs().fsub(s).fmul(inv).fmul_m(half1);
        Some((x1.to_f32(), x2.to_f32()))
    };
    if m == "store-x1-first" {
        // the aliased case keeps x2 instead of x1
        let mut s = *sc;
        let al = match roots {
            None => 0,
            Some((x1, x2)) => {
                let p1 = (specs::QX1 - SCRATCH) as usize;
                let p2 = (if i[3] != 0 { specs::QX1 } else { specs::QX2 } - SCRATCH) as usize;
                s[p1..p1 + 4].copy_from_slice(&x1.to_le_bytes());
                s[p2..p2 + 4].copy_from_slice(&x2.to_le_bytes());
                1
            }
        };
        return Expect::Returns {
            ret: ExpRet::Al(al),
            scratch: s,
        };
    }
    specs::quadratic_expect(i, sc, roots)
}

fn radian_m(x1: f32, y1: f32, x2: f32, y2: f32, m: &str) -> X87 {
    let dx = if m == "f32-differences" {
        X87::from_f32(x2 - x1)
    } else {
        X87::from_f32(x2).fsub_m(x1)
    };
    let mut dy = if m == "f32-differences" {
        X87::from_f32(y2 - y1)
    } else {
        X87::from_f32(y2).fsub_m(y1)
    };
    let is_zero = if m == "positive-zero-only" {
        dy.to_f32().to_bits() == 0 && dy.fcom(X87::zero()) == Cmp::Equal
    } else {
        !dy.fcom(X87::zero()).test_parity(0x44)
    };
    if is_zero {
        dy = X87::constant(k::F0_0001);
    }
    let sdx = dx.fcom(X87::zero());
    let sdy = dy.fcom(X87::zero());
    let t = if m == "atanf" {
        X87::from_f32(dx.fdiv(dy).to_f32().atan())
    } else {
        dx.fdiv(dy).fpatan(X87::one())
    };
    let h = f(k::HALF_PI);
    let h_last = if m == "half-pi-ulp" { f(k::HALF_PI + 1) } else { h };
    let dx_pos = if m == "dx-zero-is-positive" {
        matches!(sdx, Cmp::Greater | Cmp::Equal)
    } else {
        !sdx.test_any(0x41)
    };
    if dx_pos {
        if !sdy.test_any(0x41) {
            t.fsubr_m(h).fadd_m(h_last)
        } else {
            t.fadd_m(h).fsubr_m(h_last)
        }
    } else if !sdy.test_any(0x41) {
        t.fadd_m(h).fsubr_m(f(k::MINUS_HALF_PI))
    } else {
        t.fsubr_m(h).fsub_m(h_last)
    }
}

/// Five realistic mutants each for CVector::Normalise, CGeneral::SolveQuadratic (Unicorn route) and
/// CGeneral::GetRadianAngleBetweenPoints (CPU route). Each must be caught; the unmutated reference
/// versions written here must pass (sanity check of the mutant scaffolding).
#[test]
fn verifier_mutants() {
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_MUTANT_CASES", 200_000);
    let mut missed = vec![];
    let mut run = |name: String, spec: Spec, backend: Backend, expect_caught: bool| {
        let rep = runner::run(&spec, &img, &cfg(n, backend)).unwrap();
        let caught = rep.mismatches > 0;
        println!(
            "  {name:<55} {:<7} mismatches={:>7} of {} {:?}",
            if caught { "CAUGHT" } else { "MISSED" },
            rep.mismatches,
            rep.total(),
            rep.kinds
        );
        if let Some(e) = rep.examples.first() {
            println!(
                "      e.g. [{}] {}: {}",
                runner::fmt_inputs(&spec, &e.input),
                e.kind,
                e.detail
            );
        }
        if caught != expect_caught {
            missed.push(name);
        }
    };
    println!("mutants, {n} generated cases each (+ edges)");
    for m in [
        "reference",
        "sum-order",
        "f32-arith",
        "nan-to-zero-branch",
        "one-ulp-const",
        "divide-by-sqrt",
    ] {
        let mm = m.to_string();
        run(
            format!("Normalise/{m}"),
            normalise_spec(move |v| normalise_m(v, &mm)),
            Backend::Unicorn,
            m != "reference",
        );
    }
    for m in [
        "reference",
        "x2-by-division",
        "half-ulp-const",
        "store-x1-first",
        "f32-discriminant-before-sqrt",
        "nan-disc-no-root",
    ] {
        let mm = m.to_string();
        let mut s = specs::solve_quadratic();
        s.rust = Arc::new(move |i, sc| quadratic_m(i, sc, &mm));
        run(
            format!("SolveQuadratic/{m}"),
            s,
            Backend::Unicorn,
            m != "reference",
        );
    }
    if sa_oracle::native::available() {
        for m in [
            "reference",
            "positive-zero-only",
            "f32-differences",
            "atanf",
            "dx-zero-is-positive",
            "half-pi-ulp",
        ] {
            let mm = m.to_string();
            let mut s = specs::get_radian_angle_between_points();
            s.rust = Arc::new(move |i, sc| st0(radian_m(f(i[0]), f(i[1]), f(i[2]), f(i[3]), &mm), sc));
            run(
                format!("GetRadianAngleBetweenPoints/{m}"),
                s,
                Backend::Native,
                m != "reference",
            );
        }
    }
    assert!(missed.is_empty(), "mutants with the wrong verdict: {missed:?}");
}

// ================================ 4. LimitAngle on the range the generator never produces ============

/// The LimitAngle generator replaces every finite |x| > 2^24 by a value <= 1e5, and the edge filter
/// keeps only the fixed points above 2^24. So 2^24 < |x| < 2^33 (where `a - 360` is inexact under
/// PC=24 and the loop step changes) is never compared. Run that range on the CPU directly (Unicorn's
/// instruction budget is too small for it).
#[test]
fn verifier_limit_angle_large_range() {
    let Some(img) = img() else { return };
    if !native() {
        return;
    }
    let spec = specs::limit_angle();
    let mut rng = sa_oracle::generate::Rng::new(0xBEEF);
    let mut xs: Vec<f32> = vec![];
    for e in 24..34 {
        let lo = 2f64.powi(e);
        xs.push(lo as f32);
        xs.push(f32::from_bits((lo as f32).to_bits() + 1));
        xs.push(f32::from_bits((lo as f32).to_bits() - 1));
        for _ in 0..env_u64("VERIFY_LA_PER_OCTAVE", 20) {
            let u = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            xs.push((lo * (1.0 + u)) as f32);
        }
    }
    xs.push(8_589_934_592.0); // 2^33: the ledger says it hangs
    let xs: Vec<f32> = xs.iter().flat_map(|&x| [x, -x]).collect();
    // port results first (some need ~10^7 iterations)
    let exp: Vec<Option<X87>> = std::thread::scope(|s| {
        let hs: Vec<_> = xs
            .chunks(xs.len().div_ceil(8))
            .map(|c| {
                s.spawn(move || {
                    c.iter()
                        .map(|&x| sa_math::general::limit_angle_checked(x))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let (ret, hang): (Vec<usize>, Vec<usize>) = (0..xs.len()).partition(|&i| exp[i].is_some());
    println!(
        "LimitAngle 2^24..2^34: {} values, port returns for {}, port says hang for {}",
        xs.len(),
        ret.len(),
        hang.len()
    );
    let calls: Vec<Call> = ret.iter().map(|&i| (spec.call)(&vec![xs[i].to_bits()])).collect();
    let out = sa_oracle::native::run(&img, &calls, None).unwrap();
    let mut bad = 0;
    for (j, &i) in ret.iter().enumerate() {
        let e = st0(exp[i].unwrap(), &calls[j].scratch);
        if let Err((kind, d)) = runner::compare(&spec, &calls[j], &Ok(out[j].clone()), &e) {
            bad += 1;
            if bad <= 5 {
                println!("    MISMATCH x={:e}: {kind}: {d}", xs[i]);
            }
        }
    }
    let min_hang = hang.iter().map(|&i| xs[i].abs()).fold(f32::INFINITY, f32::min);
    println!(
        "    CPU vs port mismatches on returning values: {bad}; smallest |x| the port calls a hang: {min_hang:e}"
    );
    assert_eq!(bad, 0);
}

// ================================ 5. GetATanOfXY: is the 64-bit ST0 observable by callers? ===========

/// 35 of the 109 call sites of GetATanOfXY consume the returned ST0 directly with a PC=24 arithmetic op
/// (e.g. 0x42acfe `fadd [PI]` in CCarCtrl::SteerAIHeliFlyingAwayFromPlayer, 0x4201b8 `fsubr [esp+28]`).
/// Probe: `GetATanOfXY(x, y) + PI` computed by the original on the CPU, against the port's
/// `get_atan_of_xy_x87(..).fadd_m(PI)` and against the f32 API (`get_atan_of_xy(..)` then `+ PI`).
#[test]
fn verifier_atan_st0_consumed_by_callers() {
    let Some(img) = img() else { return };
    if !native() {
        return;
    }
    // mov eax,[esp+8]; push eax; mov eax,[esp+8]; push eax; mov eax,0x53cc70; call eax; add esp,8;
    // fadd dword [0x858cb8] (PI, as at 0x42acfe); ret
    let code: Vec<u8> = vec![
        0x8B, 0x44, 0x24, 0x08, 0x50, 0x8B, 0x44, 0x24, 0x08, 0x50, 0xB8, 0x70, 0xCC, 0x53, 0x00, 0xFF, 0xD0,
        0x83, 0xC4, 0x08, 0xD8, 0x05, 0xB8, 0x8C, 0x85, 0x00, 0xC3,
    ];
    let n = env_u64("VERIFY_ATAN_CASES", 200_000);
    let mut rng = sa_oracle::generate::Rng::new(0xA7A7);
    let mut inputs = vec![];
    while (inputs.len() as u64) < n {
        let v = sa_oracle::generate::f32_tuple(&mut rng, 2);
        let (x, y) = (v[0], v[1]);
        // the raw-FPATAN path: x > 0, y > 0, |x| >= |y|
        if x > 0.0 && y > 0.0 && x >= y && x.is_finite() {
            inputs.push((x, y));
        }
    }
    let calls: Vec<Call> = inputs
        .iter()
        .map(|&(x, y)| {
            let mut c = Call::new(PROBE, Conv::Cdecl, &[Arg::F32(x), Arg::F32(y)]);
            c.scratch = [specs::FILL; SCRATCH_LEN];
            c
        })
        .collect();
    let out = sa_oracle::native::run(&img, &calls, Some(&code)).unwrap();
    let pi = f(k::PI);
    let (mut bad_x87, mut bad_f32) = (0u64, 0u64);
    let mut ex = None;
    for (j, &(x, y)) in inputs.iter().enumerate() {
        let cpu = out[j].st0.unwrap();
        let a = sa_math::general::get_atan_of_xy_x87(x, y).fadd_m(pi);
        let b = X87::from_f32(sa_math::general::get_atan_of_xy(x, y)).fadd_m(pi);
        if a.to_ext() != cpu {
            bad_x87 += 1;
            ex.get_or_insert((x, y, cpu, a.to_ext()));
        }
        if b.to_ext() != cpu {
            bad_f32 += 1;
        }
    }
    println!(
        "GetATanOfXY raw path, ST0 + PI on {} cases: port x87 API differs in {bad_x87}, port f32 API differs in {bad_f32}",
        inputs.len()
    );
    if let Some((x, y, c, p)) = ex {
        println!(
            "    e.g. x={x:e} y={y:e} cpu {} port {}",
            ext_hex(&c),
            ext_hex(&p)
        );
    }
}
