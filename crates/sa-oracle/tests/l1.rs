#![cfg(not(windows))]
//! L1 harness self-tests. Tests that need gta_sa.exe are skipped (with a note) when it is missing; tests
//! that need the native backend (mingw + Wine + the project prefix) are skipped when it is unavailable.

use sa_oracle::call::*;
use sa_oracle::emu::{Allow, Emu};
use sa_oracle::runner::{self, Backend, Cfg};
use sa_oracle::specs::{self, Route};
use std::sync::Arc;

fn img() -> Option<Arc<sa_oracle::pe::Image>> {
    let i = sa_oracle::image_cached();
    if i.is_none() {
        eprintln!("note: gta_sa.exe 1.0 US not found, test skipped");
    }
    i
}

/// A probe emulator with `code` at PROBE.
fn probe_emu(img: &Arc<sa_oracle::pe::Image>, code: &[u8]) -> (Emu, Allow) {
    let mut e = Emu::new(img.clone()).unwrap();
    e.load_probes(code).unwrap();
    (e, Allow::range(PROBE, PROBE + code.len() as u32))
}

#[test]
fn constants_match_the_exe() {
    let Some(img) = img() else { return };
    for (name, addr, bits) in sa_math::consts::EXE_TABLE {
        assert_eq!(img.u32(*addr), Some(*bits), "{name} at {addr:#x}");
    }
}

#[test]
fn exit_stub_matches_source() {
    let dir = sa_oracle::workspace_root().join("target/sa-oracle-test");
    std::fs::create_dir_all(&dir).unwrap();
    let src = sa_oracle::workspace_root().join("crates/sa-oracle/native/exit_stub.S");
    let (obj, bin) = (dir.join("exit_stub.o"), dir.join("exit_stub.bin"));
    let ok = std::process::Command::new("as")
        .args(["--32", "-o"])
        .arg(&obj)
        .arg(&src)
        .status()
        .is_ok_and(|s| s.success())
        && std::process::Command::new("objcopy")
            .args(["-O", "binary", "-j", ".text"])
            .arg(&obj)
            .arg(&bin)
            .status()
            .is_ok_and(|s| s.success());
    if !ok {
        eprintln!("note: binutils not available, test skipped");
        return;
    }
    assert_eq!(std::fs::read(&bin).unwrap(), sa_oracle::emu::EXIT_STUB.to_vec());
}

