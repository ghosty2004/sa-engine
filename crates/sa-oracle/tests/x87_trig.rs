#![cfg(not(windows))]
//! FPATAN / FSIN / FCOS / FSINCOS: the `sa_math::x87` models against the real CPU (native backend).
//! Skipped (with a note) when gta_sa.exe or the native backend is unavailable.
//!
//! `X87_TRIG_DUMP=<dir>` additionally writes every case as text (`<instr>.txt`: operand f64 bits,
//! CPU 80-bit result as hex) for offline model fitting.

use sa_oracle::call::*;
use sa_math::x87::X87;
use sa_oracle::generate::Rng;
use std::sync::Arc;

fn img() -> Option<Arc<sa_oracle::pe::Image>> {
    let i = sa_oracle::image_cached();
    if i.is_none() {
        eprintln!("note: gta_sa.exe 1.0 US not found, test skipped");
    }
    i
}

/// Probe code (at PROBE), f64 operands from the stack:
/// - 0x00: `fld qword [esp+4] (y); fld qword [esp+12] (x); fpatan; ret`
/// - 0x10: `fld qword [esp+4]; fsin; ret`
/// - 0x18: `fld qword [esp+4]; fcos; ret`
/// - 0x20: `fld qword [esp+4]; fsincos; ret` (ST0 = cos)
/// - 0x28: `fld qword [esp+4]; fsincos; fstp st(0); ret` (ST0 = sin)
fn probes() -> Vec<u8> {
    let mut c = vec![0xCCu8; 0x40];
    c[..11].copy_from_slice(&[0xDD, 0x44, 0x24, 0x04, 0xDD, 0x44, 0x24, 0x0C, 0xD9, 0xF3, 0xC3]);
    c[0x10..0x17].copy_from_slice(&[0xDD, 0x44, 0x24, 0x04, 0xD9, 0xFE, 0xC3]);
    c[0x18..0x1F].copy_from_slice(&[0xDD, 0x44, 0x24, 0x04, 0xD9, 0xFF, 0xC3]);
    c[0x20..0x27].copy_from_slice(&[0xDD, 0x44, 0x24, 0x04, 0xD9, 0xFB, 0xC3]);
    c[0x28..0x31].copy_from_slice(&[0xDD, 0x44, 0x24, 0x04, 0xD9, 0xFB, 0xDD, 0xD8, 0xC3]);
    c
}

/// One generated f32 or f64 operand for FSIN/FCOS.
fn trig_operand(r: &mut Rng) -> f64 {
    let pi = std::f64::consts::PI;
    match r.below(10) {
        0 => f64::from(r.uniform(1.6)),
        1 => f64::from(r.uniform(7.0)),
        2 => f64::from(r.uniform(1e4)),
        3 => f64::from(f32::from_bits(r.next_u32())),
        4 => f64::from_bits(r.next_u64()),
        // near multiples of pi/2 (f32 and f64 neighbours)
        5 => {
            let k = f64::from(r.below(4000)) - 2000.0;
            let v = (k * pi / 2.0) as f32;
            f64::from(f32::from_bits(v.to_bits().wrapping_add(r.below(9)).wrapping_sub(4)))
        }
        6 => {
            let k = f64::from(r.below(4000)) - 2000.0;
            f64::from_bits((k * pi / 2.0).to_bits().wrapping_add(u64::from(r.below(9))).wrapping_sub(4))
        }
        7 => f64::from(r.uniform(1.0)) * 2f64.powi(r.below(60) as i32),
        8 => (f64::from(r.uniform(1.0)) + f64::from(r.uniform(1.0)) * 2f64.powi(-30)) * 8.0,
        _ => f64::from(r.uniform(400.0)) * pi / 180.0,
    }
}

