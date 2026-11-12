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

/// Entry registers of register-convention functions (HARNESS-5): EAX (instead of the call target),
/// EBX, ESI, EDI and EBP reach the function on both backends, laid out by `Call::regconv` from a
/// `sa_core::regconv` spec.
#[test]
fn register_entry_conventions() {
    let Some(img) = img() else { return };
    // add eax, ebx; add eax, esi; add eax, edi; add eax, ebp; add eax, [esp+4]; ret 4
    let code = [
        0x01, 0xD8, 0x01, 0xF0, 0x01, 0xF8, 0x01, 0xE8, 0x03, 0x44, 0x24, 0x04, 0xC2, 0x04, 0x00,
    ];
    let conv = sa_core::regconv::RegConv::parse("args = al ebx si edi ebp stack; pop");
    let c = Call::regconv(PROBE, &conv, &[0x01, 0x10, 0x100, 0x1000, 0x10000, 0x100000], 0xFF00_0000);
    assert_eq!((c.eax, c.ebx, c.esi, c.edi, c.ebp), (Some(0xFF00_0001), 0x10, 0xFF00_0100, 0x1000, 0x10000));
    assert_eq!((c.conv, c.stack.clone()), (Conv::Stdcall, vec![0x100000]));
    let want = [0xFF00_0001u32, 0x10, 0xFF00_0100, 0x1000, 0x10000, 0x100000]
        .iter()
        .fold(0u32, |a, &b| a.wrapping_add(b));
    let without_eax = want.wrapping_sub(0xFF00_0001).wrapping_add(PROBE);
    let (mut e, allow) = probe_emu(&img, &code);
    let o = e.call(&c, &allow).unwrap();
    assert_eq!((o.eax, o.esp_delta), (want, 0));
    // without an entry EAX, EAX is the call target (as before)
    let mut c2 = c.clone();
    c2.eax = None;
    assert_eq!(e.call(&c2, &allow).unwrap().eax, without_eax);
    if sa_oracle::native::available() {
        let mut p = vec![0xCCu8; 0x100];
        p[..code.len()].copy_from_slice(&code);
        let r = sa_oracle::native::run(&img, &[c, c2], Some(&p)).unwrap();
        assert_eq!((r[0].eax, r[0].esp_delta), (want, 0), "native");
        assert_eq!(r[1].eax, without_eax, "native");
    } else {
        eprintln!("note: native backend unavailable, native half skipped");
    }
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
            // SSE in the scanned extent is a hazard of its own (the CPU must decide, see above)
            Route::L2 { .. } => assert!(
                !h.x87.is_empty() || h.sse.is_some(),
                "{} routed L2 without a hazard",
                s.name
            ),
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
    let mut known = Vec::new();
    for s in specs::all().into_iter().filter(|s| s.route != Route::L1) {
        let rep = runner::run(&s, &img, &quick_cfg(5_000, Backend::Native)).unwrap();
        if KNOWN_CPU_MISMATCHES.contains(&s.addr) {
            known.push((s.addr, rep.mismatches));
            continue;
        }
        assert_eq!(rep.mismatches, 0, "{}", runner::print_report(&s, &rep));
    }
    for (addr, n) in known {
        assert!(n > 0, "{addr:#x} now matches the CPU: remove it from KNOWN_CPU_MISMATCHES");
    }
}