#[test]
fn calling_conventions() {
    let Some(img) = img() else { return };
    let mut code = vec![0xCCu8; 0x100];
    // 0x00 cdecl: eax = a + b
    code[0x00..0x09].copy_from_slice(&[0x8B, 0x44, 0x24, 0x04, 0x03, 0x44, 0x24, 0x08, 0xC3]);
    // 0x20 stdcall: same, ret 8
    code[0x20..0x2B].copy_from_slice(&[0x8B, 0x44, 0x24, 0x04, 0x03, 0x44, 0x24, 0x08, 0xC2, 0x08, 0x00]);
    // 0x40 thiscall: eax = [ecx] + a, ret 4
    code[0x40..0x49].copy_from_slice(&[0x8B, 0x01, 0x03, 0x44, 0x24, 0x04, 0xC2, 0x04, 0x00]);
    // 0x60 fastcall: eax = ecx + edx + a, ret 4
    code[0x60..0x6A].copy_from_slice(&[0x8D, 0x04, 0x11, 0x03, 0x44, 0x24, 0x04, 0xC2, 0x04, 0x00]);
    // 0x80 cdecl double arg: fld qword [esp+4]
    code[0x80..0x85].copy_from_slice(&[0xDD, 0x44, 0x24, 0x04, 0xC3]);
    // 0xA0 cdecl: ecx = 0x12345678, edx = ecx-on-entry + 1
    code[0xA0..0xAF].copy_from_slice(&[
        0x8D, 0x51, 0x01, 0xB9, 0x78, 0x56, 0x34, 0x12, 0xC3, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC,
    ]);
    let (mut e, allow) = probe_emu(&img, &code);
    let run = |e: &mut Emu, c: &Call| {
        let o = e.call(c, &allow).unwrap();
        assert_eq!(o.esp_delta, c.expected_esp_delta(), "{:?}", c.conv);
        o
    };
    let c = Call::new(PROBE, Conv::Cdecl, &[Arg::I32(5), Arg::I32(7)]);
    assert_eq!(run(&mut e, &c).eax, 12);
    let c = Call::new(PROBE + 0x20, Conv::Stdcall, &[Arg::I32(5), Arg::I32(7)]);
    assert_eq!(run(&mut e, &c).eax, 12);
    let mut c = Call::new(PROBE + 0x40, Conv::Thiscall, &[Arg::Ptr(SCRATCH), Arg::I32(5)]);
    c.scratch[..4].copy_from_slice(&100u32.to_le_bytes());
    assert_eq!((c.ecx, c.stack.len()), (SCRATCH, 1));
    assert_eq!(run(&mut e, &c).eax, 105);
    let c = Call::new(
        PROBE + 0x60,
        Conv::Fastcall,
        &[Arg::I32(1), Arg::I32(2), Arg::I32(3)],
    );
    assert_eq!((c.ecx, c.edx, c.stack.clone()), (1, 2, vec![3]));
    assert_eq!(run(&mut e, &c).eax, 6);
    let c = Call::new(PROBE + 0x80, Conv::Cdecl, &[Arg::F64(1.5)]);
    let o = run(&mut e, &c);
    assert_eq!(o.st0_f32, Some(1.5f32.to_bits()));
    assert_eq!(o.depth(), 1);
    // ECX and EDX after return are captured (preserved-register comparison, VER-2)
    let mut c = Call::new(PROBE + 0xA0, Conv::Cdecl, &[]);
    c.ecx = 0x41;
    let o = run(&mut e, &c);
    assert_eq!((o.ecx, o.edx), (0x1234_5678, 0x42));
    // A wrong convention is visible: the stdcall probe declared as cdecl.
    let c = Call::new(PROBE + 0x20, Conv::Cdecl, &[Arg::I32(5), Arg::I32(7)]);
    let o = e.call(&c, &allow).unwrap();
    assert_ne!(o.esp_delta, c.expected_esp_delta());
}

#[test]
fn fpu_state_is_a_parameter() {
    let Some(img) = img() else { return };
    // fnstcw [esp+4]; movzx eax, word [esp+4]; ret
    let code = [0xD9, 0x7C, 0x24, 0x04, 0x0F, 0xB7, 0x44, 0x24, 0x04, 0xC3];
    let (mut e, allow) = probe_emu(&img, &code);
    for (cw, mx) in [(0x007F, 0x9FA0), (0x027F, 0x1F80), (0x037F, 0x9FA0)] {
        let mut c = Call::new(PROBE, Conv::Cdecl, &[Arg::I32(0)]);
        c.fpu = Fpu { cw, mxcsr: mx };
        let o = e.call(&c, &allow).unwrap();
        assert_eq!(o.eax, cw as u32);
        assert_eq!(o.mxcsr, mx);
    }
    assert_eq!(
        Fpu::default(),
        Fpu {
            cw: 0x007F,
            mxcsr: 0x9FA0
        }
    );
}

#[test]
fn st0_register_read_matches_the_exit_stub() {
    let Some(img) = img() else { return };
    let cat = sa_oracle::catalog_cached().expect("catalogue");
    let spec = specs::get_atan_of_xy();
    let allow = Allow::function(cat, spec.addr, spec.callees).unwrap();
    let mut e = Emu::new(img).unwrap();
    let mut r = sa_oracle::generate::Rng::new(7);
    for _ in 0..2000 {
        let i = (spec.generate)(&mut r);
        let o = e.call(&(spec.call)(&i), &allow).unwrap();
        assert_eq!(e.last_st0_reg(), o.st0, "inputs {i:x?}");
    }
}