/// One generated (y, x) operand pair for FPATAN.
fn atan_operands(r: &mut Rng) -> (f64, f64) {
    match r.below(8) {
        0 => (f64::from(r.uniform(1.0)), f64::from(r.uniform(1.0))),
        1 => {
            // unit directions (front vectors)
            let a = f64::from(r.uniform(4.0));
            ((a.sin() as f32).into(), (a.cos() as f32).into())
        }
        2 => (f64::from(f32::from_bits(r.next_u32())), f64::from(f32::from_bits(r.next_u32()))),
        3 => (f64::from_bits(r.next_u64()), f64::from_bits(r.next_u64())),
        4 => {
            // |y| ~ |x| (atan near pi/4)
            let x = r.uniform(100.0);
            let y = f32::from_bits(x.to_bits().wrapping_add(r.below(64)).wrapping_sub(32));
            (y.into(), x.into())
        }
        5 => {
            // one operand much smaller
            let s = 2f64.powi(-(r.below(40) as i32));
            (f64::from(r.uniform(1.0)) * s, f64::from(r.uniform(1.0)))
        }
        6 => {
            let s = 2f64.powi(-(r.below(40) as i32));
            (f64::from(r.uniform(1.0)), f64::from(r.uniform(1.0)) * s)
        }
        _ => (f64::from(r.uniform(1e4)), f64::from(r.uniform(1e4))),
    }
}

/// Runs `calls` on the CPU in chunks; returns ST0 (None: empty stack) and the status word.
fn cpu(img: &sa_oracle::pe::Image, calls: &[Call]) -> Vec<(Option<[u8; 10]>, u16)> {
    let code = probes();
    let mut out = Vec::with_capacity(calls.len());
    for ch in calls.chunks(200_000) {
        let o = sa_oracle::native::run(img, ch, Some(&code)).unwrap();
        out.extend(o.iter().map(|o| (o.st0, o.fpsw)));
    }
    out
}

fn ext_hex(e: Option<[u8; 10]>) -> String {
    match e {
        None => "empty".into(),
        Some(b) => {
            let m = u64::from_le_bytes(b[..8].try_into().unwrap());
            let se = u16::from_le_bytes([b[8], b[9]]);
            format!("{se:04x}{m:016x}")
        }
    }
}

/// False for a finite non-zero CPU result whose exponent is outside the normal f64 range: `X87` holds
/// values as f64 pairs and does not model those (they are unreachable from f32 game data).
fn in_model_range(st0: Option<[u8; 10]>) -> bool {
    let Some(b) = st0 else { return true };
    let e = u16::from_le_bytes([b[8], b[9]]) & 0x7FFF;
    let m = u64::from_le_bytes(b[..8].try_into().unwrap());
    e == 0x7FFF || m == 0 || (0x3C01 + 64..=0x43FE).contains(&e)
}

/// Model-vs-CPU result of one instruction.
#[derive(Default)]
struct Tally {
    n: usize,
    exact: usize,
    f32_exact: usize,
    shown: usize,
}

impl Tally {
    fn add(&mut self, what: &str, ops: &str, cpu: Option<[u8; 10]>, model: Option<X87>) {
        self.n += 1;
        let m = model.map(|m| m.to_ext());
        if cpu == m {
            self.exact += 1;
            self.f32_exact += 1;
            return;
        }
        if let (Some(c), Some(m)) = (cpu, model)
            && X87::from_ext(c).to_f32().to_bits() == m.to_f32().to_bits()
        {
            self.f32_exact += 1;
        }
        if self.shown < 8 {
            self.shown += 1;
            eprintln!("  {what}({ops}): cpu {} model {}", ext_hex(cpu), ext_hex(m));
        }
    }

    fn report(&self, name: &str) {
        eprintln!(
            "{name}: {} cases, 80-bit exact {} ({:.4}%), stored f32 exact {} ({:.4}%)",
            self.n,
            self.exact,
            100.0 * self.exact as f64 / self.n as f64,
            self.f32_exact,
            100.0 * self.f32_exact as f64 / self.n as f64
        );
    }
}

/// FPATAN operands: special values, |y| == |x|, and the generated mix.
fn fpatan_cases(n: usize) -> Vec<(f64, f64)> {
    let sp = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.5,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        1e-300,
        -1e300,
        f64::from(f32::MIN_POSITIVE),
        f64::from(f32::from_bits(1)),
        f64::from(f32::MAX),
    ];
    let mut v: Vec<(f64, f64)> = sp.iter().flat_map(|&y| sp.iter().map(move |&x| (y, x))).collect();
    let mut r = Rng::new(0x7A7A);
    for _ in 0..2000 {
        let a = f64::from(r.uniform(1e4));
        v.push((a, a));
        v.push((a, -a));
        v.push((-a, f64::from(r.uniform(1.0)).signum() * a));
        // q exactly at a table midpoint (2k+1)/64 and at k/32
        let k = f64::from(r.below(64));
        v.push((a * (2.0 * k + 1.0) / 64.0, a * 2.0));
        v.push((a * k / 32.0, -a));
    }
    while v.len() < n {
        v.push(atan_operands(&mut r));
    }
    v
}