/// Specs whose ports are known to differ from the CPU (the functions stay `implemented`). This list
/// may only shrink. Empty since X87-2 (RwMatrixRotate matches with the 64-bit FSIN/FCOS models).
const KNOWN_CPU_MISMATCHES: &[u32] = &[];

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
    // RW-5 RwD3D9SetRenderState (preserve edx), inherited unlisted by the MVP-F base; spec queued.
    0x7FC2D0,
    // STD-3: CSprite2d::Draw(CRect, CRGBA) (preserve ecx); PORT-AUDIO-3 (merged by STD-4):
    // CAEPedSpeechAudioEntity::Constructor (preserve edx); specs queued.
    0x4E4F10, // (0x728350 is listed with WRITER-MVP-A below)
    // AI-GEN generated dtor CEventFireNearby::~CEventFireNearby (preserve ecx); spec queued.
    0x4B1F80,
    // PORT-VEH-2: GetVehicleAppearance, HeightAboveCeiling, SelectPlaneWeapon, GetPlaneNumGuns; specs queued.
    0x6D1080, 0x6D2600, 0x6D4900, 0x6D3F30,
    // PORT-WORLD-2: CBoundingBox::IsPointWithin, CGarages::FindSafeHouseIndexForGarageType,
    // CStats::FindMaxNumberOfGroupMembers; specs queued.
    0x49C300, 0x4489F0, 0x559A50,
    // PORT-POP-2: CountNeighboursToBeSwitchedOff, FindIntersection2Lines, CheckDamagedWeaponType; specs queued.
    0x4504F0, 0x4226F0, 0x43D9E0,
    // CAM-2 0x50E180, AI-5 CDecision::SetDefault, AI-4 FollowNodeRoute::GetSubTaskType; specs queued.
    0x50E180, 0x600530, 0x669650,
    // SCRIPT-3: CMissionCleanup::AddEntityToList; spec queued.
    0x4637E0,
    // PORT-RW-4: RwD3D9SetRenderState preserve edx; spec queued.
    // WRITER-MVP-A (unverified batches mvpa-ai-1, mvpa-ai-2, mvpa-render-1, mvpa-render-2): CTaskSimple ctor
    // 0x61A390, event dtor 0x4B0D00, CSprite2d::Draw 0x728350, FxManager_c::DestroyFxSystem 0x4A9810
    // (preserve ecx); specs queued for the verifier.
    0x61A390, 0x4B0D00, 0x728350, 0x4A9810,
    // WRITER-MVP-C batch mvpc-render-1: FUN_006f9eb0 0x6F9EB0 (preserve ecx, caller 0x6FA740); spec queued.
    0x6F9EB0,
    // WRITER-MVP-D: CAttractorScanner::Clear (ecx), CConversationNode::ClearRecursively (edx); specs queued.
    0x5FFF90, 0x43A710,
    // STD-6: CRunningScript::AddScriptToList (preserve ecx); spec queued.
    0x464C00,
    // STD-12 (std12-audio-1): CAEVehicleAudioEntity::GetVehicleDriveWheelSkidValue /
    // GetVehicleNonDriveWheelSkidValue (preserve edx); specs queued.
    0x4F5F30, 0x4F6000,
    // PORT-AI-8 (generated batch ai8g-2): CTaskComplexDestroyCarMelee::CalculateSearchPositionAndRanges
    // (preserve ecx, caller 0x62DD41 → 0x628A70); spec queued.
    0x6289F0,
    // WRITER-SAMP batch samp-unk-1: the vtable-only event destructors 0x4ADE90 / 0x4B13C0 (preserve ecx: their
    // callers read ECX after the call); specs queued.
    0x4ADE90, 0x4B13C0,
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

/// Probe code for the FYL2X / FPTAN measurement (at PROBE):
/// - 0x00: `fldlg2; fld dword [esp+4]; fyl2x; ret` (the AudioLog10 sequence);
/// - 0x10: `fld dword [esp+4]; fld dword [esp+8]; fyl2x; ret` (`y * log2(x)`, y first);
/// - 0x20: `fld dword [esp+4]; fptan; fstp st(0); ret` (tan; the stack is empty when nothing was pushed);
/// - 0x30: `fld dword [esp+4]; fptan; ret` (ST0 = the pushed 1.0);
/// - 0x40: `push [esp+4]; call AudioLog10 (0x4D9E50); add esp, 4; fmul dword [esp+8]; ret` (the
///   callers' composite: every caller multiplies ST0 by an f32 constant, 20.0 or 10.0).
fn transcendental_probes() -> Vec<u8> {
    let mut code = vec![0xCCu8; 0x60];
    code[..9].copy_from_slice(&[0xD9, 0xEC, 0xD9, 0x44, 0x24, 0x04, 0xD9, 0xF1, 0xC3]);
    code[0x10..0x1B].copy_from_slice(&[0xD9, 0x44, 0x24, 0x04, 0xD9, 0x44, 0x24, 0x08, 0xD9, 0xF1, 0xC3]);
    code[0x20..0x29].copy_from_slice(&[0xD9, 0x44, 0x24, 0x04, 0xD9, 0xF2, 0xDD, 0xD8, 0xC3]);
    code[0x30..0x37].copy_from_slice(&[0xD9, 0x44, 0x24, 0x04, 0xD9, 0xF2, 0xC3]);
    let call_at = PROBE + 0x44;
    let rel = 0x4D9E50u32.wrapping_sub(call_at + 5).to_le_bytes();
    code[0x40..0x44].copy_from_slice(&[0xFF, 0x74, 0x24, 0x04]);
    code[0x44] = 0xE8;
    code[0x45..0x49].copy_from_slice(&rel);
    code[0x49..0x51].copy_from_slice(&[0x83, 0xC4, 0x04, 0xD8, 0x4C, 0x24, 0x08, 0xC3]);
    code
}