#[test]
fn guard_disallowed_callee_is_named() {
    let Some(img) = img() else { return };
    let cat = sa_oracle::catalog_cached().expect("catalogue");
    let mut e = Emu::new(img).unwrap();
    // CGeneral::GetNodeHeadingFromVector calls GetRadianAngleBetweenPoints, then the CRT.
    let c = Call::new(0x53CDC0, Conv::Cdecl, &[Arg::F32(1.0), Arg::F32(2.0)]);
    let err = e
        .call(&c, &Allow::function(cat, 0x53CDC0, &[]).unwrap())
        .unwrap_err();
    match err {
        CallError::Disallowed { target, from, what } => {
            assert_eq!(target, 0x53CBE0);
            assert!((0x53CDC0..0x53CE30).contains(&from), "{from:#x}");
            assert!(what.contains("GetRadianAngleBetweenPoints"), "{what}");
        }
        e => panic!("{e}"),
    }
    // With the callee allowed, the next stop is the CRT call at 0x53CE1A.
    let err = e
        .call(&c, &Allow::function(cat, 0x53CDC0, &[0x53CBE0]).unwrap())
        .unwrap_err();
    assert!(
        matches!(err, CallError::Disallowed { target: 0x8219F0, .. }),
        "{err}"
    );
}

#[test]
fn guard_import_trap_is_named() {
    let Some(img) = img() else { return };
    let imp = img.imports[3].clone();
    let mut code = vec![0xFF, 0x15];
    code.extend_from_slice(&imp.iat.to_le_bytes());
    code.push(0xC3);
    let (mut e, allow) = probe_emu(&img, &code);
    let err = e.call(&Call::new(PROBE, Conv::Cdecl, &[]), &allow).unwrap_err();
    match err {
        CallError::Disallowed { target, what, .. } => {
            assert_eq!(target, imp.trap);
            assert!(what.contains(&imp.name), "{what}");
        }
        e => panic!("{e}"),
    }
}

#[test]
fn guard_budget_and_hang() {
    let Some(img) = img() else { return };
    let cat = sa_oracle::catalog_cached().expect("catalogue");
    let mut e = Emu::new(img).unwrap();
    e.budget = 100_000;
    let allow = Allow::function(cat, 0x53CB00, &[]).unwrap();
    let c = Call::new(0x53CB00, Conv::Cdecl, &[Arg::F32(f32::INFINITY)]);
    assert_eq!(
        e.call(&c, &allow).unwrap_err(),
        CallError::Budget { budget: 100_000 }
    );
    // The instance is reusable afterwards.
    let c = Call::new(0x53CB00, Conv::Cdecl, &[Arg::F32(190.0)]);
    assert_eq!(e.call(&c, &allow).unwrap().st0_f32, Some((-170.0f32).to_bits()));
}

#[test]
fn guard_global_write_read_and_restore() {
    let Some(img) = img() else { return };
    let orig = img.bytes(0x8A4000, 4).to_vec();
    let mut code = vec![0xCCu8; 0x40];
    // mov dword [0x8A4000], 0x12345678; ret
    code[..11].copy_from_slice(&[0xC7, 0x05, 0x00, 0x40, 0x8A, 0x00, 0x78, 0x56, 0x34, 0x12, 0xC3]);
    // mov eax, [0x8A4000]; ret
    code[0x20..0x26].copy_from_slice(&[0xA1, 0x00, 0x40, 0x8A, 0x00, 0xC3]);
    let (mut e, allow) = probe_emu(&img, &code);
    let err = e.call(&Call::new(PROBE, Conv::Cdecl, &[]), &allow).unwrap_err();
    assert!(
        matches!(
            err,
            CallError::GlobalWrite {
                addr: 0x8A4000,
                size: 4,
                ..
            }
        ),
        "{err}"
    );
    assert_eq!(e.read_mem(0x8A4000, 4).unwrap(), orig, "image restored");
    let err = e
        .call(&Call::new(PROBE + 0x20, Conv::Cdecl, &[]), &allow)
        .unwrap_err();
    assert!(
        matches!(err, CallError::GlobalRead { addr: 0x8A4000, .. }),
        "{err}"
    );
    // A constant read from .rdata is fine.
    let code2 = [0xA1, 0x50, 0x8B, 0x85, 0x00, 0xC3];
    let (mut e, allow) = probe_emu(&img, &code2);
    assert_eq!(
        e.call(&Call::new(PROBE, Conv::Cdecl, &[]), &allow).unwrap().eax,
        0
    );
}