fn measure_fpatan(n: usize) {
    let Some(img) = img() else { return };
    if !sa_oracle::native::available() {
        eprintln!("note: native backend unavailable, test skipped");
        return;
    }
    let cases = fpatan_cases(n);
    let calls: Vec<Call> =
        cases.iter().map(|&(y, x)| Call::new(PROBE, Conv::Cdecl, &[Arg::F64(y), Arg::F64(x)])).collect();
    let res = cpu(&img, &calls);
    let mut t = Tally::default();
    let mut miss = String::new();
    let mut outside = 0;
    for (&(y, x), (st0, _)) in cases.iter().zip(&res) {
        if !in_model_range(*st0) {
            outside += 1;
            continue;
        }
        let m = fld64(y).fpatan(fld64(x));
        if Some(m.to_ext()) != *st0 {
            miss += &format!("0 {:016x} {:016x} {}\n", y.to_bits(), x.to_bits(), ext_hex(*st0));
        }
        t.add("fpatan", &format!("{y:e}, {x:e} / {:#x}, {:#x}", y.to_bits(), x.to_bits()), *st0, Some(m));
    }
    if let Ok(p) = std::env::var("X87_TRIG_MISS") {
        std::fs::write(p, miss).unwrap();
    }
    eprintln!("fpatan: {outside} results outside the f64 exponent range (not modelled, skipped)");
    t.report("fpatan");
    // measured: 5 in 10^6 differ by 1 ulp of 64 bits (polynomial error near a 67-bit boundary)
    assert_eq!(t.f32_exact, t.n, "fpatan: a stored f32 differs from the CPU");
    assert!(t.n - t.exact <= t.n / 100_000, "fpatan: {} 80-bit differences", t.n - t.exact);
}

#[test]
fn fpatan_model_vs_cpu() {
    measure_fpatan(200_000);
}

#[test]
#[ignore = "10^6 cases on the CPU; run explicitly"]
fn fpatan_model_vs_cpu_1m() {
    measure_fpatan(1_000_000);
}

const TWO63: f64 = 9_223_372_036_854_775_808.0;

/// FSIN/FCOS operands: special values, f32 angles of the game's ranges, the generated mix.
fn trig_cases(n: usize) -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001),
        TWO63,
        -TWO63,
        (TWO63 - 1024.0),
        1e19,
        1e-300,
        f64::from(f32::from_bits(1)),
        f64::from(f32::MIN_POSITIVE),
        0.25,
        0.5,
        0.78,
        f64::from(0.78f32),
    ];
    for i in 0..4096 {
        // the CMaths table steps, multiples of pi/4, and their neighbours
        let a = (i as f32) * std::f32::consts::PI / 128.0;
        v.push(f64::from(a));
        v.push(f64::from(f32::from_bits(a.to_bits() + 1)));
        v.push(f64::from((i as f32) * std::f32::consts::FRAC_PI_4));
    }
    let mut r = Rng::new(0x751C);
    while v.len() < n {
        v.push(trig_operand(&mut r));
    }
    v
}

fn measure_trig(n: usize) {
    let Some(img) = img() else { return };
    if !sa_oracle::native::available() {
        eprintln!("note: native backend unavailable, test skipped");
        return;
    }
    let xs = trig_cases(n);
    type Model = fn(X87) -> X87;
    let rows: [(&str, u32, Model); 4] = [
        ("fsin", 0x10, X87::fsin),
        ("fcos", 0x18, X87::fcos),
        ("fsincos (cos)", 0x20, |x| x.fsincos().map_or(x, |r| r.1)),
        ("fsincos (sin)", 0x28, |x| x.fsincos().map_or(x, |r| r.0)),
    ];
    let mut all = true;
    for (name, off, model) in rows {
        let calls: Vec<Call> =
            xs.iter().map(|&x| Call::new(PROBE + off, Conv::Cdecl, &[Arg::F64(x)])).collect();
        let res = cpu(&img, &calls);
        let mut t = Tally::default();
        let mut outside = 0;
        for (&x, (st0, _)) in xs.iter().zip(&res) {
            if !in_model_range(*st0) {
                outside += 1;
                continue;
            }
            // `fsincos; fstp st(0)` leaves nothing when |x| >= 2^63 (the operand stays, is popped)
            let m = if off == 0x28 && x.abs() >= TWO63 && x.is_finite() {
                None
            } else {
                Some(model(fld64(x)))
            };
            t.add(name, &format!("{x:e} / {:#x}", x.to_bits()), *st0, m);
        }
        eprintln!("{name}: {outside} results outside the f64 exponent range (skipped)");
        t.report(name);
        all &= t.f32_exact == t.n;
    }
    assert!(all, "a stored f32 differs from the CPU");
}