/// Special and boundary f32 operands.
fn transcendental_edges() -> Vec<f32> {
    let f = f32::from_bits;
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        10.0,
        100.0,
        0.1,
        1e-5,
        f(0x3727_C5AC),
        f(0x3727_C5AB),
        2.0,
        0.5,
        1e38,
        f32::MAX,
        f32::MIN_POSITIVE,
        f(1),
        f(0x007F_FFFF),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f(0x7FC0_0000),
        f(0xFFC0_0000),
        f(0x7F80_0001),
        f(0x7FA0_1234),
        std::f32::consts::FRAC_PI_4,
        -std::f32::consts::FRAC_PI_4,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        9.223_371_5e18,
        9.223_372e18,
        -9.223_372e18,
        1e19,
    ];
    for k in -30..=38 {
        v.push(10f64.powi(k) as f32);
        v.push(2f64.powi(k * 3) as f32);
    }
    for i in 0..256 {
        // the CMaths table steps and small multiples of pi/4
        v.push(i as f32 * std::f32::consts::FRAC_PI_4);
        v.push(1.0 + i as f32 * f32::EPSILON);
        v.push(1.0 - i as f32 * f32::EPSILON / 2.0);
    }
    v
}

/// Result of one measured instruction.
#[derive(Default, Debug)]
struct Tally {
    n: usize,
    ext_exact: usize,
    /// |model - CPU| in units of the last 64-bit place, for the inexact ones (> 1 is a model defect)
    max_ulps: u64,
    f32_exact: usize,
    /// NaN/inf/zero/empty-stack outcomes that differ (always a defect)
    special_bad: Vec<String>,
}

fn ulps64(a: [u8; 10], b: [u8; 10]) -> u64 {
    let m = |x: [u8; 10]| u64::from_le_bytes(x[..8].try_into().unwrap());
    let se = |x: [u8; 10]| u16::from_le_bytes([x[8], x[9]]);
    if se(a) != se(b) {
        // adjacent binades: compare on a common scale
        let (ea, eb) = (se(a) & 0x7FFF, se(b) & 0x7FFF);
        if se(a) >> 15 != se(b) >> 15 || ea.abs_diff(eb) != 1 {
            return u64::MAX;
        }
        let (hi, lo) = if ea > eb { (a, b) } else { (b, a) };
        return ((m(hi) as u128) << 1)
            .abs_diff(m(lo) as u128)
            .min(u64::MAX as u128) as u64;
    }
    m(a).abs_diff(m(b))
}