#[test]
fn routes_match_hazards() {
    let Some(img) = img() else { return };
    let cat = sa_oracle::catalog_cached().expect("catalogue");
    for s in specs::all() {
        let h = sa_oracle::scan::scan(&img, cat, s.addr, s.callees).unwrap();
        // With SSE the MXCSR.FZ workaround is off, so Unicorn's x87 flushes f32 subnormals: such a
        // function must be decided by the CPU.
        assert!(
            h.sse.is_none() || s.route != Route::L1,
            "{}: SSE {:?} (MXCSR.FZ workaround not applicable) but routed to L1",
            s.name,
            h.sse
        );
        match s.route {
            Route::L1 => assert!(h.x87.is_empty(), "{} routed L1 but uses {:?}", s.name, h.x87),
            Route::L2 { .. } => assert!(!h.x87.is_empty(), "{} routed L2 without a hazard", s.name),
        }
    }
}

fn quick_cfg(cases: u64, backend: Backend) -> Cfg {
    Cfg {
        cases,
        backend,
        // the test harness runs these tests in parallel; each Unicorn instance is ~25 MB
        threads: 4,
        fixtures: false,
        keep: 2,
        snan_native: sa_oracle::native::available(),
        ..Cfg::default()
    }
}

#[test]
fn l1_specs_pass_under_unicorn() {
    let Some(img) = img() else { return };
    for s in specs::all().into_iter().filter(|s| s.route == Route::L1) {
        let rep = runner::run(&s, &img, &quick_cfg(20_000, Backend::Unicorn)).unwrap();
        assert_eq!(rep.mismatches, 0, "{}", runner::print_report(&s, &rep));
    }
}

#[test]
fn l2_specs_pass_on_the_cpu() {
    let Some(img) = img() else { return };
    if !sa_oracle::native::available() {
        eprintln!("note: native backend unavailable, test skipped");
        return;
    }
    for s in specs::all().into_iter().filter(|s| s.route != Route::L1) {
        let rep = runner::run(&s, &img, &quick_cfg(5_000, Backend::Native)).unwrap();
        assert_eq!(rep.mismatches, 0, "{}", runner::print_report(&s, &rep));
    }
}

#[test]
fn x87_model_matches_the_cpu_on_special_values() {
    let Some(img) = img() else { return };
    if !sa_oracle::native::available() {
        eprintln!("note: native backend unavailable, test skipped");
        return;
    }
    let m = sa_oracle::fidelity::nan_matrix(&img).unwrap();
    assert!(m.model_mismatches.is_empty(), "{:#?}", m.model_mismatches);
    // Unicorn is known to differ here (signalling NaNs); keep that measured.
    assert!(m.unicorn_mismatches > 0);
}

#[test]
fn mutation_smoke_test() {
    let Some(img) = img() else { return };
    for mt in sa_math::general::mutants::ALL {
        let s = specs::solve_quadratic_with(move |a, b, c| {
            sa_math::general::mutants::solve_quadratic(mt, a, b, c)
        });
        let rep = runner::run(&s, &img, &quick_cfg(20_000, Backend::Unicorn)).unwrap();
        assert!(rep.mismatches > 0, "mutant {mt:?} not caught");
    }
    for mt in sa_math::general::mutants::EQUIVALENT {
        let s = specs::solve_quadratic_with(move |a, b, c| {
            sa_math::general::mutants::solve_quadratic(mt, a, b, c)
        });
        let rep = runner::run(&s, &img, &quick_cfg(20_000, Backend::Unicorn)).unwrap();
        assert_eq!(
            rep.mismatches,
            0,
            "equivalent mutant {mt:?}: {}",
            runner::print_report(&s, &rep)
        );
    }
}