#[test]
fn trig_model_vs_cpu() {
    measure_trig(200_000);
}

#[test]
#[ignore = "10^6 cases per instruction on the CPU; run explicitly"]
fn trig_model_vs_cpu_1m() {
    measure_trig(1_000_000);
}

/// Runs the cases of `X87_TRIG_IN` (lines `<probe offset hex> <f64 bits hex> [<f64 bits hex>]`) and
/// writes `X87_TRIG_OUT` (`<input line> <ST0 80-bit hex | empty> <status word hex>`).
#[test]
#[ignore = "exploration: X87_TRIG_IN=<file> X87_TRIG_OUT=<file>"]
fn run_trig_cases_from_file() {
    let Some(img) = img() else { return };
    let inp = std::fs::read_to_string(std::env::var("X87_TRIG_IN").unwrap()).unwrap();
    let lines: Vec<&str> = inp.lines().filter(|l| !l.trim().is_empty()).collect();
    let calls: Vec<Call> = lines
        .iter()
        .map(|l| {
            let w: Vec<u64> = l.split_whitespace().map(|h| u64::from_str_radix(h, 16).unwrap()).collect();
            let args: Vec<Arg> = w[1..].iter().map(|&b| Arg::F64(f64::from_bits(b))).collect();
            Call::new(PROBE + w[0] as u32, Conv::Cdecl, &args)
        })
        .collect();
    let res = cpu(&img, &calls);
    let mut s = String::new();
    for (l, (st0, sw)) in lines.iter().zip(&res) {
        s += &format!("{l} {} {sw:04x}\n", ext_hex(*st0));
    }
    std::fs::write(std::env::var("X87_TRIG_OUT").unwrap(), s).unwrap();
}

#[test]
#[ignore = "exploration dump: X87_TRIG_DUMP=<dir> X87_TRIG_N=<cases>"]
fn dump_trig_cpu_results() {
    let Some(img) = img() else { return };
    let dir = std::env::var("X87_TRIG_DUMP").expect("X87_TRIG_DUMP");
    let n: usize = std::env::var("X87_TRIG_N").map_or(200_000, |v| v.parse().unwrap());
    let mut r = Rng::new(0x7816);
    let mut calls = Vec::new();
    let mut ops = Vec::new();
    for _ in 0..n {
        let (y, x) = atan_operands(&mut r);
        ops.push((y, x));
        calls.push(Call::new(PROBE, Conv::Cdecl, &[Arg::F64(y), Arg::F64(x)]));
    }
    let res = cpu(&img, &calls);
    let mut s = String::new();
    for ((y, x), (st0, _)) in ops.iter().zip(&res) {
        s += &format!("{:016x} {:016x} {}\n", y.to_bits(), x.to_bits(), ext_hex(*st0));
    }
    std::fs::write(format!("{dir}/fpatan.txt"), s).unwrap();
    for (name, off) in [("fsin", 0x10), ("fcos", 0x18), ("fsincos_cos", 0x20), ("fsincos_sin", 0x28)] {
        let mut r = Rng::new(0x7817);
        let xs: Vec<f64> = (0..n).map(|_| trig_operand(&mut r)).collect();
        let calls: Vec<Call> =
            xs.iter().map(|&x| Call::new(PROBE + off, Conv::Cdecl, &[Arg::F64(x)])).collect();
        let res = cpu(&img, &calls);
        let mut s = String::new();
        for (x, (st0, sw)) in xs.iter().zip(&res) {
            s += &format!("{:016x} {} {sw:04x}\n", x.to_bits(), ext_hex(*st0));
        }
        std::fs::write(format!("{dir}/{name}.txt"), s).unwrap();
    }
}

/// `fld qword`: exact, a signalling NaN is quieted.
fn fld64(v: f64) -> X87 {
    if v.is_nan() {
        X87::from_f64_exact(f64::from_bits(v.to_bits() | (1 << 51)))
    } else {
        X87::from_f64_exact(v)
    }
}