fn tally(t: &mut Tally, what: &str, x: f32, cpu: Option<[u8; 10]>, model: Option<sa_math::x87::X87>) {
    t.n += 1;
    let model_ext = model.map(|m| m.to_ext());
    if cpu == model_ext {
        t.ext_exact += 1;
        t.f32_exact += 1;
        return;
    }
    match (cpu, model) {
        (Some(c), Some(m)) if m.to_f64().is_finite() && m.to_f64() != 0.0 => {
            let u = ulps64(c, m.to_ext());
            if u > 1 && t.special_bad.len() < 3 {
                eprintln!(
                    "{what}({x:e} / {:#x}): cpu {c:x?} model {:x?}",
                    x.to_bits(),
                    m.to_ext()
                );
            }
            t.max_ulps = t.max_ulps.max(u);
            if sa_math::x87::X87::from_ext(c).to_f32().to_bits() == m.to_f32().to_bits() {
                t.f32_exact += 1;
            }
        }
        _ => t.special_bad.push(format!(
            "{what}({x:e} / {:#x}): cpu {cpu:x?} model {model_ext:x?}",
            x.to_bits()
        )),
    }
}

fn measure_transcendentals(n: usize) {
    let Some(img) = img() else { return };
    if !sa_oracle::native::available() {
        eprintln!("note: native backend unavailable, test skipped");
        return;
    }
    use sa_math::x87::X87;
    let code = transcendental_probes();
    let mut r = sa_oracle::generate::Rng::new(0x7A4);
    let edges = transcendental_edges();
    // AudioLog10 inputs: volumes and ratios (>= 1e-5), plus the whole positive f32 range
    let mut logx: Vec<f32> = edges.clone();
    while logx.len() < n {
        logx.push(match r.below(4) {
            0 => f32::from_bits(r.below(0x7F80_0000 - 0x3727_C5AC) + 0x3727_C5AC),
            1 => r.uniform(2.0).abs(),
            2 => r.uniform(100.0).abs(),
            _ => f32::from_bits(r.next_u32()),
        });
    }
    let mut yx: Vec<(f32, f32)> = edges
        .iter()
        .flat_map(|&a| [(1.0, a), (a, 2.0), (-a, 0.5), (a, 0.0)])
        .collect();
    while yx.len() < n {
        yx.push((r.uniform(4.0), f32::from_bits(r.next_u32() & 0x7FFF_FFFF)));
    }
    let mut tanx: Vec<f32> = edges.clone();
    while tanx.len() < n {
        tanx.push(match r.below(4) {
            0 => r.uniform(1.6),
            1 => r.uniform(7.0),
            2 => r.uniform(1e4),
            _ => f32::from_bits(r.next_u32()),
        });
    }
    let (mut tl, mut ty, mut tt, mut tp, mut tc) = (
        Tally::default(),
        Tally::default(),
        Tally::default(),
        Tally::default(),
        Tally::default(),
    );
    let consts = [f32::from_bits(0x41A0_0000), 10.0]; // [0x858BA4] = 20.0, [0x85862C] = 10.0
    for chunk in 0..n.div_ceil(100_000) {
        let lo = chunk * 100_000;
        let hi = (lo + 100_000).min(n);
        let mut calls = Vec::new();
        for i in lo..hi {
            let c = consts[i % 2];
            calls.push(Call::new(PROBE, Conv::Cdecl, &[Arg::F32(logx[i])]));
            calls.push(Call::new(
                PROBE + 0x10,
                Conv::Cdecl,
                &[Arg::F32(yx[i].0), Arg::F32(yx[i].1)],
            ));
            calls.push(Call::new(PROBE + 0x20, Conv::Cdecl, &[Arg::F32(tanx[i])]));
            calls.push(Call::new(PROBE + 0x30, Conv::Cdecl, &[Arg::F32(tanx[i])]));
            calls.push(Call::new(
                PROBE + 0x40,
                Conv::Cdecl,
                &[Arg::F32(logx[i]), Arg::F32(c)],
            ));
        }
        let out = sa_oracle::native::run(&img, &calls, Some(&code)).unwrap();
        for (j, i) in (lo..hi).enumerate() {
            let o = &out[5 * j..5 * j + 5];
            let x = logx[i];
            let l = X87::lg2().fyl2x(X87::from_f32(x));
            tally(&mut tl, "log10", x, o[0].st0, Some(l));
            let (y, xx) = yx[i];
            tally(
                &mut ty,
                "fyl2x",
                xx,
                o[1].st0,
                Some(X87::from_f32(y).fyl2x(X87::from_f32(xx))),
            );
            let t = X87::from_f32(tanx[i]).fptan();
            tally(&mut tt, "fptan", tanx[i], o[2].st0, t.map(|t| t.0));
            tally(
                &mut tp,
                "fptan push",
                tanx[i],
                o[3].st0,
                Some(t.map_or(X87::from_f32(tanx[i]), |t| t.1)),
            );
            // AudioLog10 composite: below 1e-5 (or NaN) the constant -5.0, else the fyl2x result
            let a = if X87::from_f32(x).fcom(X87::from_f32(1e-5)).c0() {
                X87::from_f32(-5.0)
            } else {
                l
            };
            tally(
                &mut tc,
                "AudioLog10*c",
                x,
                o[4].st0,
                Some(a.fmul_m(consts[i % 2])),
            );
        }
    }
    for (name, t) in [
        ("fldlg2; fyl2x (AudioLog10 core)", &tl),
        ("fyl2x y*log2(x)", &ty),
        ("fptan", &tt),
        ("fptan pushed 1.0", &tp),
        ("AudioLog10(x) * f32 (callers, PC=24)", &tc),
    ] {
        eprintln!(
            "{name}: {} cases, 80-bit exact {} ({:.4}%), stored-f32 exact {} ({:.4}%), max {} ulp(64), specials differing {}",
            t.n,
            t.ext_exact,
            100.0 * t.ext_exact as f64 / t.n as f64,
            t.f32_exact,
            100.0 * t.f32_exact as f64 / t.n as f64,
            t.max_ulps,
            t.special_bad.len()
        );
        assert!(
            t.special_bad.is_empty(),
            "{name}: {:#?}",
            &t.special_bad[..t.special_bad.len().min(10)]
        );
        assert!(t.max_ulps <= 1, "{name}: {} ulps", t.max_ulps);
    }
    // precision control does not apply to FYL2X/FPTAN: the same results under PC=53 and PC=64
    let mut calls = Vec::new();
    for i in 0..20_000 {
        for cw in [0x027Fu16, 0x037F] {
            let mut a = Call::new(PROBE, Conv::Cdecl, &[Arg::F32(logx[i])]);
            a.fpu.cw = cw;
            let mut b = Call::new(PROBE + 0x20, Conv::Cdecl, &[Arg::F32(tanx[i])]);
            b.fpu.cw = cw;
            calls.extend([a, b]);
        }
    }
    let out = sa_oracle::native::run(&img, &calls, Some(&code)).unwrap();
    let (mut pl, mut pt) = (0, 0);
    for i in 0..20_000 {
        let l = X87::lg2().fyl2x(X87::from_f32(logx[i])).to_ext();
        let t = X87::from_f32(tanx[i]).fptan().map(|t| t.0.to_ext());
        for k in 0..2 {
            pl += (out[4 * i + 2 * k].st0 == Some(l)) as usize;
            pt += (out[4 * i + 2 * k + 1].st0 == t) as usize;
        }
    }
    eprintln!("PC=53/64: fyl2x 80-bit exact {pl} of 40000, fptan {pt} of 40000");
    assert!(pl >= 39_000 && pt >= 37_000);
    // what the game observes is exact
    assert_eq!(tc.ext_exact, tc.n, "AudioLog10 composite");
    assert_eq!(tp.ext_exact, tp.n);
    assert!(tl.ext_exact as f64 >= 0.98 * tl.n as f64);
    assert!(ty.ext_exact as f64 >= 0.98 * ty.n as f64);
    assert!(tt.ext_exact as f64 >= 0.93 * tt.n as f64);
}

/// The FYL2X and FPTAN models of `sa_math::x87` against the CPU (100k cases each; the 10^6 run is
/// `fyl2x_fptan_model_vs_cpu_1m`).
#[test]
fn fyl2x_fptan_model_vs_cpu() {
    measure_transcendentals(100_000);
}

#[test]
#[ignore = "10^6 cases per instruction on the CPU (minutes); run explicitly"]
fn fyl2x_fptan_model_vs_cpu_1m() {
    measure_transcendentals(1_000_000);
}