/// The FSIN/FCOS model of `sa_math::x87` (66-bit-pi reduction) against the CPU: every table entry of
/// `CMaths::InitMathsTables` (the argument is the PC=24 product `i * 2pi/256`, an f32) and random
/// arguments over the whole f32 range. The stored f32 is compared (the model has 53 of the CPU's 64
/// result bits).
#[test]
fn fsin_fcos_model_matches_the_cpu() {
    let Some(img) = img() else { return };
    if !sa_oracle::native::available() {
        eprintln!("note: native backend unavailable, test skipped");
        return;
    }
    use sa_math::x87::X87;
    // 0x00: fld dword [esp+4]; fsin; ret   0x10: fld dword [esp+4]; fcos; ret
    let mut code = vec![0xCCu8; 0x20];
    code[..7].copy_from_slice(&[0xD9, 0x44, 0x24, 0x04, 0xD9, 0xFE, 0xC3]);
    code[0x10..0x17].copy_from_slice(&[0xD9, 0x44, 0x24, 0x04, 0xD9, 0xFF, 0xC3]);
    let mut xs: Vec<f32> = (0..256)
        .map(|i| {
            X87::from_i32(i)
                .fmul_m(f32::from_bits(sa_math::consts::SIN_TABLE_STEP))
                .to_f32()
        })
        .collect();
    let mut r = sa_oracle::generate::Rng::new(0xF51);
    for _ in 0..100_000 {
        xs.push(match r.below(3) {
            0 => f32::from_bits(r.next_u32()),
            1 => r.uniform(10.0),
            _ => r.uniform(1e6),
        });
    }
    let calls: Vec<Call> = xs
        .iter()
        .flat_map(|&x| [PROBE, PROBE + 0x10].map(|a| Call::new(a, Conv::Cdecl, &[Arg::F32(x)])))
        .collect();
    let out = sa_oracle::native::run(&img, &calls, Some(&code)).unwrap();
    let mut bad = Vec::new();
    for (k, o) in out.iter().enumerate() {
        let x = X87::from_f32(xs[k / 2]);
        let m = if k % 2 == 0 { x.fsin() } else { x.fcos() };
        if o.st0_f32 != Some(m.to_f32().to_bits()) {
            bad.push((xs[k / 2], k % 2, o.st0_f32, m.to_f32().to_bits()));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} differ, e.g. {:x?}",
        bad.len(),
        out.len(),
        &bad[..bad.len().min(5)]
    );
    let table = sa_math::maths::init_maths_tables();
    for (i, v) in table.iter().enumerate() {
        assert_eq!(Some(v.to_bits()), out[2 * i].st0_f32, "table[{i}]");
    }
}

/// Hooks registered with `preserve` whose register-comparing L1 spec is still to be written. The 15
/// hooks listed here when this test started covering every port crate (2026-11-11, sa-ports) all have
/// specs now (specs_cam, specs_input_ccm, specs_veh, specs_ai). The list may only shrink; a new hook
/// that preserves registers without a spec fails the test.
const PRESERVE_SPECS_PENDING: &[u32] = &[
    // AI-GEN generated dtor CEventFireNearby::~CEventFireNearby (preserve ecx); spec queued.
    0x4B1F80,
];

/// Preserved registers (VER-2, verifier finding V-9): every spec that declares them names a port
/// whose `#[hook(preserve = ...)]` covers them, and every hook that preserves registers has a spec
/// that compares them.
#[test]
fn preserved_registers_match_the_hook_registrations() {
    let all = specs::all();
    for s in all.iter().filter(|s| s.preserve != 0) {
        let p = sa_oracle::hook_preserve(s.addr).unwrap_or_else(|| panic!("{}: no hook", s.key));
        assert_eq!(p & s.preserve, s.preserve, "{}: hook preserves {p:#x}", s.key);
    }
    let missing: Vec<String> = sa_ports::ALL
        .iter()
        .filter(|r| r.preserve != 0)
        .filter(|r| !PRESERVE_SPECS_PENDING.contains(&r.addr))
        .filter(|r| !all.iter().any(|s| s.addr == r.addr && s.preserve == r.preserve))
        .map(|r| format!("{} ({:#x}) preserves {:#x}", r.name, r.addr, r.preserve))
        .collect();
    assert!(
        missing.is_empty(),
        "{} hooks preserve registers without a spec comparing them:\n{}",
        missing.len(),
        missing.join("\n")
    );
}
